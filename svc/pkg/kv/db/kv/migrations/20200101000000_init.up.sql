CREATE TABLE kv (
	namespace_id UUID NOT NULL,
	key STRING NOT NULL,
	value JSONB NOT NULL,
	update_ts INT NOT NULL,
	directory STRING NOT NULL,
	PRIMARY KEY (namespace_id, key)
);

