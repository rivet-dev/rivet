CREATE TABLE worker_instances (
  worker_instance_id UUID PRIMARY KEY,
  last_ping_ts INT
);

-- NOTE: If a row has `worker_instance_id` set and `output` unset, it is currently running
CREATE TABLE workflows (
  workflow_id UUID PRIMARY KEY,
  workflow_name TEXT NOT NULL,
  create_ts INT NOT NULL,
  ray_id UUID NOT NULL,
  -- The worker instance that's running this workflow
  worker_instance_id UUID,

  input JSONB NOT NULL,
  -- Null if incomplete
  output JSONB,

  wake_immediate BOOLEAN NOT NULL DEFAULT false,
  wake_deadline_ts INT,
  wake_signals TEXT[] NOT NULL DEFAULT ARRAY[],
  wake_sub_workflow_id UUID,

  INDEX (wake_immediate),
  INDEX (wake_deadline_ts),
  INDEX (wake_sub_workflow_id),

  -- Query by worker_instance_id for failover
  INDEX(worker_instance_id)
);

CREATE INDEX gin_workflows_wake_signals
ON workflows
USING GIN (wake_signals);

-- Stores activity outputs for replay
CREATE TABLE workflow_activity_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  activity_name TEXT NOT NULL,
  -- CRDB can't store u64, so we have to store bytes
  input_hash BYTES NOT NULL,
  input JSONB NOT NULL,
  -- Null if incomplete
  output JSONB,

  PRIMARY KEY (workflow_id, location)
);

-- Stores acknowledged signals for replay
CREATE TABLE workflow_signal_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  signal_id TEXT NOT NULL,
  signal_name TEXT NOT NULL,
  body JSONB NOT NULL,

  PRIMARY KEY (workflow_id, location)
);

-- Stores sub workflow for replay
CREATE TABLE workflow_sub_workflow_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  sub_workflow_id UUID NOT NULL REFERENCES workflows,

  PRIMARY KEY (workflow_id, location)
);

-- Stores pending signals
CREATE TABLE signals (
  signal_id UUID PRIMARY KEY,
  -- This doesn't reference the workflows table because it is possible to insert signals before a workflow
  -- exists
  workflow_id UUID NOT NULL,
  signal_name TEXT NOT NULL,

  create_ts INT NOT NULL,
  ray_id UUID NOT NULL,

  body JSONB NOT NULL,

  INDEX (workflow_id),
  INDEX (signal_name)
);
