CREATE TABLE workflow_sleep_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  deadline_ts INT NOT NULL,

  loop_location INT[],
  forgotten BOOLEAN NOT NULL DEFAULT FALSE,

  PRIMARY KEY (workflow_id, location)
);

-- Query by sleep location
CREATE INDEX idx_workflow_sleep_events_loop_location
ON workflow_sleep_events (workflow_id, loop_location);
