# 0001: exact leadership epoch and open-failure cleanup

**What it changes.** Adds `DbBuilder::with_leadership_epoch(NonZeroU64)`. The writer and the embedded compactor install exactly that epoch instead of "observed plus one", so a delayed open from an expired grant fails with `Fenced` instead of fencing the current owner. A failed open now cancels and joins its tasks before returning. Adds four failpoints and `tests/leadership_lifecycle.rs` (8 tests). The full contract is in `out/LEADERSHIP.md`, which this patch adds.

**Needed by.** Every tablet open: `ProductionSlateDbFactory::open` in `engine/packages/env/src/backends/production/slatedb.rs` passes the tablet grant epoch.

**Upstream.** `with_expected_epoch` (PR #2153, merged, unreleased) claims observed plus one and does not order grants. Upstream already exposes the fixed-epoch primitive (`init_with_epoch`, PR #1152), so an exact-epoch builder option is a small upstream change.

**Drop when.** Upstream offers an exact-epoch writer and embedded-compactor open with equivalent cleanup, and the lifecycle tests pass against it.
