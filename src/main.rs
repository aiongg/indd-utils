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
        if let Err(e) = print_info(path) {
            eprintln!("{path}: {e}");
            status = ExitCode::FAILURE;
        }
    }
    status
}

fn print_info(path: &str) -> Result<(), indd::Error> {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let objects = c.contig_objects().collect::<Result<Vec<_>, _>>()?;
    let xmp = c.xmp()?.map_or(0, <[u8]>::len);
    println!("{path}");
    println!("  version       {}", c.header.version);
    println!("  byte order    {:?}", c.header.byte_order);
    println!(
        "  master page   {} (sequence {})",
        c.active,
        c.master().sequence
    );
    println!("  db pages      {}", c.master().db_pages);
    println!("  contig objs   {}", objects.len());
    println!("  xmp bytes     {xmp}");
    Ok(())
}
