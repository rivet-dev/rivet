SQLite optimization impact matrix

Each scenario runs in a fresh process so process-wide SQLite optimization flags are read once per configuration.

| scenario | workload | server_ms | delta_vs_defaults | get_pages | fetched_pages | cache_hits | cache_misses | routed_reads | write_fallbacks |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| defaults | small-schema-read | 6.2 | 0.0% | 1 | 1 | 3 | 1 | 0 | 0 |
| defaults | rowid-range-forward | 335.0 | 0.0% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| defaults | rowid-range-backward | 347.1 | 0.0% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| defaults | secondary-index-scattered-table | 1980.9 | 0.0% | 551 | 1723 | 487 | 551 | 0 | 0 |
| defaults | aggregate-status | 589.6 | 0.0% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| defaults | parallel-read-aggregates | 616.9 | 0.0% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| defaults | parallel-read-write-transition | 379.5 | 0.0% | 24 | 998 | 508 | 24 | 0 | 0 |
| defaults | random-point-lookups | 3261.5 | 0.0% | 541 | 2106 | 470 | 541 | 0 | 0 |
| defaults | write-batch-after-wake | 379.0 | 0.0% | 12 | 14 | 3 | 12 | 0 | 0 |
| defaults | migration-create-indexes-large | 383.0 | 0.0% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| vfs-cache-only | small-schema-read | 4.5 | -27.6% | 1 | 1 | 3 | 1 | 0 | 0 |
| vfs-cache-only | rowid-range-forward | 453.6 | 35.4% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| vfs-cache-only | rowid-range-backward | 354.4 | 2.1% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| vfs-cache-only | secondary-index-scattered-table | 2351.4 | 18.7% | 535 | 1773 | 503 | 535 | 0 | 0 |
| vfs-cache-only | aggregate-status | 640.7 | 8.7% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| vfs-cache-only | parallel-read-aggregates | 738.0 | 19.6% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| vfs-cache-only | parallel-read-write-transition | 405.3 | 6.8% | 24 | 998 | 508 | 24 | 0 | 0 |
| vfs-cache-only | random-point-lookups | 3278.2 | 0.5% | 555 | 2005 | 456 | 555 | 0 | 0 |
| vfs-cache-only | write-batch-after-wake | 394.7 | 4.1% | 12 | 14 | 3 | 12 | 0 | 0 |
| vfs-cache-only | migration-create-indexes-large | 352.8 | -7.9% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| cache-read-ahead-no-preload | small-schema-read | 2.8 | -54.2% | 1 | 1 | 3 | 1 | 0 | 0 |
| cache-read-ahead-no-preload | rowid-range-forward | 412.2 | 23.0% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| cache-read-ahead-no-preload | rowid-range-backward | 340.5 | -1.9% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| cache-read-ahead-no-preload | secondary-index-scattered-table | 2612.3 | 31.9% | 561 | 1721 | 477 | 561 | 0 | 0 |
| cache-read-ahead-no-preload | aggregate-status | 590.4 | 0.1% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| cache-read-ahead-no-preload | parallel-read-aggregates | 596.8 | -3.3% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| cache-read-ahead-no-preload | parallel-read-write-transition | 355.9 | -6.2% | 24 | 998 | 508 | 24 | 0 | 0 |
| cache-read-ahead-no-preload | random-point-lookups | 2733.2 | -16.2% | 590 | 2125 | 421 | 590 | 0 | 0 |
| cache-read-ahead-no-preload | write-batch-after-wake | 381.2 | 0.6% | 12 | 14 | 3 | 12 | 0 | 0 |
| cache-read-ahead-no-preload | migration-create-indexes-large | 324.2 | -15.3% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| no-read-ahead | small-schema-read | 3.9 | -36.6% | 1 | 1 | 3 | 1 | 0 | 0 |
| no-read-ahead | rowid-range-forward | 392.0 | 17.0% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| no-read-ahead | rowid-range-backward | 343.7 | -1.0% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| no-read-ahead | secondary-index-scattered-table | 1895.2 | -4.3% | 578 | 1739 | 460 | 578 | 0 | 0 |
| no-read-ahead | aggregate-status | 646.8 | 9.7% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| no-read-ahead | parallel-read-aggregates | 631.3 | 2.3% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| no-read-ahead | parallel-read-write-transition | 354.7 | -6.5% | 24 | 998 | 508 | 24 | 0 | 0 |
| no-read-ahead | random-point-lookups | 2732.0 | -16.2% | 570 | 2100 | 441 | 570 | 0 | 0 |
| no-read-ahead | write-batch-after-wake | 375.9 | -0.8% | 12 | 14 | 3 | 12 | 0 | 0 |
| no-read-ahead | migration-create-indexes-large | 324.2 | -15.3% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| no-preload | small-schema-read | 4.0 | -35.8% | 1 | 1 | 3 | 1 | 0 | 0 |
| no-preload | rowid-range-forward | 352.4 | 5.2% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| no-preload | rowid-range-backward | 354.6 | 2.1% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| no-preload | secondary-index-scattered-table | 1929.7 | -2.6% | 575 | 1718 | 463 | 575 | 0 | 0 |
| no-preload | aggregate-status | 605.4 | 2.7% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| no-preload | parallel-read-aggregates | 604.1 | -2.1% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| no-preload | parallel-read-write-transition | 338.1 | -10.9% | 24 | 998 | 508 | 24 | 0 | 0 |
| no-preload | random-point-lookups | 2743.2 | -15.9% | 579 | 2099 | 432 | 579 | 0 | 0 |
| no-preload | write-batch-after-wake | 390.5 | 3.0% | 12 | 14 | 3 | 12 | 0 | 0 |
| no-preload | migration-create-indexes-large | 333.1 | -13.0% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| no-range-reads | small-schema-read | 3.2 | -47.7% | 1 | 1 | 3 | 1 | 0 | 0 |
| no-range-reads | rowid-range-forward | 340.9 | 1.8% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| no-range-reads | rowid-range-backward | 353.6 | 1.9% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| no-range-reads | secondary-index-scattered-table | 2433.1 | 22.8% | 554 | 1691 | 484 | 554 | 0 | 0 |
| no-range-reads | aggregate-status | 592.7 | 0.5% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| no-range-reads | parallel-read-aggregates | 663.6 | 7.6% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| no-range-reads | parallel-read-write-transition | 350.2 | -7.7% | 24 | 998 | 508 | 24 | 0 | 0 |
| no-range-reads | random-point-lookups | 2417.4 | -25.9% | 568 | 2063 | 443 | 568 | 0 | 0 |
| no-range-reads | write-batch-after-wake | 414.0 | 9.2% | 12 | 14 | 3 | 12 | 0 | 0 |
| no-range-reads | migration-create-indexes-large | 325.1 | -15.1% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| no-storage-read-cache | small-schema-read | 2.7 | -56.2% | 1 | 1 | 3 | 1 | 0 | 0 |
| no-storage-read-cache | rowid-range-forward | 359.8 | 7.4% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| no-storage-read-cache | rowid-range-backward | 358.3 | 3.2% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| no-storage-read-cache | secondary-index-scattered-table | 2565.9 | 29.5% | 575 | 1835 | 463 | 575 | 0 | 0 |
| no-storage-read-cache | aggregate-status | 648.9 | 10.1% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| no-storage-read-cache | parallel-read-aggregates | 700.7 | 13.6% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| no-storage-read-cache | parallel-read-write-transition | 399.9 | 5.4% | 24 | 998 | 508 | 24 | 0 | 0 |
| no-storage-read-cache | random-point-lookups | 2948.3 | -9.6% | 560 | 2061 | 451 | 560 | 0 | 0 |
| no-storage-read-cache | write-batch-after-wake | 361.0 | -4.7% | 12 | 14 | 3 | 12 | 0 | 0 |
| no-storage-read-cache | migration-create-indexes-large | 329.2 | -14.0% | 9 | 1029 | 3069 | 9 | 0 | 0 |
| no-read-pool | small-schema-read | 5.8 | -5.1% | 1 | 1 | 3 | 1 | 0 | 0 |
| no-read-pool | rowid-range-forward | 406.6 | 21.4% | 15 | 1223 | 1020 | 15 | 0 | 0 |
| no-read-pool | rowid-range-backward | 360.3 | 3.8% | 18 | 1230 | 1017 | 18 | 0 | 0 |
| no-read-pool | secondary-index-scattered-table | 2355.5 | 18.9% | 533 | 1849 | 505 | 533 | 0 | 0 |
| no-read-pool | aggregate-status | 600.5 | 1.9% | 24 | 2132 | 1013 | 24 | 0 | 0 |
| no-read-pool | parallel-read-aggregates | 594.0 | -3.7% | 28 | 2136 | 2019 | 28 | 0 | 0 |
| no-read-pool | parallel-read-write-transition | 357.7 | -5.7% | 24 | 998 | 508 | 24 | 0 | 0 |
| no-read-pool | random-point-lookups | 3520.8 | 7.9% | 575 | 2068 | 436 | 575 | 0 | 0 |
| no-read-pool | write-batch-after-wake | 450.6 | 18.9% | 12 | 14 | 3 | 12 | 0 | 0 |
| no-read-pool | migration-create-indexes-large | 319.6 | -16.5% | 9 | 1029 | 3069 | 9 | 0 | 0 |

Failed scenarios

- all-off: all-off matrix scenario failed with exit code 1
- transport-batching-only: transport-batching-only matrix scenario failed with exit code 1
- read-ahead-no-cache: read-ahead-no-cache matrix scenario failed with exit code 1
- no-vfs-cache: no-vfs-cache matrix scenario failed with exit code 1
