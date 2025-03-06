CREATE TABLE IF NOT EXISTS actor_logs (
    actor_id UUID,
    stream_type UInt8, -- pegboard::types::LogsStreamType
    ts DateTime64 (9),
    message String
) ENGINE = ReplicatedMergeTree ()
PARTITION BY
    toStartOfHour (ts)
ORDER BY (
    actor_id,
    toUnixTimestamp (ts),
    stream_type
)
TTL toDate (ts + toIntervalDay (3))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
