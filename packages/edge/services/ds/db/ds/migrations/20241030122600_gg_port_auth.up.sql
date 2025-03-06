CREATE TABLE server_ports_gg_auth (
	server_id UUID NOT NULL,
	port_name TEXT NOT NULL,
	auth_type INT NOT NULL,
	key TEXT,
	value TEXT NOT NULL,

	FOREIGN KEY (server_id, port_name) REFERENCES server_ports_gg (server_id, port_name),
	PRIMARY KEY (server_id, port_name)
);
