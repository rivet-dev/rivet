-- For `pull_workflows`

CREATE INDEX workflows_pred_standard
ON db_workflow.workflows (workflow_name)
WHERE
	output IS NULL AND
	worker_instance_id IS NULL AND
	silence_ts IS NULL;

CREATE INDEX workflows_pred_signals
ON db_workflow.workflows (workflow_name)
WHERE
	output IS NULL AND
	worker_instance_id IS NULL AND
	silence_ts IS NULL AND
	array_length(wake_signals, 1) != 0;

CREATE INDEX workflows_pred_sub_workflow
ON db_workflow.workflows (workflow_name)
WHERE
	output IS NULL AND
	worker_instance_id IS NULL AND
	silence_ts IS NULL AND
	wake_sub_workflow_id IS NOT NULL;

CREATE INDEX workflows_pred_sub_workflow_internal
ON db_workflow.workflows (workflow_id)
WHERE
	output IS NOT NULL;

CREATE INDEX signals_partial
ON db_workflow.signals (workflow_id, signal_name)
WHERE
	ack_ts IS NULL AND
	silence_ts IS NULL;

CREATE INDEX tagged_signals_partial
ON db_workflow.tagged_signals
USING gin (tags)
WHERE
	ack_ts IS NULL AND
	silence_ts IS NULL;

-- For observability

ALTER TABLE workflows
	ADD COLUMN last_pull_ts INT;
