CREATE TABLE IF NOT EXISTS actor_logs2 (
    namespace LowCardinality(String),
    actor_id String,
    ts DateTime64 (9),
    stream_type UInt8, -- pegboard::types::LogsStreamType
    message String
) ENGINE = ReplicatedMergeTree ()
PARTITION BY
    toStartOfHour (ts)
ORDER BY (
	namespace,
    actor_id,
    toUnixTimestamp (ts),
    stream_type
)
TTL toDate (ts + toIntervalDay(14))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
