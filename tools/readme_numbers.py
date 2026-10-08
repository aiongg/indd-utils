#!/usr/bin/env python3
"""Write the numbers of a compare.py run into README.md.

Usage: python3 -I tools/readme_numbers.py [--schemas DIR --jing DIR]
                                          [--summary PATH] [--readme PATH] [--check]

With --schemas and --jing, first builds the converter (cargo build
--release) and runs the full measurement the README reports:

    python3 -I tools/compare.py --all --exclude own/ --schemas DIR --jing DIR

Without them, uses the summary of the last run. Either way it reads
target/compare/summary.json (--summary), which compare.py writes, and
replaces the text between the markers `<!-- numbers:start -->` and
`<!-- numbers:end -->` in README.md (--readme) with the table of numbers.

The summary must come from a run over the whole corpus (no --limit or
--file) with --all and schema validation, and without the privately held
samples (--exclude own/); otherwise nothing is written and the exit
status is 1. With --check, nothing is written either; the exit status is
1 if the README's numbers differ from the summary's.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
START = "<!-- numbers:start -->"
END = "<!-- numbers:end -->"

# Files rejected for a correct reason (compare.py REJECTIONS), by the
# start of the error message: how the table names them.
REJECTION_KINDS = [
    ("not an INDD file", "not INDD files"),
    ("not supported yet: InDesign 1.x", "InDesign 1.x files"),
    ("file truncated", "truncated"),
    ("no object database", "without an object database"),
]


def run_compare(schemas, jing):
    """Build the converter and run the full measurement."""
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target.is_absolute():
        target = ROOT / target
    subprocess.run([sys.executable, "-I", str(ROOT / "tools" / "compare.py"), "--all",
                    "--exclude", "own/", "--schemas", schemas, "--jing", jing,
                    "--bin", str(target / "release" / "indd")],
                   cwd=ROOT, check=True)


def problems(summary):
    """Why the summary cannot give the README's numbers."""
    o = summary["options"]
    out = []
    if not any(e.rstrip("/") == "own" for e in o["exclude"]):
        out.append("the run included the privately held samples (use --exclude own/)")
    if o["limit"] or o["file"]:
        out.append("the run covered part of the corpus (--limit or --file)")
    if not o["all"]:
        out.append("the run left out the files without a reference IDML (use --all)")
    if not o["validated"]:
        out.append("the run did not validate the output (use --schemas and --jing)")
    return out


def counted(group, key, label):
    """The files of a group (`label`) under `key` (`failures`: valid files
    whose conversion fails; `rejected`: files rejected for a correct
    reason), with the rejected ones by kind."""
    messages = group.get(key) or []
    text = f"{len(messages):,} of {group['files']:,} {label}"
    if key != "rejected" or not messages:
        return text
    kinds = {}
    for message in messages:
        name = next((n for start, n in REJECTION_KINDS if message.startswith(start)),
                    "other reasons")
        kinds[name] = kinds.get(name, 0) + 1
    ranked = sorted(kinds.items(), key=lambda kv: -kv[1])
    return text + " (" + ", ".join(f"{n:,} {name}" for name, n in ranked) + ")"


def coverage(st):
    return 100 * st["reproduced"] / st["values"] if st["values"] else 100.0


def table(summary):
    """The text between the markers."""
    paired, others = summary["paired"], summary["others"]
    trusted, every = summary["trustworthy"], summary["all_pairs"]
    invalid = paired["invalid"] + others["invalid"]
    if invalid:
        invalid_text = (f"{paired['invalid']:,} paired files, "
                        f"{others['invalid']:,} other files")
    else:
        invalid_text = "0"
    stories = trusted["stories"]
    rows = [
        ("Conversion failures",
         f"{counted(paired, 'failures', 'paired files')}; "
         f"{counted(others, 'failures', 'other files')}"),
        ("Rejected files",
         f"{counted(paired, 'rejected', 'paired files')}; "
         f"{counted(others, 'rejected', 'other files')}"),

        ("Schema validation failures", invalid_text),
        ("Value coverage, trustworthy pairs",
         f"{coverage(trusted):.2f} % ({trusted['reproduced']:,} of "
         f"{trusted['values']:,} values)"),
        ("Value coverage, all pairs", f"{coverage(every):.2f} %"),
        ("Story text, trustworthy pairs",
         f"{stories['exact']:,} of {sum(stories.values()):,} stories exact, "
         f"{stories['differ']:,} differ, {stories['missing']:,} missing"),
    ]
    lines = [
        "From `tools/compare.py --all` with schema validation, without the",
        "privately held samples:",
        "",
        "| Measure | Result |",
        "|---|---|",
    ]
    lines += [f"| {k} | {v} |" for k, v in rows]
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--schemas", help="IDML RelaxNG schema directory: run compare.py first")
    ap.add_argument("--jing", help="directory with jing.jar, isorelax.jar, saxon.jar")
    ap.add_argument("--summary", default=str(ROOT / "target" / "compare" / "summary.json"))
    ap.add_argument("--readme", default=str(ROOT / "README.md"))
    ap.add_argument("--check", action="store_true",
                    help="change nothing; exit 1 if the README's numbers differ")
    args = ap.parse_args()
    if bool(args.schemas) != bool(args.jing):
        ap.error("--schemas and --jing go together")
    if args.schemas:
        run_compare(args.schemas, args.jing)

    summary = json.loads(Path(args.summary).read_text())
    why = problems(summary)
    if why:
        print(f"{args.summary}: not used for the README:", file=sys.stderr)
        for w in why:
            print(f"  {w}", file=sys.stderr)
        return 1

    readme = Path(args.readme)
    text = readme.read_text()
    start, end = text.find(START), text.find(END)
    if start < 0 or end < start:
        print(f"{readme}: no {START} ... {END} section", file=sys.stderr)
        return 1
    new = text[:start + len(START)] + "\n" + table(summary) + "\n" + text[end:]
    if new == text:
        print(f"{readme}: numbers up to date")
        return 0
    if args.check:
        print(f"{readme}: numbers differ from {args.summary}")
        return 1
    readme.write_text(new)
    print(f"{readme}: numbers updated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
