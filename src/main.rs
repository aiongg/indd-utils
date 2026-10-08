//! The `indd` command: converts INDD files to IDML and prints what is
//! inside an INDD file (header, objects, chunks, XMP, audit). Run it
//! without arguments for the list of subcommands.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::todo,
        clippy::unreachable
    )
)]

use std::io::Write;

use std::process::ExitCode;

const USAGE: &str = "usage:
  indd convert <in.indd> <out.idml>  convert to IDML
  indd info <file.indd>...         header, master page and container summary
  indd objects <file.indd>         one line per object: UID, class, length, first bytes
  indd uids <file.indd>            one line per object: UID, class (no object data read)
  indd object <file.indd> <uid>    write one object's bytes to stdout
  indd dump [--full] <file.indd> <uid>...
                                   print objects' chunks in hex (the first
                                   120 bytes of each, or all with --full)
  indd xmp <file.indd>             write the document's XMP packet to stdout
  indd audit [--tsv] <file.indd>   what the converter does not read in a document";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match (args.first().map(String::as_str), args.len()) {
        (Some("info"), n) if n > 1 => return info(&args[1..]),
        (Some("convert"), 3) => convert(&args[1], &args[2]),
        (Some("objects"), 2) => objects(&args[1]),
        (Some("xmp"), 2) => xmp(&args[1]),
        (Some("uids"), 2) => uids(&args[1]),
        (Some("audit"), 2) => audit(&args[1], false),
        (Some("audit"), 3) if args[1] == "--tsv" => audit(&args[2], true),
        (Some("dump"), n) if n > 3 && args[1] == "--full" => args[3..]
            .iter()
            .map(|a| parse_uid(a))
            .collect::<Result<Vec<_>, _>>()
            .and_then(|uids| dump(&args[2], &uids, usize::MAX)),
        (Some("dump"), n) if n > 2 => args[2..]
            .iter()
            .map(|a| parse_uid(a))
            .collect::<Result<Vec<_>, _>>()
            .and_then(|uids| dump(&args[1], &uids, 120)),
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

fn hex(class: Option<u32>) -> String {
    class.map_or("-".into(), |c| format!("{c:#x}"))
}

/// Print what the converter does not read in a document: classes, chunks,
/// attributes, codes and strand kinds it never looks at, and its warnings.
fn audit(path: &str, tsv: bool) -> CliResult {
    let bytes = std::fs::read(path)?;
    let name = std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
    let a = indd::audit::audit(&bytes, &name)?;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    if tsv {
        if let Some(e) = &a.error {
            writeln!(out, "error\t{e}")?;
        }
        for (class, c) in &a.classes {
            writeln!(out, "class\t{}\t{}\t{}", hex(*class), c.objects, c.read)?;
        }
        for ((class, id), (n, read)) in &a.chunks {
            writeln!(out, "chunk\t{}\t{id:#x}\t{n}\t{read}", hex(*class))?;
        }
        for (class, n) in &a.unread_streams {
            writeln!(out, "stream\t{}\t{n}", hex(*class))?;
        }
        for ((list, id), (n, read)) in &a.attrs {
            writeln!(
                out,
                "attr\t{}\t{id:#x}\t{n}\t{}",
                list.name(),
                u8::from(*read)
            )?;
        }
        for ((list, id, code), n) in &a.unknown_codes {
            writeln!(out, "code\t{}\t{id:#x}\t{code:#x}\t{n}", list.name())?;
        }
        for (kind, n) in &a.unknown_strand_kinds {
            writeln!(out, "strand\t{kind:#x}\t{n}")?;
        }
        for w in &a.warnings {
            writeln!(out, "warning\t{}", w.replace(['\t', '\n'], " "))?;
        }
        return Ok(());
    }
    writeln!(out, "{path}")?;
    if let Some(e) = &a.error {
        writeln!(out, "conversion failed: {e}")?;
    }
    let objects: usize = a.classes.values().map(|c| c.objects).sum();
    let read: usize = a.classes.values().map(|c| c.read).sum();
    writeln!(
        out,
        "objects: {objects} in {} classes; {read} read",
        a.classes.len()
    )?;
    let unread: Vec<_> = a.classes.iter().filter(|(_, c)| c.read == 0).collect();
    writeln!(out, "classes not read ({}; class: objects):", unread.len())?;
    for (class, c) in unread {
        writeln!(out, "  {:>8}: {}", hex(*class), c.objects)?;
    }
    let partly: Vec<_> = a
        .classes
        .iter()
        .filter(|(_, c)| c.read > 0 && c.read < c.objects)
        .collect();
    writeln!(
        out,
        "classes partly read ({}; class: objects read of objects):",
        partly.len()
    )?;
    for (class, c) in partly {
        writeln!(out, "  {:>8}: {} of {}", hex(*class), c.read, c.objects)?;
    }
    let chunks: Vec<_> = a.unread_chunks().collect();
    writeln!(
        out,
        "chunks never read in read classes ({}; class/chunk: objects):",
        chunks.len()
    )?;
    for ((class, id), n) in &chunks {
        writeln!(out, "  {:>8}/{id:#x}: {n}", hex(*class))?;
    }
    writeln!(
        out,
        "byte-stream objects not read ({}; class: objects):",
        a.unread_streams.len()
    )?;
    for (class, n) in &a.unread_streams {
        writeln!(out, "  {:>8}: {n}", hex(*class))?;
    }
    let attrs: Vec<_> = a.unread_attrs().collect();
    writeln!(
        out,
        "attributes never looked up ({}; list, ID: occurrences):",
        attrs.len()
    )?;
    for ((list, id), n) in &attrs {
        writeln!(out, "  {:>20}, {id:#x}: {n}", list.name())?;
    }
    writeln!(
        out,
        "unknown codes ({}; list, attribute ID, code: occurrences):",
        a.unknown_codes.len()
    )?;
    for ((list, id, code), n) in &a.unknown_codes {
        writeln!(out, "  {:>20}, {id:#x}, {code:#x}: {n}", list.name())?;
    }
    writeln!(
        out,
        "strand run kinds not read ({}; kind: runs):",
        a.unknown_strand_kinds.len()
    )?;
    for (kind, n) in &a.unknown_strand_kinds {
        writeln!(out, "  {kind:#x}: {n}")?;
    }
    writeln!(out, "warnings ({}):", a.warnings.len())?;
    for w in &a.warnings {
        writeln!(out, "  {w}")?;
    }
    Ok(())
}

fn uids(path: &str) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let db = c.database()?;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for uid in db.uids() {
        let class = db.class_of(uid).map_or("-".into(), |c| format!("{c:#x}"));
        writeln!(out, "{uid}\t{class}")?;
    }
    Ok(())
}

fn xmp(path: &str) -> CliResult {
    let bytes = std::fs::read(path)?;
    let c = indd::Container::parse(&bytes)?;
    let packet = c.xmp()?.ok_or("the file has no XMP packet")?;
    std::io::stdout().lock().write_all(packet)?;
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

/// Print the chunks of objects, at most `limit` bytes of each.
fn dump(path: &str, uids: &[u32], limit: usize) -> CliResult {
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
                        .take(limit)
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    let more = if ch.data.len() > limit { " ..." } else { "" };
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
    for warning in indd::convert_into(&bytes, &name, file)? {
        eprintln!("warning: {warning}");
    }
    Ok(())
}
