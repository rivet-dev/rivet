# Rust Workaround

## Overview

This issue is specific to Apple Silicon machines.

Dev containers have a bug with incremental builds which lead to nonsensical Rust errors.

These are documented in better detail here:

- https://stackoverflow.com/questions/72448053/rust-incremental-build-not-working-in-vscode-devcontainer
- https://github.com/docker/for-mac/issues/7059

## Moving target dir out of mounted dir

In order to fix this, we move the target path to `/target` which is not mounted to the host system. This way
the virtual fs does not trigger issues with Rust.

This complicates things, since we now compile binaries in non-standard paths.

This is done by updating `devcontainer.json` to:

- Overriding `CARGO_TARGET_DIR` to `/target`
- Updating `PATH` to include `/target/debug` and `/target/release` so we can still access `bolt`
