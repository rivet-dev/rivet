# SQLite Optimization Impact Matrix Report

Run: `impact` matrix, `smoke` profile, 10 representative workloads. Server-side SQLite time only. Each scenario ran in a fresh process.

## Important caveats

- This is a smoke-size run, so treat small percent moves as noise. The useful signal here is large regressions, failed configurations, and workload-specific direction.
- Read-pool metrics were zero in every scenario (`routedReads=0`, `modeTransitions=0`), so this run does not yet prove read-pool benefit. It mostly validates the existing VFS/storage flags.
- VFS-cache-off scenarios failed before benchmarks could run, which is a correctness finding, not just a performance result.

## Scenario Summary

| scenario | avg delta vs defaults | median delta | >10% regressions | >10% wins |
| --- | ---: | ---: | ---: | ---: |
| vfs-cache-only | +6.0% | +6.8% | 3 | 1 |
| no-read-pool | +4.2% | +3.8% | 3 | 1 |
| no-storage-read-cache | -1.5% | +5.4% | 3 | 2 |
| cache-read-ahead-no-preload | -4.1% | -1.9% | 2 | 3 |
| no-read-ahead | -5.2% | -1.0% | 1 | 3 |
| no-range-reads | -5.3% | +1.8% | 1 | 3 |
| no-preload | -6.7% | -2.1% | 0 | 4 |

## Failed Scenarios

- all-off: all-off matrix scenario failed with exit code 1
- transport-batching-only: transport-batching-only matrix scenario failed with exit code 1
- read-ahead-no-cache: read-ahead-no-cache matrix scenario failed with exit code 1
- no-vfs-cache: no-vfs-cache matrix scenario failed with exit code 1

Common failure signature: actor startup fails opening SQLite with `PRAGMA journal_mode = DELETE` returning `file is not a database`. All failed scenarios disable VFS page cache.

## Biggest Workload Moves

### vfs-cache-only

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| rowid-range-forward | +35.4% | 335.0ms | 453.6ms | 15 -> 15 |
| small-schema-read | -27.6% | 6.2ms | 4.5ms | 1 -> 1 |
| parallel-read-aggregates | +19.6% | 616.9ms | 738.0ms | 28 -> 28 |
| secondary-index-scattered-table | +18.7% | 1980.9ms | 2351.4ms | 551 -> 535 |
| aggregate-status | +8.7% | 589.6ms | 640.7ms | 24 -> 24 |

### cache-read-ahead-no-preload

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| small-schema-read | -54.2% | 6.2ms | 2.8ms | 1 -> 1 |
| secondary-index-scattered-table | +31.9% | 1980.9ms | 2612.3ms | 551 -> 561 |
| rowid-range-forward | +23.0% | 335.0ms | 412.2ms | 15 -> 15 |
| random-point-lookups | -16.2% | 3261.5ms | 2733.2ms | 541 -> 590 |
| migration-create-indexes-large | -15.3% | 383.0ms | 324.2ms | 9 -> 9 |

### no-read-ahead

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| small-schema-read | -36.6% | 6.2ms | 3.9ms | 1 -> 1 |
| rowid-range-forward | +17.0% | 335.0ms | 392.0ms | 15 -> 15 |
| random-point-lookups | -16.2% | 3261.5ms | 2732.0ms | 541 -> 570 |
| migration-create-indexes-large | -15.3% | 383.0ms | 324.2ms | 9 -> 9 |
| aggregate-status | +9.7% | 589.6ms | 646.8ms | 24 -> 24 |

### no-preload

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| small-schema-read | -35.8% | 6.2ms | 4.0ms | 1 -> 1 |
| random-point-lookups | -15.9% | 3261.5ms | 2743.2ms | 541 -> 579 |
| migration-create-indexes-large | -13.0% | 383.0ms | 333.1ms | 9 -> 9 |
| parallel-read-write-transition | -10.9% | 379.5ms | 338.1ms | 24 -> 24 |
| rowid-range-forward | +5.2% | 335.0ms | 352.4ms | 15 -> 15 |

### no-range-reads

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| small-schema-read | -47.7% | 6.2ms | 3.2ms | 1 -> 1 |
| random-point-lookups | -25.9% | 3261.5ms | 2417.4ms | 541 -> 568 |
| secondary-index-scattered-table | +22.8% | 1980.9ms | 2433.1ms | 551 -> 554 |
| migration-create-indexes-large | -15.1% | 383.0ms | 325.1ms | 9 -> 9 |
| write-batch-after-wake | +9.2% | 379.0ms | 414.0ms | 12 -> 12 |

### no-storage-read-cache

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| small-schema-read | -56.2% | 6.2ms | 2.7ms | 1 -> 1 |
| secondary-index-scattered-table | +29.5% | 1980.9ms | 2565.9ms | 551 -> 575 |
| migration-create-indexes-large | -14.0% | 383.0ms | 329.2ms | 9 -> 9 |
| parallel-read-aggregates | +13.6% | 616.9ms | 700.7ms | 28 -> 28 |
| aggregate-status | +10.1% | 589.6ms | 648.9ms | 24 -> 24 |

### no-read-pool

| workload | delta | defaults | scenario | get_pages |
| --- | ---: | ---: | ---: | ---: |
| rowid-range-forward | +21.4% | 335.0ms | 406.6ms | 15 -> 15 |
| secondary-index-scattered-table | +18.9% | 1980.9ms | 2355.5ms | 551 -> 533 |
| write-batch-after-wake | +18.9% | 379.0ms | 450.6ms | 12 -> 12 |
| migration-create-indexes-large | -16.5% | 383.0ms | 319.6ms | 9 -> 9 |
| random-point-lookups | +7.9% | 3261.5ms | 3520.8ms | 541 -> 575 |

## Initial Takeaways

1. VFS page cache cannot currently be treated as optional. Disabling it made fresh SQLite opens fail in `all-off`, `transport-batching-only`, `read-ahead-no-cache`, and `no-vfs-cache`.
2. On smoke data, read-ahead, preload, and range-read ablations did not change VFS round-trip counts for scan workloads. The scan paths still fetched 15/18/24/28 batches, so another layer is already batching these small datasets.
3. The clearest smoke regressions were scattered/random/parallel aggregate workloads when cache/storage helpers were removed, especially `secondary-index-scattered-table` and `parallel-read-aggregates`.
4. The read-pool flag does not show active routing yet. `parallel-read-aggregates` was 616.9ms by default and 594.0ms with the read pool disabled, with read-pool counters at zero. That means the current TS/native path is not exercising parallel reader routing in this benchmark run.
5. For final decisions, rerun the matrix on the standard profile or a targeted standard subset focused on `secondary-index-scattered-table`, `random-point-lookups`, `parallel-read-aggregates`, and scan workloads.
