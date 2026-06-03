SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| small-rowid-point | canary | 0.25 MiB | 255.3 | 20 | 244 | 0 | 20 | 50 | 166 |
| small-schema-read | canary | 0.25 MiB | 5.3 | 4 | 4 | 0 | 4 | 128 | 166 |
| small-range-scan | canary | 0.25 MiB | 880.8 | 133 | 2051 | 0 | 133 | 128 | 166 |
| rowid-range-forward | read | 2.00 MiB | 10817.3 | 1035 | 17179 | 0 | 1035 | 1024 | 1231 |
| rowid-range-backward | read | 2.00 MiB | 4523.6 | 1035 | 1035 | 0 | 1035 | 1024 | 1231 |
| secondary-index-covering-range | read | 2.00 MiB | 21.3 | 11 | 11 | 0 | 11 | 1024 | 1063 |
| secondary-index-scattered-table | read | 2.00 MiB | 7778.9 | 1038 | 3241 | 0 | 1038 | 1024 | 1063 |
| aggregate-status | read | 2.00 MiB | 14175.7 | 1037 | 16733 | 0 | 1037 | 1024 | 1231 |
| aggregate-time-bucket | read | 2.00 MiB | 10989.9 | 1030 | 17172 | 0 | 1030 | 1024 | 1231 |
| aggregate-tenant-time-range | read | 1.00 MiB | 226.4 | 39 | 111 | 0 | 39 | 16 | 623 |
| feed-order-by-limit | read | 1.00 MiB | 2656.3 | 519 | 519 | 0 | 519 | 512 | 623 |
| feed-pagination-adjacent | read | 1.00 MiB | 263.4 | 106 | 106 | 0 | 106 | 100 | 623 |
| join-order-items | read | 2.00 MiB | 80.9 | 32 | 38 | 0 | 32 | 2048 | 1231 |
| random-point-lookups | read | 2.00 MiB | 7718.5 | 1011 | 3304 | 0 | 1011 | 1000 | 1231 |
| hot-index-cold-table | read | 2.00 MiB | 41.3 | 16 | 26 | 0 | 16 | 8 | 1063 |
| ledger-without-rowid-range | read | 2.00 MiB | 458.6 | 94 | 176 | 0 | 94 | 564 | 176 |
| write-batch-after-wake | write | 1.00 MiB | 402.4 | 15 | 17 | 0 | 15 | 1000 | 1642 |
| update-hot-partition | write | 1.00 MiB | 6902.0 | 516 | 8498 | 0 | 516 | 64 | 623 |
| delete-churn-range-read | write | 1.00 MiB | 7040.2 | 526 | 8398 | 0 | 526 | 448 | 623 |
| migration-create-indexes-large | migration | 2.00 MiB | 14959.5 | 1030 | 17230 | 0 | 1030 |  | 1059 |
| migration-create-indexes-skewed-large | migration | 2.00 MiB | 32741.9 | 2054 | 34577 | 0 | 2054 |  | 1055 |
| migration-table-rebuild-large | migration | 2.00 MiB | 30746.7 | 2057 | 34767 | 0 | 2057 |  | 2070 |
| migration-add-column-large | migration | 2.00 MiB | 4.2 | 3 | 3 | 0 | 3 |  | 1044 |
| migration-ddl-small | canary | 0.00 MiB | 22.8 | 3 | 3 | 0 | 3 |  | 19 |
