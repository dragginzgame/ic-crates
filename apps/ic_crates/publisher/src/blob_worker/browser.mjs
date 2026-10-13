import { constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

async function read(path, maximum) {
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size < 1 || stat.size > maximum) throw new Error('file');
    const bytes = Buffer.alloc(stat.size);
    let offset = 0;
    while (offset < bytes.length) {
      const result = await file.read(bytes, offset, bytes.length - offset, null);
      if (!result.bytesRead) throw new Error('eof');
      offset += result.bytesRead;
    }
    if ((await file.read(Buffer.alloc(1), 0, 1, null)).bytesRead) throw new Error('eof');
    return bytes;
  } finally { await file.close(); }
}

let browser;
try {
  if (Number(process.versions.node.split('.')[0]) !== 24) throw new Error('node');
  const [mode, inputPath, ...extra] = process.argv.slice(2);
  if (!['prepare', 'upload'].includes(mode) || extra.length) throw new Error('arguments');
  const input = JSON.parse(await read(inputPath, 300 * 1024));
  const config = input.configuration;
  if (mode === 'prepare') globalThis.fetch = () => { throw new Error('offline'); };
  const sdk = await import(pathToFileURL(config.sdk_module.path).href);
  const signerJSON = (await read(config.browser_identity, 1024)).toString('utf8');
  const advertised = sdk.Ed25519KeyIdentity.fromJSON(signerJSON);
  const identity = sdk.Ed25519KeyIdentity.fromSecretKey(advertised.getKeyPair().secretKey);
  if (identity.getPrincipal().toText() !== input.binding.uploader ||
      advertised.getPrincipal().toText() !== input.binding.uploader) throw new Error('identity');
  const bootstrap = {
    schema: 1, operation: 'bootstrap', signer: { kind: 'ed25519', json: signerJSON },
    configuration: {
      host: input.replica_url, rootKey: new Uint8Array(input.root_key),
      service: input.binding.service, tenant: input.binding.tenant, uploader: input.binding.uploader,
      project: config.project, bucket: config.bucket, origin: config.provider_origin,
      maxBodyBytes: config.max_body_bytes, maxRequests: config.max_requests,
      maxRequestBytes: config.max_request_bytes, maxTotalRequestBytes: config.max_total_request_bytes,
      maxJobs: 1, timeoutSeconds: config.browser_seconds,
    },
    journal: { database: 'ic-crates-publication', maxSlots: 1, mode: 'create' },
  };
  sdk.publicationConfiguration({ ...bootstrap.configuration, identity, intents: null });
  if (mode === 'prepare') {
    const bytes = await read(input.body, config.max_body_bytes);
    const digest = createHash('sha256').update(bytes).digest('hex');
    if (digest !== input.body_sha256 || bytes.length.toString() !== input.bytes) throw new Error('body');
    const prepared = await sdk.StorageClient.prepareFile(new Uint8Array(bytes), 'application/gzip');
    console.log(JSON.stringify({ schema: 1, root: prepared.hash, bytes: prepared.byteLength.toString(),
      manifest_json: prepared.manifestJSON, body_sha256: digest, uploader: identity.getPrincipal().toText(),
      provider_requests: 0, retry_authorized: false }));
  } else {
    const { chromium } = await import(pathToFileURL(config.playwright_module.path).href);
    const { launchPublicationBrowser } = await import(pathToFileURL(config.launcher_module.path).href);
    const selectedChromium = {
      launchPersistentContext: (profile, options) => chromium.launchPersistentContext(profile,
        { ...options, executablePath: config.chromium.path }),
    };
    browser = await launchPublicationBrowser(selectedChromium, {
      profile: input.profile, assetPort: config.asset_port, hostBundle: config.host_bundle.path,
      workerBundle: config.worker_bundle.path, bootstrap, nativeSession: null,
    });
    console.log(JSON.stringify(await browser.execute({ id: 1, index: 0, action: 'upload',
      transfer: { binding: input.binding, body: input.body, body_sha256: input.body_sha256,
        manifest_json: input.manifest_json, bytes: input.bytes,
        preparation: { content_type: 'application/gzip' } },
    })));
  }
} catch {
  console.error('Blob browser phase refused; retain its original evidence.');
  process.exitCode = 3;
} finally {
  try { await browser?.close(); } catch {
    console.error('Blob browser cleanup failed; inspect the retained profile.');
    process.exitCode = 3;
  }
}
