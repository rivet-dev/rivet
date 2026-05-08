# VBARE schema migrations

Procedural guide for adding a new schema version to a vbare-backed protocol crate (envoy-protocol, runner-protocol, epoxy-protocol, depot-protocol, ups-protocol).

## Layout

Every protocol crate has:

- `engine/sdks/schemas/<protocol>/v{N}.bare` — versioned schema files. **Never edit a published one in place.**
- `engine/sdks/rust/<protocol>/src/generated.rs` — `include!`s vbare-generated `vN_generated.rs` into `pub mod vN`.
- `engine/sdks/rust/<protocol>/src/versioned/` — directory with one `mod.rs` and one `vN_to_vM.rs` file per adjacent migration step.
- `engine/sdks/rust/<protocol>/src/lib.rs` — re-exports `generated::v{LATEST}::*` and `PROTOCOL_VERSION`.

`mod.rs` owns the multi-variant wrapper enums and `OwnedVersionedData` impls. The `vN_to_vM.rs` files own the field-by-field per-type converters.

## Adding a new schema version

1. **Copy the latest schema** to a new file: `cp engine/sdks/schemas/<protocol>/vN.bare engine/sdks/schemas/<protocol>/v{N+1}.bare`. Edit the new file.
2. **Bump the protocol constant** alongside the schema. For most crates that's `PROTOCOL_VERSION` (or `PROTOCOL_MK2_VERSION` for runner-protocol). For runner-protocol also bump `rivetkit-typescript/packages/engine-runner/src/mod.ts` `PROTOCOL_VERSION`.
3. **Update `lib.rs`** to re-export the new `generated::v{N+1}::*`.
4. **Run the converter scaffolder** for the new pair:
   ```
   tsx scripts/vbare-gen-converters/index.ts \
     engine/sdks/schemas/<protocol>/vN.bare \
     engine/sdks/schemas/<protocol>/v{N+1}.bare \
     engine/sdks/rust/<protocol>/src/versioned \
     --types Root1,Root2,...
   ```
   `--types` is a comma-separated list of the wrapper-enum names (e.g. `ToEnvoy,ToRivet,ToEnvoyConn,ToGateway,ToOutbound,ActorCommandKeyData` for envoy-protocol). The script transitively pulls in every type those wrappers reference; types that are byte-identical primitive aliases are skipped (no converter needed).
   This writes two files: `vN_to_v{N+1}.rs` and `v{N+1}_to_vN.rs`.
5. **Fill in every `todo!()`.** The script puts a `todo!()` wherever a struct field, union variant, or enum value exists on one side but not the other. For each one, decide:
   - **Field added** (forward direction): default value, e.g. `None` for optional, the moral equivalent for non-optional.
   - **Field dropped** (backward direction): nothing to fill — the field doesn't appear in the target struct.
   - **Variant added**: forward direction unreachable from older payloads. Backward direction must `bail!(...)` with a descriptive message, or — if the feature is part of a tracked compatibility surface — return a structured error like `incompatible(ProtocolCompatibilityFeature::Foo, ...)`.
   - **Variant whose shape changed too much to map cleanly**: bail at the union arm with a `bail!(...)` describing the incompatibility. Inner per-type converters for that variant become unreachable; their auto-generated `todo!()`s in field positions are dead and can stay.
6. **Update `mod.rs`**:
   - Add a `Vn` variant to each wrapper enum.
   - Add a `mod vN_to_v{N+1};` and `mod v{N+1}_to_vN;` declaration.
   - Add the new version to every match arm in `deserialize_version` and `serialize_version`.
   - Update `type Latest = vN::T;`, `wrap_latest`, and `unwrap_latest` to point at the new latest.
   - Append the new step methods (`vN_to_v{N+1}`, `v{N+1}_to_vN`) to the `impl T` block.
   - Update `deserialize_converters` and `serialize_converters`.
7. **Run the crate's tests**: `cargo test -p <crate-name>`. Add a round-trip test that exercises the new step (round-trip from latest down to vN-1 and back).

## Converter chain ordering (mod.rs)

The two converter vectors look identical-shaped but walk in **opposite directions**, and getting them backwards silently breaks `serialize`:

- `deserialize_converters` is **bottom-up**: index `i` upgrades V{i+1} to V{i+2}. For five versions: `[v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5]`.
- `serialize_converters` is **top-down**: index `0` downgrades the latest variant one step. For five versions: `[v5_to_v4, v4_to_v3, v3_to_v2, v2_to_v1]`.

The vbare runtime takes a prefix of `serialize_converters` based on the requested target version (`take((latest + 1) - target)`), so a reversed list will run downgrade steps in the wrong order and either panic on `Self::Vn(_) => bail!("unexpected version")` or produce a wrongly-typed wire payload.

## Why each converter is `Result<T>`

Every generated converter returns `Result<T>` even when the body is infallible (`Ok(...)` of a field-by-field copy). Reasons:

- Cross-version migrations frequently need to `bail!` at union arms when downgrading. Forcing every converter to be `Result` means a downgrade can drop in a `bail!` without changing the function signature or rippling `?`s through every caller.
- Iteration patterns (`.collect::<Result<Vec<_>>>()?`, `.transpose()?`) compose uniformly when every leaf is fallible. Mixed fallible/infallible signatures fight type inference inside closures.
- The compiler optimises `Ok(x)?` away. There is no runtime cost.

## Avoid

- **Editing a published `*.bare`** in place. Always add a new versioned file.
- **`serde_bare::to_vec` + `from_slice`** to "convert" between adjacent versions. Even if the bytes are identical today, schema drift breaks it silently. Always reconstruct field-by-field. The script enforces this by emitting field assignments rather than a re-serialize round-trip.
- **`vec![Ok, Ok, ...]` converter chains**. If the wrapper enum has multiple version variants but the converters are no-ops, the chain isn't actually walking — `unwrap_latest()` will fail with "version not latest" because no upgrade ran.
- **Single-variant wrapper enums** (`enum ToEnvoy { V5(v5::ToEnvoy) }`). The whole point of `OwnedVersionedData` is that each version inhabits its own variant and the chain walks them. A single variant collapses the chain into manual nested calls inside `deserialize_version` / `serialize_version`, which is the exact anti-pattern this layout replaces. See `engine/sdks/rust/envoy-protocol/src/versioned/mod.rs` and `engine/sdks/rust/epoxy-protocol/src/versioned.rs` for correct shapes.

## The converter script

Lives at `scripts/vbare-gen-converters/`:

- `index.ts` — entry point. Parses BARE via `@bare-ts/tools`, walks the type graph, emits Rust.
- `templates/*.hbs` — Handlebars templates for the per-type Rust output (`struct.hbs`, `union.hbs`, `enum.hbs`, `wrapper-alias.hbs`, `todo.hbs`, `file.hbs`).

CLI:

```
tsx scripts/vbare-gen-converters/index.ts <from.bare> <to.bare> <out_dir> \
  [--types T1,T2,...] [--from-ns vN] [--to-ns vM]
```

What it does NOT generate:

- **`mod.rs`** — the wrapper enums and `OwnedVersionedData` impls are hand-written (boilerplate that's small per crate and rarely touched once correct).
- **Protocol-specific compatibility errors** like `ProtocolCompatibilityError` in envoy-protocol's `mod.rs`. Add those by hand and reference them from the converter file's bail sites.

What it does:

- One `Result<T>`-returning function per shared user-defined type, named `convert_<snake>_<from_ns>_to_<to_ns>`.
- Skips top-level converters for primitive aliases (`type Id str`) and `void` types — both are identity in Rust so a wrapping function would only add noise.
- Uses `todo!()` for any field, union variant, or enum value that isn't structurally identical on both sides.
- Inner expressions for nested aliases use `?` to propagate; `Option<T>` uses `.map(...).transpose()?`; `Vec<T>` uses `.collect::<Result<Vec<_>>>()?`; maps wrap their iter map closure with an explicit `-> Result<_>` return.

Re-running the script overwrites the existing files. **Do not re-run the script after filling in todos** unless you are prepared to redo every manual edit. The intended flow is: scaffold once, fill in once, leave the file alone forever (per the vbare design tenet that migration code must never be retroactively changed).
