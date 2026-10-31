#!/usr/bin/env python3
"""Regenerate committed object_store source after changing its release or patch."""

import argparse
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import urllib.request

VERSION = "0.14.2"
SHA256 = "f1796bc93603f78c5760a69f2d58badc9618d22adade0a95385bb2adbae4eb94"
DIRECTORY = Path(__file__).resolve().parent
ROOT = DIRECTORY.parents[1]
PATCH = DIRECTORY / "patches" / f"object_store-{VERSION}-conditional-delete.patch"
DESTINATION = DIRECTORY / "out"
URL = f"https://static.crates.io/crates/object_store/object_store-{VERSION}.crate"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="use a local .crate archive for offline setup")
    args = parser.parse_args()

    patch = PATCH.read_bytes()
    if args.archive:
        data = args.archive.read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError(f"checksum mismatch for object_store {VERSION}")

    # Prepare and patch in isolation. A failed download or patch leaves the existing
    # source untouched; never edit Cargo's shared registry cache.
    DESTINATION.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".object_store-", dir=DESTINATION.parent) as temp:
        staging = Path(temp)
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            archive.extractall(staging, filter="data")
        source = staging / f"object_store-{VERSION}"
        subprocess.run(
            ["patch", "--batch", "--forward", "--fuzz=0", "-p1"],
            input=patch,
            cwd=source,
            check=True,
        )
        # This dependency uses the workspace lockfile.
        (source / "Cargo.lock").unlink(missing_ok=True)

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

    print(f"Regenerated object_store {VERSION} in {DESTINATION.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
