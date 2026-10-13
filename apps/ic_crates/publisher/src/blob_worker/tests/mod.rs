use super::*;

#[test]
fn selected_tools_refuse_changed_bytes_before_dispatch() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("tool");
    std::fs::write(&path, b"original").unwrap();
    let selected = config::SelectedFile {
        sha256: ic_host_fs::read::hash_file(&path, 1024)
            .unwrap()
            .sha256
            .to_string(),
        path: path.clone(),
    };
    selected.verify().unwrap();
    std::fs::write(&path, b"different").unwrap();
    assert!(selected.verify().is_err());
}

#[test]
#[ignore = "requires selected released Blob/SDK tools; use make test-blob-worker"]
fn real_sdk_preflight_matches_the_original_cargo_job_and_refuses_mutation() {
    let selected = PathBuf::from(
        std::env::var_os("IC_CRATES_BLOB_TEST_CONFIGURATION").expect("select test tools"),
    );
    let mut config = Configuration::read(&selected).unwrap();
    let evidence =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/blob-worker-tests");
    std::fs::create_dir_all(&evidence).unwrap();
    let temporary = tempfile::Builder::new()
        .prefix("preflight.")
        .tempdir_in(evidence)
        .unwrap()
        .keep();
    let temporary = temporary.as_path();
    eprintln!("Worker test evidence: {}", temporary.display());
    // Generate ephemeral fixture signers through Node's crypto and the selected SDK.
    let key_script = r#"
      const {generateKeyPairSync} = require('node:crypto');
      const {writeFileSync} = require('node:fs');
      const {pathToFileURL} = require('node:url');
      (async()=>{
        const sdk=await import(pathToFileURL(process.argv[1]).href);
        const directory=process.argv[2];
        for(const name of ['uploader','verifier']) {
          const {privateKey}=generateKeyPairSync('ed25519');
          writeFileSync(directory+'/'+name+'.pem',privateKey.export({format:'pem',type:'pkcs8'}),{flag:'wx',mode:0o600});
          const identity=sdk.Ed25519KeyIdentity.fromSecretKey(Buffer.from(privateKey.export({format:'jwk'}).d,'base64url'));
          writeFileSync(directory+'/'+name+'.json',JSON.stringify(identity.toJSON()),{flag:'wx',mode:0o600});
        }
      })().catch(()=>process.exit(1));"#;
    assert!(
        Command::new(&config.node.path)
            .args(["-e", key_script])
            .arg(&config.sdk_module.path)
            .arg(temporary)
            .status()
            .unwrap()
            .success()
    );
    config.uploader_identity = temporary.join("uploader.pem");
    config.verifier_identity = temporary.join("verifier.pem");
    config.browser_identity = temporary.join("uploader.json");
    use ic_agent::Identity as _;
    let principal = |path: &Path| {
        ic_agent::identity::BasicIdentity::from_pem(std::fs::read(path).unwrap().as_slice())
            .unwrap()
            .sender()
            .unwrap()
    };
    let mut bridge = crate::tests::config(temporary);
    bridge.worker = std::env::current_exe().unwrap();
    bridge.uploader = principal(&config.uploader_identity);
    bridge.worker_timeout_seconds = 600;
    bridge.permission_seconds = 900;
    config.verifier = principal(&config.verifier_identity);
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    bridge.replica_url = format!("http://{}", server.server_addr());
    let key = ic_agent::Agent::builder()
        .with_url("https://icp-api.io")
        .build()
        .unwrap()
        .read_root_key();
    let root_key = temporary.join("root-key.der");
    save(&root_key, &key).unwrap();
    bridge.root_key_file = Some(root_key);
    let stopping = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stop = stopping.clone();
    let observed = requests.clone();
    let serving = std::thread::spawn(move || {
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            if let Some(request) = server.recv_timeout(Duration::from_millis(100)).unwrap() {
                observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                request
                    .respond(tiny_http::Response::empty(503).with_header(
                        tiny_http::Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
                    ))
                    .unwrap();
            }
        }
    });
    // Incompressible data crosses the SDK's 1 MiB leaf boundary.
    let mut input = crate::tests::body("0.1.0", "wire_demo-0.1.0/Cargo.toml", false);
    let metadata_bytes = u32::from_le_bytes(input[..4].try_into().unwrap()) as usize;
    input.truncate(4 + metadata_bytes);
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive_builder = tar::Builder::new(gzip);
    let mut state = 0x1234_5678_u32;
    let data: Vec<u8> = (0..2 * 1024 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect();
    for (path, bytes) in [
        (
            "wire_demo-0.1.0/Cargo.toml",
            b"[package]\nname=\"wire_demo\"\nversion=\"0.1.0\"\n".as_slice(),
        ),
        ("wire_demo-0.1.0/data.bin", data.as_slice()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive_builder
            .append_data(&mut header, path, bytes)
            .unwrap();
    }
    let archive_bytes = archive_builder.into_inner().unwrap().finish().unwrap();
    input.extend_from_slice(&(archive_bytes.len() as u32).to_le_bytes());
    input.extend_from_slice(&archive_bytes);
    let upload = crate::upload::receive(
        &mut std::io::Cursor::new(&input),
        bridge.max_archive_bytes,
        &bridge.spool,
        "sparse+https://fixture.example/index/",
    )
    .unwrap();
    let archive = upload.file.path();
    // Build the same canonical job projection the publisher retains, without
    // dispatching admission, a worker or any IC/provider request.
    let job = bridge.spool.join("wire-demo@0.1.0");
    let permission = UploadAdmissionRequest {
        upload: ic_blob_storage_contracts::dto::reference::ReferenceUpload {
            service: bridge.blob_service,
            tenant: bridge.registry,
            namespace: 1,
            upload: 1,
            object: 2,
            incarnation: 1,
            first_reference: 3,
            root: *upload.manifest.hashes().provider_root.as_bytes(),
            bytes: upload.bytes,
        },
        uploader: bridge.uploader,
        expires_at_ns: u64::MAX,
    };
    let manifest = UploadManifestRequest {
        permission,
        declaration: ic_blob_storage_contracts::dto::upload::manifest::UploadManifestDeclaration {
            chunks: upload
                .manifest
                .manifest()
                .chunks()
                .iter()
                .map(|hash| *hash.as_bytes())
                .collect(),
            headers: vec![
                ic_blob_storage_contracts::dto::upload::manifest::UploadManifestHeader {
                    name: "Content-Length".into(),
                    value: upload.bytes.to_string(),
                },
                ic_blob_storage_contracts::dto::upload::manifest::UploadManifestHeader {
                    name: "Content-Type".into(),
                    value: "application/gzip".into(),
                },
            ],
        },
    };
    let key = ic_agent::Agent::builder()
        .with_url(&bridge.replica_url)
        .build()
        .unwrap()
        .read_root_key();
    let intent = Intent {
        configuration: serde_json::to_string(&bridge).unwrap(),
        actor: "fixture".into(),
        root_digest: checksum_hex(
            ic_blob_storage_contracts::identity::ContentDigest::compute(&key).as_bytes(),
        ),
        worker_digest: ic_host_fs::read::hash_file(&bridge.worker, crate::config::MAX_WORKER_BYTES)
            .unwrap()
            .sha256
            .to_string(),
        index: upload.index.clone(),
        bytes: upload.bytes,
    };
    save(&job.join("archive.crate"), &std::fs::read(archive).unwrap()).unwrap();
    save(
        &job.join("intent.json"),
        &serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    save(
        &job.join("permission.candid"),
        &candid::encode_one(permission).unwrap(),
    )
    .unwrap();
    save(
        &job.join("manifest.candid"),
        &candid::encode_one(manifest).unwrap(),
    )
    .unwrap();
    save(
        &job.join("publish.candid"),
        &candid::encode_one(PublishRequest {
            index_json: upload.index.line().unwrap(),
            permission,
        })
        .unwrap(),
    )
    .unwrap();
    let configuration = temporary.join("worker.json");
    save(&configuration, &serde_json::to_vec(&config).unwrap()).unwrap();
    let output = temporary.join("preflight");
    let prepared = prepare(&configuration, &job, &output).unwrap();
    assert_eq!(requests.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(output.join("preflight.json").is_file());
    assert!(!output.join("preparation").exists());
    assert!(!output.join("browser-profile").exists());
    assert!(!job.join("blob-worker").exists());
    assert!(preflight_blob_worker(&configuration, &job, &output).is_err());
    // The real Chromium/IndexedDB owner can open the bound original profile,
    // but an unauthenticated loopback IC substitute cannot authorize transfer.
    let report = node(&prepared, "upload", config.browser_seconds + 10).unwrap();
    assert_eq!(report["state"], "failed");
    assert_eq!(report["retry_authorized"], false);
    assert!(
        output
            .join("browser-profile/publication-binding.json")
            .is_file()
    );
    assert!(requests.load(std::sync::atomic::Ordering::Relaxed) > 0);
    // The real signed native CLI fails closed before browser or verification.
    let before_dispatch = requests.load(std::sync::atomic::Ordering::Relaxed);
    assert!(run_blob_worker(&configuration, &job).is_err());
    assert_eq!(
        requests.load(std::sync::atomic::Ordering::Relaxed),
        before_dispatch + 1
    );
    let worker = job.join("blob-worker");
    assert!(worker.join("prepare-upload.stdout").is_file());
    assert!(worker.join("preparation/signed-request.cbor").is_file());
    let outcome: Value =
        serde_json::from_slice(&std::fs::read(worker.join("preparation/outcome.json")).unwrap())
            .unwrap();
    assert_eq!(outcome["outcome"], "uncertain");
    assert_eq!(outcome["provider_requests"], 0);
    assert_eq!(outcome["retry_authorized"], false);
    assert!(!worker.join("browser-profile").exists());
    assert!(!worker.join("observation").exists());
    assert!(!worker.join("completed.json").exists());
    let before = requests.load(std::sync::atomic::Ordering::Relaxed);
    assert!(run_blob_worker(&configuration, &job).is_err());
    assert_eq!(requests.load(std::sync::atomic::Ordering::Relaxed), before);
    stopping.store(true, std::sync::atomic::Ordering::Relaxed);
    serving.join().unwrap();
    std::fs::write(job.join("archive.crate"), b"changed").unwrap();
    let refused = temporary.join("changed");
    assert!(preflight_blob_worker(&configuration, &job, &refused).is_err());
    assert!(!refused.exists());
}
