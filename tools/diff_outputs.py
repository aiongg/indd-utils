#!/usr/bin/env python3
"""Compare the IDML output of two converter revisions over the corpus.

Usage: python3 -I tools/diff_outputs.py [OLD [NEW]] [--exclude PREFIX]...
                                        [--file SUBSTR] [--jobs N] [--show N]

OLD and NEW are git revisions; NEW may also be `.`, the working tree.
Defaults: OLD is HEAD, NEW is the working tree. Each revision is exported
with `git archive` and built in release mode into its own target directory
under ~/.cache/indd-diff/ (INDD_DIFF_CACHE overrides it); the binary built
from a commit is kept under bin/<commit> and reused. The working tree is
built in place with its own target directory there.

Then every INDD and INDT file under corpus/ (files with the same SHA-256
once; without the paths in corpus/exclude.txt and the --exclude prefixes)
is converted with both binaries, in parallel. Two outputs are the same
when the packages have the same entries, in the same order, with the same
bytes. ZIP headers are not compared (the writer stores a fixed date), so
only content differences count. Also reported: files where one binary
fails and the other does not, and files whose warnings differ.

Prints the counts, the differing package entries by kind, and the first
--show N differing files; writes the full list to diff.tsv in the cache
directory. The outputs that differ, or where only one binary fails, stay
in out/old and out/new there for inspection; the others are deleted as
soon as they are compared, and each run starts with empty folders.
Exit status 1 if any output, failure or warning differs.
"""

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tarfile
import io
import zipfile
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = Path(os.environ.get("INDD_DIFF_CACHE", Path.home() / ".cache" / "indd-diff"))


def git(*args):
    return subprocess.run(["git", "-C", str(ROOT), *args], check=True,
                          capture_output=True, text=True).stdout.strip()


def build(rev):
    """Path of a release binary for `rev` (`.` is the working tree)."""
    env = dict(os.environ)
    if rev == ".":
        env["CARGO_TARGET_DIR"] = str(CACHE / "target-worktree")
        subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, env=env, check=True)
        return CACHE / "target-worktree" / "release" / "indd"
    sha = git("rev-parse", "--verify", rev + "^{commit}")
    binary = CACHE / "bin" / sha
    if binary.exists():
        return binary
    src = CACHE / "src" / sha
    if not (src / "Cargo.toml").exists():
        src.mkdir(parents=True, exist_ok=True)
        data = subprocess.run(["git", "-C", str(ROOT), "archive", sha],
                              check=True, capture_output=True).stdout
        with tarfile.open(fileobj=io.BytesIO(data)) as tar:
            tar.extractall(src, filter="data")
    # One target directory per commit: cargo does not include the source
    # path in its fingerprint, so a shared directory would reuse a binary
    # built from another commit's source.
    env["CARGO_TARGET_DIR"] = str(CACHE / "target" / sha)
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=src, env=env, check=True)
    binary.parent.mkdir(parents=True, exist_ok=True)
    built = CACHE / "target" / sha / "release" / "indd"
    binary.write_bytes(built.read_bytes())
    binary.chmod(0o755)
    # The binary is kept; the commit's build directory is not needed again.
    shutil.rmtree(CACHE / "target" / sha)
    return binary


def corpus_files(exclude, substr):
    """Distinct INDD and INDT files under corpus/, largest first."""
    corpus = ROOT / "corpus"
    known = {}
    inv = corpus / "inventory.tsv"
    if inv.exists():
        lines = inv.read_text().splitlines()
        cols = lines[0].split("\t")
        for line in lines[1:]:
            row = dict(zip(cols, line.split("\t")))
            known[row["path"]] = row["sha256"]
    ex = corpus / "exclude.txt"
    if ex.exists():
        exclude = exclude + [line.strip() for line in ex.read_text().splitlines()
                             if line.strip() and not line.startswith("#")]
    seen, out = set(), []
    for f in sorted(corpus.rglob("*")):
        if f.suffix.lower() not in (".indd", ".indt") or not f.is_file():
            continue
        path = str(f.relative_to(corpus))
        if any(path.startswith(e) for e in exclude) or substr and substr not in path:
            continue
        d = known.get(path) or hashlib.sha256(f.read_bytes()).hexdigest()
        if d in seen:
            continue
        seen.add(d)
        out.append(f)
    out.sort(key=lambda f: -f.stat().st_size)
    return out


def convert(binary, src, dst):
    """(exit status, stderr lines) of one conversion."""
    p = subprocess.run([str(binary), "convert", str(src), str(dst)],
                       capture_output=True, text=True, errors="replace")
    return p.returncode, p.stderr.splitlines()


def entries(path):
    try:
        with zipfile.ZipFile(path) as z:
            return [(i.filename, z.read(i)) for i in z.infolist()]
    except (OSError, zipfile.BadZipFile):
        return None


def kind(name):
    """Entry name with its file name replaced by its directory, for counting."""
    return name.rsplit("/", 1)[0] + "/*" if "/" in name else name


def compare(old, new, i, f):
    """Compare one file's outputs; keep them only if they differ."""
    a, b = CACHE / "out" / "old" / f"{i}.idml", CACHE / "out" / "new" / f"{i}.idml"
    result = compare_outputs(old, new, f, a, b)
    if not result["entries"] and result["status"][0] == result["status"][1]:
        a.unlink(missing_ok=True)
        b.unlink(missing_ok=True)
    return result


def compare_outputs(old, new, f, a, b):
    sa, wa = convert(old, f, a)
    sb, wb = convert(new, f, b)
    result = {"file": f, "status": (sa, sb), "warnings": wa != wb, "entries": []}
    if sa != 0 or sb != 0:
        return result
    if a.read_bytes() == b.read_bytes():
        return result
    ea, eb = entries(a), entries(b)
    if ea is None or eb is None:
        result["entries"] = ["(unreadable package)"]
        return result
    if [n for n, _ in ea] != [n for n, _ in eb]:
        result["entries"].append("(entry list)")
    da, db = dict(ea), dict(eb)
    result["entries"] += [n for n in da if n in db and da[n] != db[n]]
    return result


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("old", nargs="?", default="HEAD")
    ap.add_argument("new", nargs="?", default=".")
    ap.add_argument("--exclude", action="append", default=[])
    ap.add_argument("--file", help="only files whose path contains SUBSTR")
    ap.add_argument("--jobs", type=int, default=os.cpu_count())
    ap.add_argument("--show", type=int, default=20)
    args = ap.parse_args()

    old, new = build(args.old), build(args.new)
    for d in ("old", "new"):
        shutil.rmtree(CACHE / "out" / d, ignore_errors=True)
        (CACHE / "out" / d).mkdir(parents=True)
    files = corpus_files(args.exclude, args.file)
    print(f"{len(files)} files; old {args.old} ({old}), new {args.new} ({new})",
          file=sys.stderr)
    with ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(lambda t: compare(old, new, *t), enumerate(files)))

    corpus = ROOT / "corpus"
    differ = [r for r in results if r["entries"]]
    status = [r for r in results if r["status"][0] != r["status"][1]]
    failed = [r for r in results if r["status"] != (0, 0) and r["status"][0] == r["status"][1]]
    warned = [r for r in results if r["warnings"]]
    kinds = Counter(kind(n) for r in differ for n in r["entries"])
    with open(CACHE / "diff.tsv", "w") as out:
        out.write("index\tpath\tstatus_old\tstatus_new\twarnings_differ\tentries\n")
        for i, r in enumerate(results):
            if r["entries"] or r["warnings"] or r["status"][0] != r["status"][1]:
                out.write(f"{i}\t{r['file'].relative_to(corpus)}\t{r['status'][0]}\t"
                          f"{r['status'][1]}\t{r['warnings']}\t{' '.join(r['entries'])}\n")

    print(f"files converted:            {len(results)}")
    print(f"outputs differ:             {len(differ)}")
    print(f"package entries differ:     {sum(kinds.values())}")
    print(f"failure status differs:     {len(status)}")
    print(f"warnings differ:            {len(warned)}")
    print(f"failed with both:           {len(failed)}")
    for k, n in kinds.most_common():
        print(f"  {n:6d}  {k}")
    for r in (differ + status + warned)[:args.show]:
        i = results.index(r)
        print(f"  [{i}] {r['file'].relative_to(corpus)}: status {r['status']}, "
              f"warnings {'differ' if r['warnings'] else 'same'}, "
              f"entries {' '.join(r['entries'][:5])}")
    print(f"details: {CACHE / 'diff.tsv'}; differing outputs in {CACHE / 'out'}")
    return 1 if differ or status or warned else 0


if __name__ == "__main__":
    sys.exit(main())
