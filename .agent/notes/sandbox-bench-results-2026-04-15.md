# Sandbox Bench Results

Date: 2026-04-15

## Environment

- Sandbox deploy target: `kitchen-sink-staging`
- Cloud Run region: `us-east4`
- Namespace: `kitchen-sink-gv34-staging-52gh`
- Public API host: `https://api.staging.rivet.dev`
- Published preview version: `0.0.0-pr.4667.33279e9`
- Deployed revision: `kitchen-sink-staging-00027-m6g`

## Smoke Run

- Filter used: `insert single x10`
- Baseline RTT: `133.7ms`
- Status: `passed`

| Benchmark | E2E | Server | Per-Op | RTT |
| --- | ---: | ---: | ---: | ---: |
| Insert single x10 | 275.7ms | 120.9ms | 12.1ms | 154.8ms |
| Insert single x100 | 1449.4ms | 1319.5ms | 13.2ms | 129.9ms |
| Insert single x1000 | 11728.7ms | 11588.1ms | 11.6ms | 140.6ms |
| Insert single x10000 | 120443.7ms | 120299.0ms | 12.0ms | 144.7ms |

## Full Run

- Baseline RTT: `139.7ms`
- Status: `interrupted before completion`
- Note: These are only the results captured before the run was stopped.

### Latency

| Benchmark | E2E |
| --- | ---: |
| HTTP ping (health endpoint) | 185.1ms |
| Action ping (warm actor) | 133.6ms |
| Cold start (fresh actor) | 847.9ms |
| Wake from sleep | 340.7ms |

### SQLite

| Benchmark | E2E |
| --- | ---: |
| Insert single x10 | 231.6ms |
| Insert single x100 | 1334.4ms |
| Insert single x1000 | 12781.8ms |
| Insert single x10000 | 118590.1ms |
| Insert TX x1 | 135.9ms |
| Insert TX x10 | 135.3ms |
| Insert TX x10000 | 7470.1ms |
| Insert batch x10 | 126.4ms |
| Point read x100 | 176.0ms |
| Full scan (500 rows) | 408.6ms |
| Range scan indexed | 392.0ms |
| Range scan unindexed | 380.1ms |
| Bulk update | 231.6ms |
| Bulk delete | 284.9ms |
| Hot row updates x100 | 1225.9ms |
| Hot row updates x10000 | 123365.9ms |
| VACUUM after delete | 436.7ms |
| Large payload insert (32KB x20) | 419.6ms |
| Mixed OLTP x1 | 145.1ms |
| JSON extract query | 734.1ms |
| JSON each aggregation | 156.1ms |
| Complex: aggregation | 212.6ms |
| Complex: subquery | 224.6ms |
| Complex: join (200 rows) | 348.5ms |
| Complex: CTE + window functions | 225.2ms |
| Migration (50 tables) | 179.5ms |
| Concurrent 5 actors wall time | 1476.2ms |
| Concurrent 5 actors (per-actor) | 1281.9ms |

### Chat Log Inserts

| Benchmark | E2E |
| --- | ---: |
| Insert chat log (500 KB) | 2398.7ms |
| Insert chat log (1 MB) | 4011.8ms |
| Insert chat log (5 MB) | 13284.3ms |
| Insert chat log (10 MB) | 26199.8ms |
| Insert chat log (100 MB) | 260277.8ms |

### Chat Log Reads Captured Before Interruption

| Benchmark | E2E |
| --- | ---: |
| Select with limit (500 KB) | 3131.7ms |
| Select after index (500 KB) | 2081.3ms |
| Count (500 KB) | 2153.4ms |
| Sum (500 KB) | 2002.5ms |
| Select with limit (1 MB) | 5853.9ms |

## Notes

- The health endpoint worked on staging for a throwaway actor created during verification.
- The health endpoint timed out for the prod actor ID `pevc30aj99d4kjah5peqo19ytnn610` when tested with a 15 second timeout.
