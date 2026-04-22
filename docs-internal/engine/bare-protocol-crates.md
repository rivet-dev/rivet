# BARE + vbare protocol crates

Conventions for RivetKit protocol crates that use BARE schemas with versioned codecs via `vbare`.

## Workspace integration

- New crates under `rivetkit-rust/packages/` that should inherit repo-wide workspace deps must set `[package] workspace = "../../../"` and be added to the root `/Cargo.toml` workspace members.

## Schema quirks

- RivetKit protocol crates with BARE `uint` fields use `vbare_compiler::Config::with_hash_map()` because `serde_bare::Uint` does not implement `Hash`.
- vbare schemas must define structs before unions reference them. Move legacy TS schemas' out-of-order definitions before adding them to Rust protocol crates.
- vbare types introduced in a later protocol version still need identity converters for skipped earlier versions so `serialize_with_embedded_version(latest)` sees the right latest version.

## TS codec generation

- Protocol crate `build.rs` TS codec generation follows `engine/packages/runner-protocol/build.rs`: run `@bare-ts/tools`, post-process to `@rivetkit/bare-ts`, and write generated imports under `rivetkit-typescript/packages/rivetkit/src/common/bare/generated/<protocol>/`.

## Usage

- RivetKit core actor/inspector BARE protocol code uses generated protocol crates plus `vbare::OwnedVersionedData`, not hand-rolled BARE cursors or writers.
- The high-level `rivetkit` crate stays a thin typed wrapper over `rivetkit-core` and re-exports shared transport/config types instead of redefining them.
- When `rivetkit` needs ergonomic helpers on a `rivetkit-core` type it re-exports, prefer an extension trait plus `prelude` re-export instead of wrapping and replacing the core type.
