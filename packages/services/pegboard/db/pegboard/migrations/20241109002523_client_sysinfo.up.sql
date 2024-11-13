ALTER TABLE clients
	DROP COLUMN cpu,
	DROP COLUMN memory;

ALTER TABLE clients ADD system_info JSONB;

