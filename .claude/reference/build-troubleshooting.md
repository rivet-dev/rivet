# Build troubleshooting

Known foot-guns when building RivetKit packages.

## DTS / type build fails with missing `@rivetkit/*`

- If `rivetkit` type or DTS builds fail with missing `@rivetkit/*` declarations, run `pnpm build -F rivetkit` from repo root (Turbo build path) **before** changing TypeScript `paths`.
- Do not add temporary `@rivetkit/*` path aliases in `rivetkit-typescript/packages/rivetkit/tsconfig.json` to work around stale or missing built declarations.

## NAPI not picking up `rivetkit-core` changes

- After native `rivetkit-core` changes, use `pnpm --filter @rivetkit/rivetkit-napi build:force` before TS driver tests because the normal N-API build skips when a prebuilt `.node` exists.

## `JsActorConfig` field churn

- When removing `rivetkit-napi` `JsActorConfig` fields, keep `impl From<JsActorConfig> for FlatActorConfig` explicit and set any wider core-only fields to `None` instead of dropping them from the struct literal.

## tsup passes but runtime imports fail

- When trimming `rivetkit` entrypoints, update `package.json` `exports`, `files`, and `scripts.build` together. `tsup` can still pass while stale exports point at missing dist files.

## Inspector UI embed: empty bundle / `ui_asset_not_found` 404

- `rivetkit-core`'s `build.rs` embeds `frontend/dist/inspector-ui` (+ `dist/inspector-tab`) into the native binary via `include_dir!`, and that binary serves `/inspector/ui/*`. But the inspector UI is built *downstream* of `rivetkit-napi`/`rivetkit-wasm` in the package graph (`build:inspector-ui` needs `rivetkit`'s dist, which needs napi), so it can't be a direct `dependsOn` without a turbo cycle. The workaround is the **two-step build**: phase-1 `build` links the native binary with an *empty* bundle, then the scoped `@rivetkit/rivetkit-napi#build:embed` / `@rivetkit/rivetkit-wasm#build:embed` tasks (run by `pnpm -w build` = `turbo build build:embed`) re-run the native build *after* `build:inspector-ui` so the real assets get embedded.
- **The trap:** any turbo invocation that runs `@rivetkit/rivetkit-napi#build` (or the wasm one) **without** `build:embed` (a partial build, `--filter=rivetkit`, rebuilding any downstream package, IDE tooling) relinks the `.node`/wasm with the **empty** bundle and leaves it that way. Symptom: loading `/inspector/ui/*` (e.g. via the dashboard iframe, tunneled engine → runner) returns `{"group":"inspector","code":"ui_asset_not_found",...}` 404. The engine embeds the real bundle but tunnels `/gateway/{actor}/inspector/ui/` to the runner, so it's the *runner's* `.node` that must be embedded.
- **Fix:** rebuild via `pnpm -w build` (includes `build:embed`), or after any partial native build run `pnpm -F @rivetkit/rivetkit-napi build:force` (wasm: rebuild the wasm pkg) to re-embed. Verify with `grep -qa "$(ls frontend/dist/inspector-ui/assets/*.js | head -1 | xargs basename)" <the .node>`.
- **Fully removing the trap** requires breaking the cycle so napi/wasm `build` can depend on the UI directly. That means extracting the inspector protocol/client (`inspector/client`, `inspector-tab`, bare codecs) into a leaf package with no `rivetkit-napi` dependency. Until then, the two-step build's empty-then-embed window is load-bearing.
