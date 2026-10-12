use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Usage: ic-crates /absolute/path/bridge.json");
        std::process::exit(2);
    }
    if let Err(error) = ic_crates::run(Path::new(&args[0])) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
