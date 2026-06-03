SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | routed_reads | write_fallbacks | mode_transitions | reader_opens | reader_closes | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| chat-log-select-limit | read | 2.00 MiB | 241.9 | 0 | 0 | 0 | 0 | 0 | 11 | 612 | 1081 | 11 | 100 | 598 |
| chat-log-select-indexed | read | 2.00 MiB | 51.0 | 0 | 0 | 0 | 0 | 0 | 7 | 135 | 112 | 7 | 100 | 598 |
| chat-log-count | read | 2.00 MiB | 7.9 | 0 | 0 | 0 | 0 | 0 | 4 | 4 | 3 | 4 | 512 | 598 |
| chat-log-sum | read | 2.00 MiB | 274.6 | 0 | 0 | 0 | 0 | 0 | 42 | 609 | 542 | 42 | 1 | 598 |
| chat-tool-read-fanout | read | 2.00 MiB | 219.6 | 0 | 0 | 0 | 0 | 0 | 12 | 613 | 1084 | 12 | 512 | 598 |
