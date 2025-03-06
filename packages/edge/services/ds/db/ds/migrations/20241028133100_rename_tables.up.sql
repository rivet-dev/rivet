ALTER TABLE docker_ports_protocol_game_guard
	ALTER COLUMN port_number DROP NOT NULL;

ALTER TABLE docker_ports_protocol_game_guard
	RENAME TO server_ports_gg;

ALTER TABLE docker_ports_host
	ALTER COLUMN port_number DROP NOT NULL;

ALTER TABLE docker_ports_host
	RENAME TO server_ports_host;

ALTER TABLE internal_ports
	RENAME TO server_proxied_ports;
