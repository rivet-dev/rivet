CREATE TABLE clients (
	client_id UUID PRIMARY KEY,
	create_ts INT NOT NULL,
	last_ping_ts INT NOT NULL,
	last_event_idx INT NOT NULL DEFAULT 0,
	last_command_idx INT NOT NULL DEFAULT 0,

	-- Total resources
	cpu INT NOT NULL DEFAULT 0,
	memory INT NOT NULL DEFAULT 0,

	drain_ts INT,
	delete_ts INT
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
	client_id UUID NOT NULL,
	config JSONB NOT NULL, -- pegboard::protocol::ContainerConfig
	create_ts INT NOT NULL,

	-- See protocol.rs `ContainerState` for info
	start_ts INT,
	running_ts INT,
	stopping_ts INT,
	stop_ts INT,
	exit_ts INT,

	pid INT,
	exit_code INT,

	INDEX(client_id)
);

CREATE INDEX ON containers (((config->'resources'->'cpu')::INT));
CREATE INDEX ON containers (((config->'resources'->'memory')::INT));
