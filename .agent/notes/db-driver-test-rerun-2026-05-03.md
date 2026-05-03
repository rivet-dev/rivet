# DB Driver Test Rerun 2026-05-03

Config: static registry, bare encoding, native/local plus wasm/remote.

## Results

- [x] actor-db: native/local passed, wasm/remote passed
- [x] actor-db-raw: native/local passed, wasm/remote passed
- [x] actor-db-init-order: native/local passed, wasm/remote passed
- [x] actor-db-pragma-migration: native/local passed, wasm/remote passed
- [x] actor-sleep-db: native/local passed, wasm/remote passed
- [x] actor-db-stress: native/local passed, wasm/remote passed on rerun

## Notes

- Fixed a grace-deadline shutdown bug where SQLite cleanup happened after final state serialization, allowing delayed callbacks from the old generation to issue late DB work.
- Added a shared closed flag to `SqliteDb` so both local and remote SQLite handles fail closed after cleanup.
- The first full wasm/remote stress run hit `ltx trailer checksums must be zeroed` in the kitchen-sink case. The isolated kitchen-sink rerun and the full wasm/remote stress rerun both passed.
