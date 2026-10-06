use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "usage:
  indd convert <in.indd> <out.idml>  convert to IDML
  indd info <file.indd>...         header, master page and container summary
  indd objects <file.indd>         one line per object: UID, class, length, first bytes
  indd object <file.indd> <uid>    write one object's bytes to stdout
  indd dump <file.indd> <uid>...   print objects' chunks in hex";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match (args.first().map(String::as_str), args.len()) {
        (Some("info"), n) if n > 1 => return info(&args[1..]),
        (Some("convert"), 3) => convert(&args[1], &args[2]),
        (Some("objects"), 2) => objects(&args[1]),
        (Some("dump"), n) if n > 2 => args[2..]
            .iter()
            .map(|a| parse_uid(a))
            .collect::<Result<Vec<_>, _>>()
            .and_then(|uids| dump(&args[1], &uids)),
        (Some("object"), 3) => match args[2].parse() {
            Ok(uid) => object(&args[1], uid),
            Err(_) => Err(format!("not a UID: {}", args[2]).into()),
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

type CliResult = Result<(), Box<dyn std::error::Error>>;

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

fn print_info(path: &str) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let contig = c.contig_objects().collect::<Result<Vec<_>, _>>()?;
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
    match c.database() {
        Ok(db) => println!(
            "  objects       {} ({} tree entries)",
            db.uids().count(),
            db.entries().len()
        ),
        Err(e) => println!("  objects       {e}"),
    }
    println!("  contig objs   {}", contig.len());
    println!("  xmp bytes     {xmp}");
    Ok(())
}

fn objects(path: &str) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let db = c.database()?;
    let mut out = std::io::stdout().lock();
    for uid in db.uids() {
        let data = db.object(uid)?.unwrap_or_default();
        let head: Vec<String> = data.iter().take(24).map(|b| format!("{b:02x}")).collect();
        let class = db.class_of(uid).map_or("-".into(), |c| format!("{c:#x}"));
        writeln!(out, "{uid}\t{class}\t{}\t{}", data.len(), head.join(" "))?;
    }
    Ok(())
}

fn object(path: &str, uid: u32) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let data = c
        .database()?
        .object(uid)?
        .ok_or_else(|| format!("no object with UID {uid}"))?;
    std::io::stdout().lock().write_all(&data)?;
    Ok(())
}

fn parse_uid(s: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let parsed = match s.strip_prefix("0x").or_else(|| s.strip_prefix('u')) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => s.parse(),
    };
    parsed.map_err(|_| format!("not a UID: {s}").into())
}

fn dump(path: &str, uids: &[u32]) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let db = c.database()?;
    let mut out = std::io::stdout().lock();
    for &uid in uids {
        let Some(obj) = db.get(uid)? else {
            writeln!(out, "== {uid} ({uid:#x}): no data")?;
            continue;
        };
        let class = obj.class.map_or("-".into(), |c| format!("{c:#x}"));
        writeln!(
            out,
            "== {uid} ({uid:#x}) class {class} len {}",
            obj.bytes.len()
        )?;
        match obj.chunks() {
            Some(chunks) => {
                for ch in chunks {
                    let hex: Vec<String> = ch
                        .data
                        .iter()
                        .take(120)
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    let more = if ch.data.len() > 120 { " ..." } else { "" };
                    writeln!(
                        out,
                        "  {:#07x} {:5}: {}{more}",
                        ch.id,
                        ch.data.len(),
                        hex.join(" ")
                    )?;
                }
            }
            None => writeln!(out, "  (not chunked)")?,
        }
    }
    Ok(())
}

fn convert(input: &str, output: &str) -> CliResult {
    let bytes = std::fs::read(input)?;
    let name = std::path::Path::new(input)
        .file_name()
        .map_or_else(|| input.to_string(), |n| n.to_string_lossy().into_owned());
    let file = std::io::BufWriter::new(std::fs::File::create(output)?);
    indd::convert(&bytes, &name, file)?;
    Ok(())
}
