ALTER TABLE db_workflow.workflow_sleep_events
	ADD COLUMN state INT NOT NULL DEFAULT 0; -- event::SleepState
