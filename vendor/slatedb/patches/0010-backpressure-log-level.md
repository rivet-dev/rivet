# 0010: backpressure logs at debug, with a timeout counter

**What it changes.** The `applying backpressure` and `backpressure timeout` messages in `DbInner::maybe_apply_backpressure` move from warn to debug. Adds `db_stats::BACKPRESSURE_TIMEOUT_COUNT`, incremented each time a wait hits its 30 s timeout; `BACKPRESSURE_COUNT` already counts each check that made a write wait.

**Why.** Every waiting write logs on every pass of the loop, so a slow object store produced about 25,000 warn lines in 16 minutes. Depot exports both counters as `depot_cold_store_backpressure_total{result="wait"|"timeout"}`, so the signal stays in metrics.

**Upstream.** Candidate for upstream: a rate-limited warn or a lower level.

**Drop when.** Upstream lowers or rate-limits these logs.
