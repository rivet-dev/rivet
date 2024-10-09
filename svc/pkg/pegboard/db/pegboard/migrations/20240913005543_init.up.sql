CREATE TABLE clients (
	client_id UUID PRIMARY KEY,
	create_ts INT NOT NULL,
	last_event_idx INT NOT NULL DEFAULT 0,
	last_command_idx INT NOT NULL DEFAULT 0
);

CREATE TABLE client_events (
	client_id UUID PRIMARY KEY,
	index INT NOT NULL,
	payload JSONB NOT NULL,
	ack_ts INT NOT NULL,

	UNIQUE (client_id, index)
);

CREATE TABLE client_commands (
	client_id UUID PRIMARY KEY,
	index INT NOT NULL,
	payload JSONB NOT NULL,
	create_ts INT NOT NULL,

	UNIQUE (client_id, index)
);

CREATE TABLE containers (
	container_id UUID PRIMARY KEY,
	config JSONB NOT NULL, -- pegboard::protocol::ContainerConfig
	create_ts INT NOT NULL,

	-- See protocol.rs `ContainerState` for info
	start_ts INT,
	running_ts INT,
	stopping_ts INT,
	stop_ts INT,
	exit_ts INT,

	pid INT,
	exit_code INT
);

CREATE INDEX containers_config_resources_cpu_idx
ON containers (config->'resources'->'cpu'::INT);
CREATE INDEX containers_config_resources_memory_idx
ON containers (config->'resources'->'memory'::INT);
