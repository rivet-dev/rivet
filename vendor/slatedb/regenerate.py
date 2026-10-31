#!/usr/bin/env python3
"""Regenerate the committed SlateDB source from its pinned release and ordered patches.

`out/` is generated: never edit it by hand. Change SlateDB by editing or adding a patch in
`patches/`, listing it in PATCHES, and rerunning this script. `--check` regenerates into a
temporary directory and fails if the result differs from the committed `out/`.
"""

import argparse
import filecmp
import hashlib
import io
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

VERSION = "0.17.0"
SHA256 = "543da8a0c19e1a6af286a3c314a1c936af598603f3af98549514fffffb837334"
DIRECTORY = Path(__file__).resolve().parent
ROOT = DIRECTORY.parents[1]
DESTINATION = DIRECTORY / "out"
URL = f"https://static.crates.io/crates/slatedb/slatedb-{VERSION}.crate"

# Applied in this order, each with zero fuzz. Every patch has a sibling `.md` that states
# what it changes, which Depot caller needs it, and when it can be dropped.
PATCHES = [
    "0000-packaging.patch",
    "0001-leadership-epoch.patch",
    "0002-retained-checkpoint-id.patch",
    "0003-wal-replay-marker.patch",
    "0004-compaction-filter-full-input.patch",
    "0005-projected-clone-verification.patch",
    "0006-view-watermark-merge.patch",
    "0007-cache-memory-usage.patch",
    "0008-boundary-check-reuse.patch",
    "0009-local-write-notices.patch",
    "0010-backpressure-log-level.patch",
]


def load_archive(path):
    if path:
        data = path.read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError(f"checksum mismatch for slatedb {VERSION}")
    return data


def build(data, staging):
    """Extracts and patches the crate under `staging`, returning the patched directory."""
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        archive.extractall(staging, filter="data")
    source = staging / f"slatedb-{VERSION}"
    for name in PATCHES:
        subprocess.run(
            ["patch", "--batch", "--forward", "--fuzz=0", "--quiet", "-p1"],
            input=(DIRECTORY / "patches" / name).read_bytes(),
            cwd=source,
            check=True,
        )
    # A failed hunk leaves rejects behind instead of failing every patch implementation.
    leftovers = [p for p in source.rglob("*") if p.suffix in (".rej", ".orig") and p.name != "Cargo.toml.orig"]
    if leftovers:
        raise RuntimeError(f"patch left {leftovers[0].relative_to(source)}")
    return source


def differences(left, right):
    """Lists paths that differ between two trees, including hidden and missing files."""
    found = []

    def walk(comparison, prefix):
        for name in comparison.left_only + comparison.right_only + comparison.funny_files:
            found.append(prefix / name)
        _, mismatch, errors = filecmp.cmpfiles(
            comparison.left, comparison.right, comparison.common_files, shallow=False
        )
        found.extend(prefix / name for name in mismatch + errors)
        for name, sub in comparison.subdirs.items():
            walk(sub, prefix / name)

    walk(filecmp.dircmp(left, right, ignore=[]), Path())
    return sorted(found)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--archive", type=Path, help="use a local .crate archive for offline setup")
    parser.add_argument("--check", action="store_true", help="fail if out/ differs from a fresh regeneration")
    args = parser.parse_args()
    data = load_archive(args.archive)

    if args.check:
        with tempfile.TemporaryDirectory(prefix=".slatedb-check-") as temp:
            source = build(data, Path(temp))
            changed = differences(source, DESTINATION)
        if changed:
            for path in changed:
                print(f"differs: {path}", file=sys.stderr)
            sys.exit("vendor/slatedb/out does not match its patch series; edit a patch and run `just slatedb-regenerate`")
        print(f"vendor/slatedb/out matches slatedb {VERSION} plus {len(PATCHES)} patches")
        return

    # Prepare and patch in isolation. A failed download or patch leaves the existing
    # source untouched; never edit Cargo's shared registry cache.
    with tempfile.TemporaryDirectory(prefix=".slatedb-", dir=DIRECTORY) as temp:
        staging = Path(temp)
        source = build(data, staging)

        # Keep the old tree until its replacement is ready, including on rename failure.
        previous = staging / "previous"
        if DESTINATION.exists():
            DESTINATION.rename(previous)
        try:
            source.rename(DESTINATION)
        except OSError:
            if previous.exists():
                previous.rename(DESTINATION)
            raise

    print(f"Regenerated slatedb {VERSION} in {DESTINATION.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
