-- For `pull_workflows` fn
CREATE INDEX ON signals (ack_ts, silence_ts, workflow_id)
STORING (signal_name);
CREATE INDEX ON workflows (worker_instance_id)
STORING (
	workflow_name,
	create_ts,
	ray_id,
	input,
	output,
	error,
	wake_immediate,
	wake_deadline_ts,
	wake_signals,
	wake_sub_workflow_id,
	tags,
	silence_ts
);
DROP INDEX workflows@workflows_worker_instance_id_idx1;
