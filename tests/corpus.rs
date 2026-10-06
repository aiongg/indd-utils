//! Tests over the local, git-ignored `corpus/` directory. Each test passes
//! without checking anything when the corpus is absent.

use std::path::{Path, PathBuf};

fn corpus_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    root.is_dir().then_some(root)
}

fn indd_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != ".git") {
                indd_files(&path, out);
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
    let mut files = Vec::new();
    indd_files(&root, &mut files);
    assert!(!files.is_empty(), "corpus/ contains no INDD files");

    let failures: Vec<String> = files
        .iter()
        .filter_map(|p| {
            indd::read_header(p)
                .err()
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
