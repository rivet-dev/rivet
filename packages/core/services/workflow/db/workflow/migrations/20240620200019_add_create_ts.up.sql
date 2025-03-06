ALTER TABLE workflow_activity_events
ADD COLUMN create_ts INT NOT NULL DEFAULT 1;

ALTER TABLE workflow_signal_events
ADD COLUMN ack_ts INT NOT NULL DEFAULT 1;

ALTER TABLE workflow_sub_workflow_events
ADD COLUMN create_ts INT NOT NULL DEFAULT 1;
