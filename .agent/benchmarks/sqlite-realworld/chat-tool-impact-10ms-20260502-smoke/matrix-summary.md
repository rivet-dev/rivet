SQLite optimization impact matrix

Each scenario runs in a fresh process so process-wide SQLite optimization flags are read once per configuration.

| scenario | workload | server_ms | delta_vs_defaults | get_pages | fetched_pages | cache_hits | cache_misses | routed_reads | write_fallbacks |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| defaults | chat-log-select-limit | 176.5 | 0.0% | 11 | 612 | 1081 | 11 | 0 | 0 |
| defaults | chat-log-select-indexed | 46.5 | 0.0% | 7 | 135 | 112 | 7 | 0 | 0 |
| defaults | chat-log-count | 7.8 | 0.0% | 4 | 4 | 3 | 4 | 0 | 0 |
| defaults | chat-log-sum | 215.0 | 0.0% | 42 | 609 | 542 | 42 | 0 | 0 |
| defaults | chat-tool-read-fanout | 183.2 | 0.0% | 12 | 613 | 1084 | 12 | 0 | 0 |
| vfs-cache-only | chat-log-select-limit | 173.2 | -1.9% | 11 | 612 | 1081 | 11 | 0 | 0 |
| vfs-cache-only | chat-log-select-indexed | 46.5 | -0.1% | 7 | 135 | 112 | 7 | 0 | 0 |
| vfs-cache-only | chat-log-count | 14.7 | 88.9% | 4 | 4 | 3 | 4 | 0 | 0 |
| vfs-cache-only | chat-log-sum | 256.3 | 19.2% | 42 | 609 | 542 | 42 | 0 | 0 |
| vfs-cache-only | chat-tool-read-fanout | 187.3 | 2.2% | 12 | 613 | 1084 | 12 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-select-limit | 175.6 | -0.5% | 11 | 612 | 1081 | 11 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-select-indexed | 93.2 | 100.4% | 7 | 135 | 112 | 7 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-count | 39.7 | 408.3% | 4 | 4 | 3 | 4 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-sum | 200.9 | -6.5% | 42 | 609 | 542 | 42 | 0 | 0 |
| cache-read-ahead-no-preload | chat-tool-read-fanout | 182.6 | -0.4% | 12 | 613 | 1084 | 12 | 0 | 0 |
| no-read-ahead | chat-log-select-limit | 178.8 | 1.3% | 11 | 612 | 1081 | 11 | 0 | 0 |
| no-read-ahead | chat-log-select-indexed | 43.7 | -6.1% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-read-ahead | chat-log-count | 21.9 | 180.1% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-read-ahead | chat-log-sum | 202.2 | -5.9% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-read-ahead | chat-tool-read-fanout | 180.6 | -1.5% | 12 | 613 | 1084 | 12 | 0 | 0 |
| no-preload | chat-log-select-limit | 181.3 | 2.7% | 11 | 612 | 1081 | 11 | 0 | 0 |
| no-preload | chat-log-select-indexed | 43.1 | -7.4% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-preload | chat-log-count | 7.5 | -3.4% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-preload | chat-log-sum | 374.5 | 74.2% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-preload | chat-tool-read-fanout | 278.3 | 51.9% | 12 | 613 | 1084 | 12 | 0 | 0 |

Failed scenarios

- all-off: all-off matrix scenario failed with exit code 1
- transport-batching-only: transport-batching-only matrix scenario failed with exit code 1
- read-ahead-no-cache: read-ahead-no-cache matrix scenario failed with exit code 1
- no-vfs-cache: no-vfs-cache matrix scenario failed with exit code 1
