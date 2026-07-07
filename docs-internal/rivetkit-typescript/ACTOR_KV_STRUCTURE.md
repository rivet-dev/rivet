# Actor KV Structure

This is a legacy reference for the frozen actor KV keyspace used by pre-SQLite RivetKit actors. Current RivetKit runtime persistence lives in internal SQLite tables in `rivetkit-core`; the first wake on the migrated runtime imports these keys into SQLite and leaves the KV bytes untouched for downgrade safety. The only live post-migration write to this keyspace is the inspector token mirror at `[3]`.

## Master Tree

```text
persisted data (1)/    # Actor state metadata blob.
  actor-persist

connections (2)/    # conn_id is UTF-8 bytes.
  {conn_id_utf8}/
    conn-persist

inspector token (3)/
  token

user kv (4)/
  {user_key_bytes...}/
    value

queue (5)/    # Queue namespace.
  v1 (1)/    # Queue data version.
    metadata (1)    # Queue metadata payload.
    messages (2)/
      {message_id_u64_be}/
        message

workflow (6)/    # Workflow namespace.
  v1 (1)/    # Workflow data version.
    names (1)/
      {name_index}/
    history (2)/
      {location_segments...}    # fdb-tuple path segments: name_index or [loop_idx, iteration].
    workflow fields (3)/
      state (1)
      output (2)
      error (3)
      input (4)
    entry metadata (4)/
      {entry_id}

traces (7)/    # Traces namespace.
  v1 (1)/    # Traces data version.
    data (1)/
      {bucket_start_sec}/
        {chunk_id}    # fdb-tuple key: [1, bucket_start_sec, chunk_id].
```
