CREATE MATERIALIZED VIEW IF NOT EXISTS actor_logs2_with_metadata
(
    namespace LowCardinality(String),
    actor_id String,
    ts DateTime64(9),
    stream_type UInt8, -- pegboard::types::LogsStreamType
    message String,
    project_id UUID,
    env_id UUID,
    datacenter_id UUID,
    tags Map(String, String),
    build_id UUID,
    client_id UUID,
	durable Bool
)
ENGINE = ReplicatedMergeTree()
PARTITION BY (env_id, toStartOfHour(ts))
ORDER BY (env_id, toUnixTimestamp(ts), actor_id, stream_type)
TTL toDate(ts + toIntervalDay(14))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1
AS SELECT
	l.namespace,
    l.actor_id,
    l.ts,
    l.stream_type,
    l.message,
    a.project_id,
    a.env_id,
    a.datacenter_id,
    a.tags,
    a.build_id,
    a.client_id,
	a.durable
FROM actor_logs2 l
LEFT JOIN db_pegboard_analytics.actors a ON l.actor_id = a.actor_id;

