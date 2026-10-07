#!/usr/bin/env python3
"""Rank what the converter does not read, over the whole corpus.

Runs `indd audit --tsv` on every distinct INDD and INDT file in
corpus/inventory.tsv and counts, for each item, the number of files in
which it occurs and is not read:

- classes: object classes of which the converter reads no object;
- chunks: chunk IDs of a class that the converter reads in no object of
  that class, in files where it reads the class;
- streams: objects without chunks (byte streams) that are not read;
- attributes: attribute IDs never looked up, per kind of attribute list;
- codes: values of known attributes that have no IDML value (unknown
  enumeration codes);
- strand kinds: kinds of strand run data that are not read;
- warnings (numbers and names replaced by #) and conversion errors.

Each table also gives the number of files in which the converter does read
the item, so items it reads nowhere ("never read") stand apart from items it
reads only in some files. This needs no reference IDML: it shows what to
decode next.

Usage: python3 -I tools/audit_corpus.py [--exclude PREFIX]... [--top N]
                                        [--out DIR] [--bin PATH] [--jobs N]
Run from the repository root after `cargo build --release`. Prints the top
N rows of each table (default 15) and writes every row to DIR (default
target/audit/) as <table>.tsv.
"""

import argparse
import os
import re
import subprocess
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "indd"


def inventory():
    """Rows of corpus/inventory.tsv (tools/inventory.py), as dicts."""
    lines = (ROOT / "corpus" / "inventory.tsv").read_text().splitlines()
    cols = lines[0].split("\t")
    return [dict(zip(cols, line.split("\t"))) for line in lines[1:]]


def warning_kind(msg):
    """A warning with its numbers and quoted names replaced, for counting."""
    msg = re.sub(r'"[^"]*"', '"…"', msg)
    msg = re.sub(r"\b0x[0-9a-fA-F]+\b", "#", msg)
    return re.sub(r"\b\d+(\.\d+)?\b", "#", msg)


class Table:
    """Items of one kind: files where each is not read, files where it is
    read, and occurrences in the files where it is not read."""

    def __init__(self, name, columns, what, has_read=True):
        self.name = name
        self.columns = columns
        self.what = what
        # Whether the item can also occur read (a class, chunk or
        # attribute), or is only ever reported unread (a code, a warning).
        self.has_read = has_read
        self.unread = Counter()
        self.read = Counter()
        self.count = Counter()

    def rows(self):
        return sorted(self.unread, key=lambda k: (-self.unread[k], -self.count[k], k))

    def write(self, out):
        with open(out / f"{self.name}.tsv", "w") as f:
            files = ["files not read", "files read"] if self.has_read else ["files"]
            f.write("\t".join([*self.columns, *files, self.what]) + "\n")
            for k in self.rows():
                n = [self.unread[k], self.read[k]] if self.has_read else [self.unread[k]]
                f.write("\t".join([*k, *map(str, n), str(self.count[k])]) + "\n")

    def show(self, top):
        rows = self.rows()
        if self.has_read:
            never = sum(1 for k in rows if not self.read[k])
            print(f"\n{self.name}: {len(rows)} not read in some file, {never} read in none "
                  f"(files not read / files read / {self.what}):")
            for k in rows[:top]:
                print(f"  {self.unread[k]:5} {self.read[k]:5} {self.count[k]:9}  {' '.join(k)}")
        else:
            print(f"\n{self.name}: {len(rows)} (files / {self.what}):")
            for k in rows[:top]:
                print(f"  {self.unread[k]:5} {self.count[k]:9}  {' '.join(k)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exclude", action="append", default=[],
                    help="leave out files whose path under corpus/ starts with this (repeatable)")
    ap.add_argument("--top", type=int, default=15, help="rows to print per table")
    ap.add_argument("--out", default=str(ROOT / "target" / "audit"),
                    help="directory for the full tables (default target/audit)")
    ap.add_argument("--bin", default=str(BIN), help="converter binary")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 1)
    args = ap.parse_args()

    seen = set()
    files = []
    for r in inventory():
        path = r["path"]
        if any(path.startswith(e) for e in args.exclude) or r["sha256"] in seen:
            continue
        seen.add(r["sha256"])
        files.append(path)

    def run(path):
        return subprocess.run([args.bin, "audit", "--tsv", ROOT / "corpus" / path],
                              capture_output=True, text=True)

    classes = Table("classes", ["class"], "objects")
    chunks = Table("chunks", ["class", "chunk"], "objects")
    streams = Table("streams", ["class"], "objects", has_read=False)
    attrs = Table("attributes", ["list", "attribute"], "occurrences")
    codes = Table("codes", ["list", "attribute", "code"], "occurrences", has_read=False)
    strands = Table("strand-kinds", ["kind"], "runs", has_read=False)
    warnings = Table("warnings", ["warning"], "warnings", has_read=False)
    errors = Counter()      # audit failed: message -> files
    conversion = Counter()  # audited, but the conversion failed
    audited = 0
    with ThreadPoolExecutor(max(1, args.jobs)) as pool:
        for path, r in zip(files, pool.map(run, files)):
            if r.returncode != 0:
                errors[r.stderr.strip().splitlines()[-1] if r.stderr.strip() else "?"] += 1
                continue
            audited += 1
            read_classes = set()
            lines = [line.split("\t") for line in r.stdout.splitlines()]
            for f in lines:
                if f[0] == "class" and int(f[3]) > 0:
                    read_classes.add(f[1])
            per_file = defaultdict(set)
            for f in lines:
                kind = f[0]
                if kind == "error":
                    conversion[warning_kind(f[1])] += 1
                elif kind == "class":
                    (classes.unread if f[3] == "0" else classes.read)[(f[1],)] += 1
                    if f[3] == "0":
                        classes.count[(f[1],)] += int(f[2])
                elif kind == "chunk" and f[1] in read_classes:
                    key = (f[1], f[2])
                    (chunks.unread if f[4] == "0" else chunks.read)[key] += 1
                    if f[4] == "0":
                        chunks.count[key] += int(f[3])
                elif kind == "stream":
                    streams.unread[(f[1],)] += 1
                    streams.count[(f[1],)] += int(f[2])
                elif kind == "attr":
                    key = (f[1], f[2])
                    (attrs.unread if f[4] == "0" else attrs.read)[key] += 1
                    if f[4] == "0":
                        attrs.count[key] += int(f[3])
                elif kind == "code":
                    key = (f[1], f[2], f[3])
                    codes.unread[key] += 1
                    codes.count[key] += int(f[4])
                elif kind == "strand":
                    strands.unread[(f[1],)] += 1
                    strands.count[(f[1],)] += int(f[2])
                elif kind == "warning":
                    per_file["warning"].add(warning_kind(f[1]))
                    warnings.count[(warning_kind(f[1]),)] += 1
            for w in per_file["warning"]:
                warnings.unread[(w,)] += 1

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    tables = [classes, chunks, streams, attrs, codes, strands, warnings]
    print(f"files audited: {audited} of {len(files)} distinct INDD/INDT files; "
          f"not audited: {sum(errors.values())}")
    for msg, n in errors.most_common(args.top):
        print(f"  {n:5}  {msg[:150]}")
    print(f"audited files whose conversion failed: {sum(conversion.values())}")
    for msg, n in conversion.most_common(args.top):
        print(f"  {n:5}  {msg[:150]}")
    for t in tables:
        t.show(args.top)
        t.write(out)
    print(f"\nfull tables in {out}/")


if __name__ == "__main__":
    main()
