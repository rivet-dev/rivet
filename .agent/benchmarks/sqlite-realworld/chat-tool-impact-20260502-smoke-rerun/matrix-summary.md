SQLite optimization impact matrix

Each scenario runs in a fresh process so process-wide SQLite optimization flags are read once per configuration.

| scenario | workload | server_ms | delta_vs_defaults | get_pages | fetched_pages | cache_hits | cache_misses | routed_reads | write_fallbacks |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| defaults | chat-log-select-limit | 171.6 | 0.0% | 10 | 582 | 1082 | 10 | 0 | 0 |
| defaults | chat-log-select-indexed | 45.2 | 0.0% | 7 | 135 | 112 | 7 | 0 | 0 |
| defaults | chat-log-count | 7.9 | 0.0% | 4 | 4 | 3 | 4 | 0 | 0 |
| defaults | chat-log-sum | 272.9 | 0.0% | 42 | 609 | 542 | 42 | 0 | 0 |
| defaults | chat-tool-read-fanout | 221.4 | 0.0% | 12 | 613 | 1084 | 12 | 0 | 0 |
| vfs-cache-only | chat-log-select-limit | 198.9 | 15.9% | 10 | 582 | 1082 | 10 | 0 | 0 |
| vfs-cache-only | chat-log-select-indexed | 53.1 | 17.5% | 7 | 135 | 112 | 7 | 0 | 0 |
| vfs-cache-only | chat-log-count | 7.8 | -2.1% | 4 | 4 | 3 | 4 | 0 | 0 |
| vfs-cache-only | chat-log-sum | 256.1 | -6.2% | 42 | 609 | 542 | 42 | 0 | 0 |
| vfs-cache-only | chat-tool-read-fanout | 195.8 | -11.5% | 12 | 613 | 1084 | 12 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-select-limit | 189.8 | 10.6% | 10 | 582 | 1082 | 10 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-select-indexed | 47.0 | 4.1% | 7 | 135 | 112 | 7 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-count | 9.9 | 24.7% | 4 | 4 | 3 | 4 | 0 | 0 |
| cache-read-ahead-no-preload | chat-log-sum | 281.7 | 3.2% | 42 | 609 | 542 | 42 | 0 | 0 |
| cache-read-ahead-no-preload | chat-tool-read-fanout | 194.1 | -12.3% | 12 | 613 | 1084 | 12 | 0 | 0 |
| no-read-ahead | chat-log-select-limit | 241.9 | 41.0% | 11 | 612 | 1081 | 11 | 0 | 0 |
| no-read-ahead | chat-log-select-indexed | 51.0 | 12.8% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-read-ahead | chat-log-count | 7.9 | -0.9% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-read-ahead | chat-log-sum | 274.6 | 0.6% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-read-ahead | chat-tool-read-fanout | 219.6 | -0.8% | 12 | 613 | 1084 | 12 | 0 | 0 |
| no-preload | chat-log-select-limit | 183.4 | 6.9% | 10 | 582 | 1082 | 10 | 0 | 0 |
| no-preload | chat-log-select-indexed | 45.0 | -0.3% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-preload | chat-log-count | 10.2 | 28.0% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-preload | chat-log-sum | 199.0 | -27.1% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-preload | chat-tool-read-fanout | 197.7 | -10.7% | 12 | 613 | 1084 | 12 | 0 | 0 |
| no-range-reads | chat-log-select-limit | 203.2 | 18.4% | 11 | 612 | 1081 | 11 | 0 | 0 |
| no-range-reads | chat-log-select-indexed | 47.8 | 5.7% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-range-reads | chat-log-count | 12.6 | 59.4% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-range-reads | chat-log-sum | 277.7 | 1.7% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-range-reads | chat-tool-read-fanout | 202.0 | -8.7% | 11 | 583 | 1085 | 11 | 0 | 0 |
| no-storage-read-cache | chat-log-select-limit | 188.3 | 9.7% | 10 | 582 | 1082 | 10 | 0 | 0 |
| no-storage-read-cache | chat-log-select-indexed | 48.8 | 7.9% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-storage-read-cache | chat-log-count | 7.2 | -9.5% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-storage-read-cache | chat-log-sum | 342.6 | 25.5% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-storage-read-cache | chat-tool-read-fanout | 192.9 | -12.9% | 11 | 583 | 1085 | 11 | 0 | 0 |
| no-read-pool | chat-log-select-limit | 185.9 | 8.3% | 10 | 582 | 1082 | 10 | 0 | 0 |
| no-read-pool | chat-log-select-indexed | 62.4 | 38.1% | 7 | 135 | 112 | 7 | 0 | 0 |
| no-read-pool | chat-log-count | 7.5 | -4.9% | 4 | 4 | 3 | 4 | 0 | 0 |
| no-read-pool | chat-log-sum | 211.7 | -22.4% | 42 | 609 | 542 | 42 | 0 | 0 |
| no-read-pool | chat-tool-read-fanout | 201.0 | -9.2% | 12 | 613 | 1084 | 12 | 0 | 0 |

Failed scenarios

- all-off: all-off matrix scenario failed with exit code 1
- transport-batching-only: transport-batching-only matrix scenario failed with exit code 1
- read-ahead-no-cache: read-ahead-no-cache matrix scenario failed with exit code 1
- no-vfs-cache: no-vfs-cache matrix scenario failed with exit code 1
