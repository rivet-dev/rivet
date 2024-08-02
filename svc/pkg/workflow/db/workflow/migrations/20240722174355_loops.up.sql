-- Stores loops for replay
CREATE TABLE workflow_loop_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  iteration INT NOT NULL,
  output JSONB,

  loop_location INT[],
  forgotten BOOLEAN NOT NULL DEFAULT FALSE,

  PRIMARY KEY (workflow_id, location)
);

-- Query by loop location
CREATE INDEX idx_workflow_loop_events_loop_location
ON workflow_loop_events (workflow_id, loop_location);



ALTER TABLE workflow_activity_events
	ADD COLUMN loop_location INT[],
	ADD COLUMN forgotten BOOLEAN NOT NULL DEFAULT FALSE;

-- Query by loop location
CREATE INDEX idx_workflow_activity_events_loop_location
ON workflow_activity_events (workflow_id, loop_location);

ALTER TABLE workflow_signal_events
	ADD COLUMN loop_location INT[],
	ADD COLUMN forgotten BOOLEAN NOT NULL DEFAULT FALSE;

-- Query by loop location
CREATE INDEX idx_workflow_signal_events_loop_location
ON workflow_signal_events (workflow_id, loop_location);

ALTER TABLE workflow_sub_workflow_events
	ADD COLUMN loop_location INT[],
	ADD COLUMN forgotten BOOLEAN NOT NULL DEFAULT FALSE;

-- Query by loop location
CREATE INDEX idx_workflow_sub_workflow_events_loop_location
ON workflow_sub_workflow_events (workflow_id, loop_location);

ALTER TABLE workflow_signal_send_events
	ADD COLUMN loop_location INT[],
	ADD COLUMN forgotten BOOLEAN NOT NULL DEFAULT FALSE;

-- Query by loop location
CREATE INDEX idx_workflow_signal_send_events_loop_location
ON workflow_signal_send_events (workflow_id, loop_location);

ALTER TABLE workflow_message_send_events
	ADD COLUMN loop_location INT[],
	ADD COLUMN forgotten BOOLEAN NOT NULL DEFAULT FALSE;

-- Query by loop location
CREATE INDEX idx_workflow_message_send_events_loop_location
ON workflow_message_send_events (workflow_id, loop_location);
