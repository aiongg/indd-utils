#!/usr/bin/env python3
"""Download the test fixtures listed in tests/fixtures/manifest.json.

Each file is saved under tests/fixtures/files/ (git-ignored) and checked
against the size and SHA-256 in the manifest. Files that are already
present and match are not downloaded again. Exits non-zero if any file
cannot be downloaded or does not match.

Usage: python3 -I tools/fetch_fixtures.py
"""

import hashlib
import json
import os
import sys
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tests" / "fixtures" / "manifest.json"
DEST = ROOT / "tests" / "fixtures" / "files"
TIMEOUT = 60


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def matches(path: Path, size: int, sha256: str) -> bool:
    return path.is_file() and path.stat().st_size == size and sha256_of(path) == sha256


def target(rel: str) -> Path:
    """Resolve a manifest path, refusing anything outside DEST."""
    p = PurePosixPath(rel)
    if p.is_absolute() or ".." in p.parts or not p.parts:
        raise ValueError(f"bad path in manifest: {rel!r}")
    return DEST.joinpath(*p.parts)


def download(url: str, path: Path, size: int, sha256: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".part-")
    try:
        h = hashlib.sha256()
        n = 0
        req = urllib.request.Request(url, headers={"User-Agent": "indd-fetch-fixtures"})
        with os.fdopen(fd, "wb") as out, urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            while chunk := resp.read(1 << 16):
                n += len(chunk)
                if n > size:
                    raise ValueError(f"more than the expected {size} bytes")
                h.update(chunk)
                out.write(chunk)
        if n != size:
            raise ValueError(f"got {n} bytes, expected {size}")
        if h.hexdigest() != sha256:
            raise ValueError(f"SHA-256 {h.hexdigest()}, expected {sha256}")
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def main() -> int:
    entries = json.loads(MANIFEST.read_text(encoding="utf-8"))["files"]
    failures = 0
    for e in entries:
        rel, size, sha256 = e["path"], e["size"], e["sha256"]
        try:
            path = target(rel)
            if matches(path, size, sha256):
                print(f"present  {rel}")
                continue
            download(e["url"], path, size, sha256)
            print(f"fetched  {rel}")
        except Exception as err:
            failures += 1
            print(f"FAILED   {rel}: {err}", file=sys.stderr)
    print(f"{len(entries) - failures} of {len(entries)} fixtures ready in {DEST.relative_to(ROOT)}/")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
