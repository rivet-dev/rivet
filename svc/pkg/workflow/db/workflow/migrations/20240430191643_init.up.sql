CREATE TABLE nodes (
  node_id UUID PRIMARY KEY,
  last_ping_ts INT
);

-- TODO: In the event of a node failure, clear all of the wake conditions and remove the node id. This can be
-- done in a periodic GC service
CREATE TABLE workflows (
  workflow_id UUID PRIMARY KEY,
  workflow_name TEXT NOT NULL,
  -- The node that's running this workflow
  node_id UUID,
  input TEXT NOT NULL,
  -- Null if incomplete
  output TEXT,

  wake_immediate BOOLEAN NOT NULL DEFAULT false,
  wake_deadline_ts INT,
  wake_signals TEXT[] NOT NULL DEFAULT ARRAY[],
  wake_sub_workflow_id UUID,

  INDEX (wake_immediate),
  INDEX (wake_deadline_ts),
  INDEX (wake_sub_workflow_id)
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
  input TEXT NOT NULL,
  -- Null if incomplete
  output TEXT,

  PRIMARY KEY (workflow_id, location)
);

-- Stores acknowledged signals for replay
CREATE TABLE workflow_signal_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  signal_id TEXT NOT NULL,
  signal_name TEXT NOT NULL,
  body TEXT NOT NULL,

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
  body TEXT NOT NULL,

  create_ts INT NOT NULL,

  INDEX (workflow_id),
  INDEX (signal_name)
);
