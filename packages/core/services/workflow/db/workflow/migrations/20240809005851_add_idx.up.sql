-- Unused
DROP INDEX workflows@workflows_wake_deadline_ts_idx;
DROP INDEX workflows@workflows_wake_sub_workflow_id_idx;
DROP INDEX workflows@gin_workflows_wake_signals;

-- For forgetting events in `update_loop` fn
CREATE INDEX ON workflow_activity_events (loop_location) STORING (activity_name, input_hash, input, output, create_ts, forgotten);
CREATE INDEX ON workflow_loop_events (loop_location) STORING (iteration, output, forgotten);
CREATE INDEX ON workflow_message_send_events (loop_location) STORING (tags, message_name, body, forgotten);
CREATE INDEX ON workflow_signal_events (loop_location) STORING (signal_id, signal_name, body, ack_ts, forgotten);
CREATE INDEX ON workflow_signal_send_events (loop_location) STORING (signal_id, signal_name, body, forgotten);
CREATE INDEX ON workflow_sub_workflow_events (loop_location) STORING (sub_workflow_id, create_ts, forgotten);

-- For `pull_workflows` fn
CREATE INDEX ON signals (ack_ts) STORING (workflow_id, signal_name);
CREATE INDEX ON tagged_signals (ack_ts) STORING (tags, signal_name);
CREATE INDEX ON workflows (worker_instance_id)
STORING (
	workflow_name, create_ts, ray_id, input, output, error, wake_immediate, wake_deadline_ts, wake_signals, wake_sub_workflow_id, tags
);
DROP INDEX workflows@workflows_worker_instance_id_idx;

-- For `workflow-gc`
CREATE INDEX ON worker_instances (last_ping_ts);

-- For `workflow-metrics-publish`
CREATE INDEX ON workflows (wake_deadline_ts, wake_sub_workflow_id, error)
STORING (workflow_name, output, wake_immediate, wake_signals);
