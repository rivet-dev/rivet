CREATE TABLE workflow_gc (
	worker_instance_id UUID,
	lock_ts INT
);

INSERT INTO workflow_gc (worker_instance_id, lock_ts) VALUES (NULL, NULL);

CREATE TABLE workflow_metrics (
	worker_instance_id UUID,
	lock_ts INT
);

INSERT INTO workflow_metrics (worker_instance_id, lock_ts) VALUES (NULL, NULL);
