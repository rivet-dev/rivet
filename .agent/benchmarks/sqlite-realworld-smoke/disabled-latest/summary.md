SQLite real-world benchmark

Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.

| workload | category | size | server_ms | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| rowid-range-forward | read | 2.00 MiB | 14946.6 | 1035 | 17179 | -3 | 1035 | 1024 | 1231 |
| secondary-index-scattered-table | read | 2.00 MiB | 8251.8 | 1038 | 3514 | -3 | 1038 | 1024 | 1063 |
| random-point-lookups | read | 2.00 MiB | 8725.9 | 1011 | 3208 | -3 | 1011 | 1000 | 1231 |
| migration-add-column-large | migration | 2.00 MiB | 2.7 | 3 | 3 | -3 | 3 |  | 1044 |
