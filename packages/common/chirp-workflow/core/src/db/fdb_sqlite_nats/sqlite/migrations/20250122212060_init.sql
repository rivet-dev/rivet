-- Activity events
CREATE TABLE workflow_activity_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  activity_name TEXT NOT NULL,
  input_hash BLOB NOT NULL, -- u64
  input BLOB NOT NULL, -- JSONB
  -- Null if incomplete
  output BLOB, -- JSONB
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_activity_events_location_idx
ON workflow_activity_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_activity_events_loop_location_idx
ON workflow_activity_events (loop_location)
WHERE NOT forgotten;

CREATE TABLE workflow_activity_errors (
  location BLOB PRIMARY KEY, -- JSONB
  activity_name TEXT NOT NULL,
  error TEXT NOT NULL,
  ts INT NOT NULL,
  FOREIGN KEY (location) REFERENCES workflow_activity_events (location)
) STRICT;

-- Signal events
CREATE TABLE workflow_signal_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  signal_id BLOB NOT NULL, -- UUID
  signal_name TEXT NOT NULL,
  body BLOB NOT NULL, -- JSONB
  ack_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_signal_events_location_idx
ON workflow_signal_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_signal_events_loop_location_idx
ON workflow_signal_events (loop_location)
WHERE NOT forgotten;

-- Sub workflow events
CREATE TABLE workflow_sub_workflow_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  sub_workflow_id BLOB NOT NULL, -- UUID
  sub_workflow_name TEXT NOT NULL,
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_sub_workflow_events_location_idx
ON workflow_sub_workflow_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_sub_workflow_events_loop_location_idx
ON workflow_sub_workflow_events (loop_location)
WHERE NOT forgotten;

-- Signal send events
CREATE TABLE workflow_signal_send_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  signal_id BLOB NOT NULL, -- UUID
  signal_name TEXT NOT NULL,
  body BLOB NOT NULL, -- JSONB
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_signal_send_events_location_idx
ON workflow_signal_send_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_signal_send_events_loop_location_idx
ON workflow_signal_send_events (loop_location)
WHERE NOT forgotten;

-- Message send events
CREATE TABLE workflow_message_send_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  tags BLOB NOT NULL, -- JSONB
  message_name TEXT NOT NULL,
  body BLOB NOT NULL, -- JSONB
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_message_send_events_location_idx
ON workflow_message_send_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_message_send_events_loop_location_idx
ON workflow_message_send_events (loop_location)
WHERE NOT forgotten;

-- Loop events
CREATE TABLE workflow_loop_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  iteration INT NOT NULL,
  state BLOB NOT NULL, -- JSONB
  output BLOB, -- JSONB
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_loop_events_location_idx
ON workflow_loop_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_loop_events_loop_location_idx
ON workflow_loop_events (loop_location)
WHERE NOT forgotten;

-- Sleep events
CREATE TABLE workflow_sleep_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  deadline_ts INT NOT NULL,
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  state INT NOT NULL DEFAULT 0, -- event::SleepState
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_sleep_events_location_idx
ON workflow_sleep_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_sleep_events_loop_location_idx
ON workflow_sleep_events (loop_location)
WHERE NOT forgotten;

-- Branch events
CREATE TABLE workflow_branch_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_branch_events_location_idx
ON workflow_branch_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_branch_events_loop_location_idx
ON workflow_branch_events (loop_location)
WHERE NOT forgotten;

-- Removed events
CREATE TABLE workflow_removed_events (
  location BLOB PRIMARY KEY, -- JSONB
  event_type INT NOT NULL, -- event::EventType
  event_name TEXT NOT NULL,
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_removed_events_location_idx
ON workflow_removed_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_removed_events_loop_location_idx
ON workflow_removed_events (loop_location)
WHERE NOT forgotten;

-- Version check events
CREATE TABLE workflow_version_check_events (
  location BLOB PRIMARY KEY, -- JSONB
  version INT NOT NULL,
  create_ts INT NOT NULL,
  loop_location BLOB, -- JSONB
  forgotten INT NOT NULL DEFAULT false -- BOOLEAN
) STRICT;

CREATE INDEX workflow_version_check_events_location_idx
ON workflow_version_check_events (location)
WHERE NOT forgotten;

CREATE INDEX workflow_version_check_events_loop_location_idx
ON workflow_version_check_events (loop_location)
WHERE NOT forgotten;
