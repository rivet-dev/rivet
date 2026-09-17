# UniversalDB Transactions

## Basic Transaction Structure

All database operations run inside a transaction using `db.run()`. The closure receives a `RetryableTransaction` and returns a `Result<T>`. Transactions are automatically retried on conflicts.

```rust
ctx.udb()?
    .run(|tx| async move {
        // ... database operations ...
        Ok(result)
    })
    .custom_instrument(tracing::info_span!("my_transaction"))
    .await?;
```

## Subspaces

Scope transactions to a key prefix using `with_subspace()`. This is typically the first operation inside a transaction.

```rust
ctx.udb()?
    .run(|tx| async move {
        let tx = tx.with_subspace(keys::subspace());
        // All operations are now scoped to this subspace
        Ok(())
    })
    .await?;
```

## Isolation Levels

Two isolation levels control read behavior:

- **`Serializable`**: Reads add conflict ranges. Transaction fails if read keys are modified by another transaction.
- **`Snapshot`**: Reads don't add conflict ranges. Suitable for read-only queries where stale data is acceptable.

```rust
use universaldb::utils::IsolationLevel::*;

// Use Serializable for reads that affect writes
let value = tx.read(&my_key, Serializable).await?;

// Use Snapshot for listing/queries where staleness is acceptable
let value = tx.read(&my_key, Snapshot).await?;
```

## Reading Data

### Single Key Read

```rust
// Read with FormalKey (returns error if key doesn't exist)
let value: T::Value = tx.read(&my_key, Serializable).await?;

// Read with FormalKey (returns Option)
let value: Option<T::Value> = tx.read_opt(&my_key, Serializable).await?;

// Check if key exists
let exists: bool = tx.exists(&my_key, Snapshot).await?;
```

### Low-Level Read

```rust
// Get raw bytes (without FormalKey deserialization)
let raw: Option<Slice> = tx.get(&packed_key, Serializable).await?;
```

## Writing Data

```rust
// Write a key-value pair (uses FormalKey for serialization)
tx.write(&my_key, my_value)?;

// Delete a single key
tx.delete(&my_key);

// Delete all keys in a subspace
tx.delete_subspace(&some_subspace);
```

## Range Queries

Stream over a range of keys using `get_ranges_keyvalues()`.

```rust
use futures_util::TryStreamExt;
use universaldb::options::StreamingMode;

let subspace = keys::subspace().subspace(&MyKey::subspace(id));
let (start, end) = subspace.range();

let mut stream = tx.get_ranges_keyvalues(
    universaldb::RangeOption {
        mode: StreamingMode::Iterator,
        reverse: true,  // Optional: iterate in reverse order
        ..(start, end).into()
    },
    Snapshot,
);

while let Some(entry) = stream.try_next().await? {
    let (key, value) = tx.read_entry::<MyKey>(&entry)?;
    // Process entry...
}
```

### RangeOption Fields

- `begin` / `end`: Key selectors defining the range
- `limit`: Maximum number of results
- `mode`: How large each page of the read is. See [Range Paging](#range-paging)
- `target_bytes`: Lowers the page size a mode implies. `0` keeps the mode's own size
- `reverse`: Iterate in reverse lexicographical order

### Range Paging

A range read never loads the whole range. Drivers return it one page at a time, and `get_ranges_keyvalues()` fetches the next page only when the consumer has drained the current one. A stream that is dropped early leaves the rest of the range unread, and a read holds at most one page in memory however large the range is.

A page ends at `limit` rows or after the row that brings it to the mode's byte budget, whichever comes first. It always holds at least one row. The budgets are the ones FoundationDB's client uses per batch:

| Mode | Page budget |
|------|-------------|
| `Iterator` (default) | 4 KB for the first page, growing 1.5x per page up to 120 KB |
| `WantAll`, `Serial` | 120 KB |
| `Large` | 4 KB |
| `Medium` | 1 KB |
| `Small` | 256 B |
| `Exact` | `limit` rows in one page, with no byte budget |

- Use `Iterator` when the consumer may stop early and `WantAll` when it reads the whole range.
- Put a known row cap in `RangeOption.limit` instead of only breaking out of the consumer loop. The last page then stops at the cap rather than filling its byte budget.
- A `Serializable` read adds one read conflict range for the whole range, not one per page.
- Pending writes of the transaction are merged into the page that covers their key, so a paged read returns the same rows in the same order as an unpaged one.
- As in FoundationDB, a range stream that is still being consumed returns keys the transaction writes ahead of it. Collect the range before writing when the writes can land in the part of the range that is not read yet.
- Paging bounds memory, not work. A range read still returns every value it passes. When small keys are interleaved with large values the caller does not need, seek past them with `get_key` instead of scanning.

Each page is one `TransactionDriver::get_range` call. It returns `Values`, whose `more()` says whether the range has rows past the page, and `RangeOption::next_range(&page)` yields the range left to read.

### Building Range Bounds

```rust
// From subspace (most common)
let subspace = keys::subspace().subspace(&MyKey::subspace(ns_id, name));
let (start, end) = subspace.range();

// Custom end bound (e.g., for pagination by create_ts)
let end = if let Some(created_before) = created_before {
    universaldb::utils::end_of_key_range(&tx.pack(
        &MyKey::subspace_with_create_ts(ns_id, name, created_before),
    ))
} else {
    end
};

// Build RangeOption
let range_opt = universaldb::RangeOption {
    mode: StreamingMode::Iterator,
    reverse: true,
    ..(start, end).into()
};
```

### KeySelector for Pagination

```rust
use universaldb::KeySelector;

// Start after a specific key (for cursor-based pagination)
let begin = if let Some(after_id) = after_id {
    let after_key = MyKey::new(after_id);
    KeySelector::first_greater_than(tx.pack(&after_key))
} else {
    KeySelector::first_greater_or_equal(subspace_start)
};
```

## Transaction Limitations

UniversalDB follows FoundationDB's transaction model with strict limits:

| Limit | Value |
|-------|-------|
| **Time limit** | 5 seconds |
| **Transaction size** | 10 MB total (keys + values) |
| **Key size** | 10 KB max |
| **Value size** | 100 KB max |

Transactions that exceed these limits will fail.

## Early Timeout Pattern

For long-running iterations, check elapsed time and exit early to avoid transaction timeout:

```rust
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

ctx.udb()?
    .run(|tx| async move {
        let start = Instant::now();
        let mut stream = tx.get_ranges_keyvalues(range_opt, Snapshot);

        while let Some(entry) = stream.try_next().await? {
            // Exit early to avoid 5s transaction timeout
            if start.elapsed() > EARLY_TXN_TIMEOUT {
                tracing::warn!("timed out, will continue in next batch");
                break;
            }

            // Process entry...
        }

        Ok(last_cursor)  // Return cursor for next batch
    })
    .await?;
```

Use `2500ms` as a safe threshold (half of 5s limit) to allow time for commit.

## Value Chunking

For values exceeding the 100 KB limit, use `FormalChunkedKey` to split across multiple keys:

```rust
impl FormalChunkedKey for MetadataKey {
    type ChunkKey = MetadataChunkKey;
    type Value = MetadataKeyData;

    fn chunk(&self, chunk: usize) -> Self::ChunkKey {
        MetadataChunkKey { runner_id: self.runner_id, chunk }
    }

    fn combine(&self, chunks: Vec<Value>) -> Result<Self::Value> {
        // Concatenate chunk bytes and deserialize
        let bytes: Vec<u8> = chunks.iter()
            .flat_map(|x| x.value().iter().copied())
            .collect();
        deserialize(&bytes)
    }

    fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
        let bytes = serialize(value)?;
        Ok(bytes.chunks(universaldb::utils::CHUNK_SIZE)  // 10 KB chunks
            .map(|x| x.to_vec())
            .collect())
    }
}
```

The chunk size is 10 KB (`universaldb::utils::CHUNK_SIZE`).

## Key Unpacking

### Unpack Key Tuple

Extract tuple components from raw key bytes:

```rust
// Unpack to a typed key struct
let key: MyKey = tx.unpack(entry.key())?;

// Unpack to raw tuple (useful for filtering by key type)
let (prefix1, prefix2, id, key_type): (usize, usize, Id, usize) =
    tx.unpack(entry.key())?;
```

### Read Entry (Key + Value)

Parse both the key and value from a range query entry:

```rust
let (key, value) = tx.read_entry::<MyKey>(&entry)?;
```

## FormalKey Pattern

Keys implement `FormalKey` to define their associated value type and serialization:

```rust
impl FormalKey for MyKey {
    type Value = MyValueType;

    fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
        // Deserialize bytes to value
    }

    fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
        // Serialize value to bytes
    }
}
```

This enables type-safe reads/writes:

```rust
// tx.write() uses MyKey::serialize()
tx.write(&MyKey::new(id), my_value)?;

// tx.read() uses MyKey::deserialize()
let value: MyValueType = tx.read(&MyKey::new(id), Serializable).await?;
```
