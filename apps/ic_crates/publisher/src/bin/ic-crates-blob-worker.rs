use std::path::PathBuf;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [job] => std::env::var_os("IC_CRATES_BLOB_WORKER_CONFIG")
            .map(PathBuf::from)
            .ok_or_else(|| {
                "Set IC_CRATES_BLOB_WORKER_CONFIG to an absolute configuration file.".into()
            })
            .and_then(|configuration| {
                ic_crates::run_blob_worker(&configuration, &PathBuf::from(job))
            }),
        [mode, configuration, job, output] if mode == "preflight" => {
            ic_crates::preflight_blob_worker(
                &PathBuf::from(configuration),
                &PathBuf::from(job),
                &PathBuf::from(output),
            )
        }
        _ => Err(
            "Usage: ic-crates-blob-worker JOB | preflight CONFIG JOB NEW_OUTPUT_DIRECTORY".into(),
        ),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
