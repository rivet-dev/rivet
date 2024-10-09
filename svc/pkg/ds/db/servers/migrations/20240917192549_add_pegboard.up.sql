CREATE TABLE servers_pegboard (
	server_id UUID PRIMARY KEY REFERENCES servers,
	pegboard_container_id UUID NOT NULL,
	pegboard_client_id STRING,

	INDEX (pegboard_container_id)
);
