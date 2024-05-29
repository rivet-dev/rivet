SET allow_experimental_object_type = 1;

CREATE TABLE IF NOT EXISTS cf_tail_events
(
	script_name String,
	ts DateTime64(3),  -- This is tail_event.eventTimestamp, DateTime64(3) is milliseconds
	ray_id UUID,  -- Extracted by tail worker
	tail_event JSON,  -- TODO: might need to do the trickery used in analytics event
	INDEX idx_ray_id (ray_id) TYPE bloom_filter() GRANULARITY 4
)
ENGINE = ReplicatedMergeTree()
PARTITION BY toStartOfHour(ts)
ORDER BY (script_name, ts, ray_id)
TTL toDate(ts + toIntervalDay(3))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
