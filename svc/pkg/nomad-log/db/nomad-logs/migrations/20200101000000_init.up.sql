CREATE TABLE IF NOT EXISTS logs
(
	alloc String,
	task String,
	stream_type UInt8, -- backend::nomad_log::StreamType
	ts DateTime64(3),
	idx UInt32,  -- Index of the message sent within the same timestamp
	message String
)
ENGINE = ReplicatedMergeTree()
PARTITION BY toStartOfHour(ts)
ORDER BY (ts, idx, alloc, task, stream_type)
TTL toDate(ts + toIntervalDay(2))
SETTINGS index_granularity = 32768;
