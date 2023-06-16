SET allow_experimental_object_type = 1;

CREATE TABLE events (
  event_id UUID,
  ray_id UUID,
  ts DateTime64(3),
  name LowCardinality(String),
  properties JSON DEFAULT properties_raw,
  properties_raw String EPHEMERAL
) ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(ts)
SAMPLE BY cityHash64(event_id)
ORDER BY (toDate(ts), name, cityHash64(event_id));

