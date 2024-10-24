CREATE TABLE IF NOT EXISTS server_logs (
    server_id UUID,
    stream_type UInt8, -- backend::ds::log::StreamType
    ts DateTime64 (9),
    message String
) ENGINE = ReplicatedMergeTree ()
PARTITION BY
    toStartOfHour (ts)
ORDER BY (
    server_id,
    toUnixTimestamp (ts),
    stream_type
)
TTL toDate (ts + toIntervalDay (3))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
