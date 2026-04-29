# Build troubleshooting

Known foot-guns when building RivetKit packages.

## `registry.start()` fails with missing `@rivetkit/engine-cli-*`

- In monorepo development, `registry.start()` may start the local engine. If the optional `@rivetkit/engine-cli-*` platform package is missing, run `cargo build -p rivet-engine` and set `RIVET_ENGINE_BINARY=/home/nathan/r5/target/debug/rivet-engine`.

## DTS / type build fails with missing `@rivetkit/*`

- If `rivetkit` type or DTS builds fail with missing `@rivetkit/*` declarations, run `pnpm build -F rivetkit` from repo root (Turbo build path) **before** changing TypeScript `paths`.
- Do not add temporary `@rivetkit/*` path aliases in `rivetkit-typescript/packages/rivetkit/tsconfig.json` to work around stale or missing built declarations.

## NAPI not picking up `rivetkit-core` changes

- After native `rivetkit-core` changes, use `pnpm --filter @rivetkit/rivetkit-napi build:force` before TS driver tests because the normal N-API build skips when a prebuilt `.node` exists.

## `JsActorConfig` field churn

- When removing `rivetkit-napi` `JsActorConfig` fields, keep `impl From<JsActorConfig> for FlatActorConfig` explicit and set any wider core-only fields to `None` instead of dropping them from the struct literal.

## tsup passes but runtime imports fail

- When trimming `rivetkit` entrypoints, update `package.json` `exports`, `files`, and `scripts.build` together. `tsup` can still pass while stale exports point at missing dist files.
