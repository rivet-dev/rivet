CREATE TABLE IF NOT EXISTS service_logs (
    service_name String,
    ts DateTime64 (9),
    stream String,
    level String,
    fields Map(String, String),
    message String
) ENGINE = ReplicatedMergeTree ()
PARTITION BY
    toStartOfHour (ts)
ORDER BY (
    service_name,
    toUnixTimestamp (ts),
    stream
)
TTL toDate (ts + toIntervalDay (15))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
