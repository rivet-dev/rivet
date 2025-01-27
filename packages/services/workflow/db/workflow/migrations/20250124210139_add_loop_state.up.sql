ALTER TABLE workflow_loop_events
	ADD COLUMN state JSONB NOT NULL DEFAULT 'null';
