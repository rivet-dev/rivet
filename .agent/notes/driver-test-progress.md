# Driver Test Suite Progress

Started: 2026-05-01
Config: registry (static), encoding (bare), runtime (native only)
Scope: DB driver tests only

## DB Tests

- [x] actor-db | Actor Database
- [x] actor-db-raw | Actor Database Raw Tests
- [x] actor-db-pragma-migration | Actor Database Pragma Migration
- [x] actor-sleep-db | Actor Sleep Database Tests
- [x] actor-db-stress | Actor Database Stress Tests
- [x] actor-db-init-order | Actor DB Init Order

## Log

- 2026-04-26T14:06:57-07:00 manager-driver: PASS
- 2026-04-26T14:07:27-07:00 actor-conn: PASS
- 2026-04-26T14:07:37-07:00 actor-conn-state: PASS
- 2026-04-26T14:07:42-07:00 conn-error-serialization: PASS
- 2026-04-26T14:08:14-07:00 actor-destroy: PASS
- 2026-04-26T14:08:19-07:00 request-access: PASS
- 2026-04-26T14:08:31-07:00 actor-handle: PASS
- 2026-04-26T14:08:31-07:00 action-features: PASS
- 2026-04-26T14:08:46-07:00 access-control: PASS
- 2026-04-26T14:08:51-07:00 actor-vars: PASS
- 2026-04-26T14:08:58-07:00 actor-metadata: PASS
- 2026-04-26T14:08:59-07:00 actor-onstatechange: PASS
- 2026-04-26T14:10:59-07:00 actor-db: FAIL (exit 124)
- 2026-04-26T14:12:00-07:00 runner: stale suite-description filters found for action-features, actor-onstatechange, actor-db, gateway-query-url, and likely other renamed suites; switching to per-file bare filter.
- 2026-04-26T14:12:54-07:00 action-features: PASS (bare file filter)
- 2026-04-26T14:12:59-07:00 actor-onstatechange: PASS (bare file filter)
- 2026-04-26T14:17:33-07:00 actor-db: FAIL (exit 1, bare file filter)
- 2026-05-01 12:45:05 PDT actor-db: FAIL - 4 failures in static/bare run. First failing test reproduced standalone: `persists across sleep and wake cycles` returned count 0 instead of 1 after sleep/wake.
- 2026-05-01 13:02:09 PDT actor-db: PASS (13 passed, 26 skipped, 25.4s). Fixed VFS persisted page-1 bootstrap, hot-only sparse page reads, and actor2 serverful reallocate transition ordering.
- 2026-05-01 13:02:30 PDT actor-db-raw: PASS (5 passed, 10 skipped, 4.5s).
- 2026-05-01 13:02:52 PDT actor-db-pragma-migration: PASS (4 passed, 8 skipped, 4.0s).
- 2026-05-01 13:10:52 PDT actor-sleep-db: PASS (14 passed, 58 skipped, 59.1s). Fixed sleep DB fixture hold behavior and made sqlite cleanup terminal for stale actor-context instances.
- 2026-05-01 13:11:48 PDT actor-db-stress: PASS (3 passed, 27.8s).
- 2026-05-01 13:12:41 PDT actor-db-init-order: PASS (6 passed, 12 skipped, 7.4s).
- 2026-05-01 13:13:02 PDT DB TESTS COMPLETE - 6/6 DB file groups passed for static/bare.
- 2026-05-01 14:22:27 PDT DB TESTS RERUN STARTED - static/bare.
- 2026-05-01 14:23:06 PDT actor-db rerun: PASS (13 passed, 26 skipped, 23.7s).
- 2026-05-01 14:23:25 PDT actor-db-raw rerun: PASS (5 passed, 10 skipped, 5.3s).
- 2026-05-01 14:24:37 PDT actor-db-pragma-migration rerun: PASS (4 passed, 8 skipped, 53.4s).
- 2026-05-01 14:25:56 PDT actor-sleep-db rerun: PASS (14 passed, 58 skipped, 64.0s).
- 2026-05-01 14:27:04 PDT actor-db-stress rerun: PASS (3 passed, 28.7s).
- 2026-05-01 14:28:00 PDT actor-db-init-order rerun: PASS (6 passed, 12 skipped, 7.9s).
- 2026-05-01 14:28:04 PDT DB TESTS RERUN COMPLETE - 6/6 DB file groups passed for static/bare.
- 2026-05-02T02:55:45-07:00 actor-conn-state: PASS (static/bare file filter with explicit pre-await onConnect subscription regression, 9 tests).
- 2026-05-03 18:13 PDT actor-sleep-db rerun [native]: PASS (26 passed, 208 skipped, 62.6s).
- 2026-05-03 18:25 PDT DB TESTS RERUN STARTED [native only].
- 2026-05-03 18:25 PDT actor-db rerun [native]: PASS (13 passed, 104 skipped, 14.0s).
- 2026-05-03 18:26 PDT actor-db-raw rerun [native]: PASS (5 passed, 40 skipped, 6.7s).
- 2026-05-03 18:27 PDT actor-db-pragma-migration rerun [native]: PASS (4 passed, 32 skipped, 4.4s).
- 2026-05-03 18:28 PDT actor-db-stress rerun [native]: PASS (5 passed, 40 skipped, 25.2s).
- 2026-05-03 18:29 PDT actor-db-init-order rerun [native]: PASS (6 passed, 48 skipped, 6.6s).
- 2026-05-03 18:29 PDT DB TESTS RERUN COMPLETE [native only] - 6/6 DB file groups passed.
