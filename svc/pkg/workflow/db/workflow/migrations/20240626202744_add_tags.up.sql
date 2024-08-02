-- Add tags
ALTER TABLE workflows
  ADD COLUMN tags JSONB;

CREATE INDEX gin_workflows_tags
ON workflows
USING GIN (tags);


-- Stores pending signals with tags
CREATE TABLE tagged_signals (
  signal_id UUID PRIMARY KEY,
  tags JSONB NOT NULL,
  signal_name TEXT NOT NULL,

  create_ts INT NOT NULL,
  ack_ts INT,
  ray_id UUID NOT NULL,

  body JSONB NOT NULL
);

CREATE INDEX gin_tagged_signals_tags
ON tagged_signals
USING GIN (tags)
WHERE ack_ts IS NULL;

-- Fix signal indexes
ALTER TABLE signals
  ADD COLUMN ack_ts INT;

DROP INDEX signals@signals_signal_name_idx;
DROP INDEX signals@signals_workflow_id_idx;
