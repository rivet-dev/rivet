CREATE INDEX workflows_active_idx 
ON db_workflow.workflows (worker_instance_id)
WHERE
    output IS NULL AND
    silence_ts IS NULL AND
    (
        wake_immediate OR 
        wake_deadline_ts IS NOT NULL OR 
        cardinality(wake_signals) > 0 OR 
        wake_sub_workflow_id IS NOT NULL
    );

CREATE INDEX worker_instances_ping_idx
ON db_workflow.worker_instances (last_ping_ts, worker_instance_id);

CREATE INDEX workflows_total_count_idx 
ON db_workflow.workflows (workflow_name);

CREATE INDEX workflows_active_count_idx
ON db_workflow.workflows (workflow_name)
WHERE
    output IS NULL AND
    worker_instance_id IS NOT NULL AND
    silence_ts IS NULL;

CREATE INDEX workflows_dead_count_idx 
ON db_workflow.workflows (workflow_name, error)
WHERE
    error IS NOT NULL AND 
    output IS NULL AND 
    silence_ts IS NULL AND 
    wake_immediate = FALSE AND 
    wake_deadline_ts IS NULL AND 
    cardinality(wake_signals) = 0 AND 
    wake_sub_workflow_id IS NULL;

CREATE INDEX workflows_sleeping_count_idx 
ON db_workflow.workflows (workflow_name)
WHERE
    worker_instance_id IS NULL AND 
    output IS NULL AND 
    silence_ts IS NULL AND 
    (
        wake_immediate OR 
        wake_deadline_ts IS NOT NULL OR 
        cardinality(wake_signals) > 0 OR 
        wake_sub_workflow_id IS NOT NULL
    );

CREATE INDEX signals_unack_idx
ON db_workflow.signals (signal_name)
WHERE ack_ts IS NULL AND silence_ts IS NULL;

CREATE INDEX tagged_signals_unack_idx
ON db_workflow.tagged_signals (signal_name)
WHERE ack_ts IS NULL AND silence_ts IS NULL;

DROP INDEX IF EXISTS workflow_sleep_events@workflow_sleep_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_removed_events@workflow_removed_events_workflow_id_loop_location_hash_idx1;
DROP INDEX IF EXISTS workflow_branch_events@workflow_branch_events_workflow_id_loop_location_hash_idx1;
DROP INDEX IF EXISTS workflow_version_check_events@workflow_version_check_events_workflow_id_loop_location_hash_idx1;
DROP INDEX IF EXISTS workflow_activity_events@workflow_activity_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_signal_events@workflow_signal_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_sub_workflow_events@workflow_sub_workflow_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_signal_send_events@workflow_signal_send_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_message_send_events@workflow_message_send_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_loop_events@workflow_loop_events_workflow_id_loop_location2_hash_idx1;
DROP INDEX IF EXISTS workflow_signal_send_events@workflow_signal_send_events_loop_location_idx;
