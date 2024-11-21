ALTER TABLE servers
	RENAME COLUMN kill_timeout_ms TO lifecycle_kill_timeout_ms,
	ADD COLUMN lifecycle_durable BOOLEAN DEFAULT false;
