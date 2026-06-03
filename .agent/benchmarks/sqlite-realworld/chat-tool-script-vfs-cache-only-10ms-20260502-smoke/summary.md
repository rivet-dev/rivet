SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | routed_reads | write_fallbacks | mode_transitions | reader_opens | reader_closes | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| chat-tool-script | read | 2.00 MiB | 166.7 | 0 | 0 | 0 | 0 | 0 | 17 | 487 | 398 | 17 | 7 | 475 |
