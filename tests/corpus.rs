//! Tests over the local, git-ignored `corpus/` directory. Each test passes
//! without checking anything when the corpus is absent.
//!
//! `corpus/exclude.txt` (local, optional) lists paths under `corpus/` to
//! leave out, one per line, for example a folder still being downloaded.
//!
//! Two kinds of file cannot be converted, and the tests accept them: the
//! InDesign 1.x layout ([`indd::Error::Unsupported`], `header.md`) and files
//! without an object database ([`indd::Error::NoDatabase`], `database.md`).

use std::path::{Path, PathBuf};

fn corpus_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    root.is_dir().then_some(root)
}

fn excluded(root: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(root.join("exclude.txt"))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| root.join(l))
        .collect()
}

/// The INDD and INDT files under `root`, without the excluded paths.
fn corpus_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    indd_files(root, &excluded(root), &mut files);
    files
}

/// A file this crate cannot read by design (see the module comment).
fn known_unreadable(e: &indd::Error) -> bool {
    matches!(
        e,
        indd::Error::Unsupported(_) | indd::Error::NoDatabase { .. }
    )
}

fn indd_files(dir: &Path, skip: &[PathBuf], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if skip.iter().any(|s| path.starts_with(s)) {
            continue;
        }
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != ".git") {
                indd_files(&path, skip, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("indd") || e.eq_ignore_ascii_case("indt"))
        {
            out.push(path);
        }
    }
}

#[test]
fn every_corpus_header_parses() {
    let Some(root) = corpus_root() else {
        eprintln!("corpus/ not present; skipping");
        return;
    };
    let files = corpus_files(&root);
    assert!(!files.is_empty(), "corpus/ contains no INDD files");

    let failures: Vec<String> = files
        .iter()
        .filter_map(|p| {
            indd::read_header(p)
                .err()
                .filter(|e| !known_unreadable(e))
                .map(|e| format!("{}: {e}", p.display()))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

#[test]
fn every_corpus_container_has_xmp() {
    let Some(root) = corpus_root() else {
        eprintln!("corpus/ not present; skipping");
        return;
    };
    let files = corpus_files(&root);

    let failures: Vec<String> = files
        .iter()
        .filter_map(|p| {
            let bytes = std::fs::read(p).ok()?;
            let result = indd::Container::parse(&bytes).and_then(|c| c.xmp());
            match result {
                Ok(Some(_)) => None,
                Ok(None) => Some(format!("{}: no XMP", p.display())),
                Err(e) if known_unreadable(&e) => None,
                Err(e) => Some(format!("{}: {e}", p.display())),
            }
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

#[test]
fn every_corpus_object_reads() {
    let Some(root) = corpus_root() else {
        eprintln!("corpus/ not present; skipping");
        return;
    };
    let files = corpus_files(&root);

    let mut checked = 0;
    let mut failures = Vec::new();
    for p in &files {
        let bytes = std::fs::read(p).unwrap();
        let result = indd::Container::parse(&bytes).and_then(|c| {
            let db = c.database()?;
            for uid in db.uids() {
                db.object(uid)?;
            }
            Ok(())
        });
        match result {
            Ok(()) => checked += 1,
            Err(e) if known_unreadable(&e) => {}
            Err(e) => failures.push(format!("{}: {e}", p.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked > 0);
}

#[test]
fn every_corpus_file_converts() {
    let Some(root) = corpus_root() else {
        eprintln!("corpus/ not present; skipping");
        return;
    };
    let files = corpus_files(&root);
    let mut failures = Vec::new();
    for p in &files {
        let bytes = std::fs::read(p).unwrap();
        match indd::convert(&bytes, "test.indd", std::io::sink()) {
            Err(e) if !known_unreadable(&e) => failures.push(format!("{}: {e}", p.display())),
            _ => {}
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
