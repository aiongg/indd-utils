use std::process::ExitCode;

const USAGE: &str = "usage: indd info <file.indd>...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("info") if args.len() > 1 => info(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn info(paths: &[String]) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    for path in paths {
        match indd::read_header(path) {
            Ok(h) => println!(
                "{path}\t{}\t{:?}\t{}",
                String::from_utf8_lossy(&h.kind),
                h.byte_order,
                h.version
            ),
            Err(e) => {
                eprintln!("{path}: {e}");
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}
