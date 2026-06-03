SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| small-rowid-point | canary | 0.25 MiB | 36.0 | 4 | 67 | 13 | 4 | 50 | 166 |
| small-schema-read | canary | 0.25 MiB | 5.1 | 1 | 1 | 0 | 1 | 128 | 166 |
| small-range-scan | canary | 0.25 MiB | 60.0 | 6 | 152 | 124 | 6 | 128 | 166 |
| rowid-range-forward | read | 2.00 MiB | 334.6 | 15 | 1223 | 1017 | 15 | 1024 | 1231 |
| rowid-range-backward | read | 2.00 MiB | 356.4 | 18 | 1230 | 1014 | 18 | 1024 | 1231 |
| secondary-index-covering-range | read | 2.00 MiB | 17.9 | 8 | 8 | 0 | 8 | 1024 | 1063 |
| secondary-index-scattered-table | read | 2.00 MiB | 2003.8 | 539 | 1935 | 496 | 539 | 1024 | 1063 |
| aggregate-status | read | 2.00 MiB | 584.5 | 24 | 2132 | 1010 | 24 | 1024 | 1231 |
| aggregate-time-bucket | read | 2.00 MiB | 331.5 | 10 | 1216 | 1017 | 10 | 1024 | 1231 |
| aggregate-tenant-time-range | read | 1.00 MiB | 101.5 | 32 | 127 | 4 | 32 | 16 | 623 |
| feed-order-by-limit | read | 1.00 MiB | 183.1 | 9 | 621 | 507 | 9 | 512 | 623 |
| feed-pagination-adjacent | read | 1.00 MiB | 49.5 | 7 | 135 | 96 | 7 | 100 | 623 |
| join-order-items | read | 2.00 MiB | 50.8 | 29 | 35 | 0 | 29 | 2048 | 1231 |
| random-point-lookups | read | 2.00 MiB | 2616.4 | 592 | 1981 | 416 | 592 | 1000 | 1231 |
| hot-index-cold-table | read | 2.00 MiB | 29.9 | 11 | 17 | 2 | 11 | 8 | 1063 |
| ledger-without-rowid-range | read | 2.00 MiB | 232.6 | 44 | 139 | 47 | 44 | 564 | 176 |
| write-batch-after-wake | write | 1.00 MiB | 358.2 | 12 | 14 | 0 | 12 | 1000 | 1642 |
