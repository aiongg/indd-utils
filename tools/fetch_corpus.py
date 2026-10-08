#!/usr/bin/env python3
"""Download the openly licensed corpus samples listed in
tools/corpus-manifest.json.

Each file is saved under corpus/open/ (git-ignored) and checked against
the size and SHA-256 in the manifest. A file comes either from its own
pinned URL or from a zip archive: the archive is downloaded once into
corpus/open/.archives/, checked against its size and checksum, and only
the members the manifest lists are extracted. Files that are already
present and match are not downloaded again. Exits non-zero if any file
cannot be downloaded or does not match.

The manifest names each file's licence and source. The files are about
5.5 GB; the downloads, archives included, somewhat more.

Usage: python3 -I tools/fetch_corpus.py [--source SLUG ...] [--list]
                                        [--keep-archives]
  --source SLUG    fetch only the files of these sources (repeatable)
  --list           list the sources with their licence and file count
  --keep-archives  keep the downloaded archives after extraction
"""

import hashlib
import io
import json
import os
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from collections import defaultdict
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tools" / "corpus-manifest.json"
DEST = ROOT / "corpus" / "open"
ARCHIVES = DEST / ".archives"
TIMEOUT = 120
USER_AGENT = "indd-utils-fetch-corpus/1 (format research; one request at a time)"
# Seconds between two requests to the same host; some hosts limit bursts.
PAUSE = 1.0
RETRIES = 3

_last_request: dict[str, float] = {}


def sha256_of(path: Path) -> str:
    return digest_of(path, "sha256")


def digest_of(path: Path, algorithm: str) -> str:
    h = hashlib.new(algorithm)
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def matches(path: Path, size: int, sha256: str) -> bool:
    return path.is_file() and path.stat().st_size == size and sha256_of(path) == sha256


def target(rel: str, base: Path = DEST) -> Path:
    """Resolve a manifest path, refusing anything outside `base`."""
    p = PurePosixPath(rel)
    if p.is_absolute() or ".." in p.parts or not p.parts or p.parts[0].startswith("."):
        raise ValueError(f"bad path in manifest: {rel!r}")
    return base.joinpath(*p.parts)


def checksum(entry: dict) -> tuple[str, str]:
    """The strongest checksum an archive entry gives: (algorithm, hex)."""
    for algorithm in ("sha256", "sha1", "md5"):
        if entry.get(algorithm):
            return algorithm, entry[algorithm]
    raise ValueError(f"archive {entry['id']!r} has no checksum")


def pace(url: str) -> None:
    host = urllib.parse.urlparse(url).netloc
    wait = _last_request.get(host, 0.0) + PAUSE - time.monotonic()
    if wait > 0:
        time.sleep(wait)
    _last_request[host] = time.monotonic()


def download(url: str, path: Path, size: int, algorithm: str, expected: str) -> None:
    """Download `url` to `path`, checking its size and digest; retry on
    HTTP 429 and 5xx with a growing pause."""
    if urllib.parse.urlparse(url).scheme != "https":
        raise ValueError(f"not an https URL: {url}")
    path.parent.mkdir(parents=True, exist_ok=True)
    for attempt in range(RETRIES + 1):
        pace(url)
        fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".part-")
        try:
            h = hashlib.new(algorithm)
            n = 0
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with os.fdopen(fd, "wb") as out, urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
                while chunk := resp.read(1 << 16):
                    n += len(chunk)
                    if n > size:
                        raise ValueError(f"more than the expected {size} bytes")
                    h.update(chunk)
                    out.write(chunk)
            if n != size:
                raise ValueError(f"got {n} bytes, expected {size}")
            if h.hexdigest() != expected:
                raise ValueError(f"{algorithm} {h.hexdigest()}, expected {expected}")
            os.replace(tmp, path)
            return
        except urllib.error.HTTPError as err:
            Path(tmp).unlink(missing_ok=True)
            if attempt == RETRIES or not (err.code == 429 or err.code >= 500):
                raise
            time.sleep(PAUSE * 10 * (attempt + 1))
        except BaseException:
            Path(tmp).unlink(missing_ok=True)
            raise


def read_member(archive: Path, member, size: int) -> bytes:
    """The bytes of an archive member; a list names a member of a zip
    inside the archive. Reads at most `size` + 1 bytes of the member."""
    names = member if isinstance(member, list) else [member]
    with zipfile.ZipFile(archive) as z:
        for name in names[:-1]:
            z = zipfile.ZipFile(io.BytesIO(z.read(name)))
        with z.open(names[-1]) as f:
            data = f.read(size + 1)
    if len(data) != size:
        raise ValueError(f"member has {'more than ' if len(data) > size else ''}{min(len(data), size)} bytes, expected {size}")
    return data


def write_checked(path: Path, data: bytes, sha256: str) -> None:
    if hashlib.sha256(data).hexdigest() != sha256:
        raise ValueError(f"SHA-256 {hashlib.sha256(data).hexdigest()}, expected {sha256}")
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".part-")
    try:
        with os.fdopen(fd, "wb") as out:
            out.write(data)
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def main(argv: list[str]) -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    sources = manifest["sources"]
    if "--list" in argv:
        for s in sources:
            print(f"{s['slug']}\t{s['licence']}\t{s['files']}\t{s['homepage']}")
        return 0
    wanted = {argv[i + 1] for i, a in enumerate(argv[:-1]) if a == "--source"}
    unknown = wanted - {s["slug"] for s in sources}
    if unknown:
        print(f"unknown sources: {', '.join(sorted(unknown))}", file=sys.stderr)
        return 2
    keep = "--keep-archives" in argv
    homes = {s["homepage"] for s in sources if not wanted or s["slug"] in wanted}
    files = [f for f in manifest["files"] if f["source"] in homes]
    archives = {a["id"]: a for a in manifest["archives"]}

    failures = 0
    by_archive = defaultdict(list)
    for f in files:
        rel = f["path"]
        try:
            path = target(rel)
            if matches(path, f["size"], f["sha256"]):
                print(f"present  {rel}")
            elif "archive" in f:
                by_archive[f["archive"]].append((f, path))
            else:
                download(f["url"], path, f["size"], "sha256", f["sha256"])
                print(f"fetched  {rel}")
        except Exception as err:
            failures += 1
            print(f"FAILED   {rel}: {err}", file=sys.stderr)

    for aid, members in by_archive.items():
        try:
            a = archives[aid]
            algorithm, expected = checksum(a)
            archive = target(aid, ARCHIVES)
            if not (archive.is_file() and archive.stat().st_size == a["size"]
                    and digest_of(archive, algorithm) == expected):
                download(a["url"], archive, a["size"], algorithm, expected)
                print(f"fetched  archive {aid}")
        except Exception as err:
            failures += len(members)
            print(f"FAILED   archive {aid}: {err}", file=sys.stderr)
            continue
        for f, path in members:
            try:
                write_checked(path, read_member(archive, f["member"], f["size"]), f["sha256"])
                print(f"unpacked {f['path']}")
            except Exception as err:
                failures += 1
                print(f"FAILED   {f['path']}: {err}", file=sys.stderr)
        if not keep:
            archive.unlink(missing_ok=True)

    print(f"{len(files) - failures} of {len(files)} files ready in {DEST.relative_to(ROOT)}/")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
