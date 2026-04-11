# @rivetkit/engine-cli

Platform-specific rivet-engine binary distribution. Shipped as a set of
`@rivetkit/engine-cli-<platform>` packages. The meta package at
`@rivetkit/engine-cli` exposes `getEnginePath()` which returns the absolute
path to the binary for the current host.

## Supported platforms

- `linux-x64-musl` — Linux x86_64 (static, runs on any distro)
- `linux-arm64-musl` — Linux aarch64 (static)
- `darwin-x64` — macOS Intel
- `darwin-arm64` — macOS Apple Silicon

Windows is not currently published via this package — engine Windows builds
are handled separately via the release workflow.

## Usage

```js
const { getEnginePath } = require("@rivetkit/engine-cli");
const { spawn } = require("node:child_process");

const child = spawn(getEnginePath(), ["start"]);
```
