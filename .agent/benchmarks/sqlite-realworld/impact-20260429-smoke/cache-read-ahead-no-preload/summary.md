SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | routed_reads | write_fallbacks | mode_transitions | reader_opens | reader_closes | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| small-schema-read | canary | 0.25 MiB | 2.8 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 3 | 1 | 128 | 166 |
| rowid-range-forward | read | 2.00 MiB | 412.2 | 0 | 0 | 0 | 0 | 0 | 15 | 1223 | 1020 | 15 | 1024 | 1231 |
| rowid-range-backward | read | 2.00 MiB | 340.5 | 0 | 0 | 0 | 0 | 0 | 18 | 1230 | 1017 | 18 | 1024 | 1231 |
| secondary-index-scattered-table | read | 2.00 MiB | 2612.3 | 0 | 0 | 0 | 0 | 0 | 561 | 1721 | 477 | 561 | 1024 | 1063 |
| aggregate-status | read | 2.00 MiB | 590.4 | 0 | 0 | 0 | 0 | 0 | 24 | 2132 | 1013 | 24 | 1024 | 1231 |
| parallel-read-aggregates | read | 2.00 MiB | 596.8 | 0 | 0 | 0 | 0 | 0 | 28 | 2136 | 2019 | 28 | 4112 | 1231 |
| parallel-read-write-transition | write | 1.00 MiB | 355.9 | 0 | 0 | 0 | 0 | 0 | 24 | 998 | 508 | 24 | 1568 | 623 |
| random-point-lookups | read | 2.00 MiB | 2733.2 | 0 | 0 | 0 | 0 | 0 | 590 | 2125 | 421 | 590 | 1000 | 1231 |
| write-batch-after-wake | write | 1.00 MiB | 381.2 | 0 | 0 | 0 | 0 | 0 | 12 | 14 | 3 | 12 | 1000 | 1642 |
| migration-create-indexes-large | migration | 2.00 MiB | 324.2 | 0 | 0 | 0 | 0 | 0 | 9 | 1029 | 3069 | 9 |  | 1059 |
