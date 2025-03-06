ALTER TABLE servers
	RENAME COLUMN kill_timeout_ms TO lifecycle_kill_timeout_ms,
	ADD COLUMN lifecycle_durable BOOLEAN NOT NULL DEFAULT false,
	ADD COLUMN reschedule_retry_count INT NOT NULL DEFAULT 0,
	ADD COLUMN last_reschedule_retry_ts INT;
