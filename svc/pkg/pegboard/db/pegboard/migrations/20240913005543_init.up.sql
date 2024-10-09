CREATE TABLE clients (
	client_id UUID PRIMARY KEY,
	create_ts INT NOT NULL
);

CREATE TABLE client_state (
	client_id UUID PRIMARY KEY REFERENCES clients (client_id),
	last_event_idx INT NOT NULL
);

CREATE TABLE client_events (
	client_id UUID PRIMARY KEY,
	index INT NOT NULL,
	ack_ts INT NOT NULL
);

CREATE TABLE client_commands (
	client_id UUID PRIMARY KEY,
	index INT NOT NULL,
	create_ts INT NOT NULL
);

CREATE TABLE containers (
	container_id UUID PRIMARY KEY,
	start_ts INT NOT NULL,
	running_ts INT,
	stop_ts INT,
	exit_ts INT,
	exit_code INT
);
