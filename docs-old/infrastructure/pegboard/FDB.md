# Design

- txns cannot have more than 10MB of data transferred, should be below 1MB
  - limit total txn size to 1MB
- txns cannot run for more than 5 seconds (shouldnt be an issue)
- keys cannot be larger than 10KB, but should be below 1KB
- values cannot be larger than 100KB, but should be chunked at 10KB
  - use subspace for each key and chunk data
  - use get_ranges_key_values to retrieve
- 1GiB max size using get_estimated_range_size_bytes (check before puts)

## Schema

```
structure                    created

dir pegboard                  before actor start
  partition {actor_id}        before actor start
    dir kv                    before actor start
      subspace {key}          at start of each put txn
        key metadata          during put txn
        subspace data         during put txn
          key {idx}           during put txn
```
