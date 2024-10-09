CREATE TABLE servers_pegboard (
	server_id UUID PRIMARY KEY REFERENCES servers,
	pegboard_container_id UUID NOT NULL

	INDEX (pegboard_container_id)
);

-- Agnostify
ALTER TABLE internal_ports
	RENAME COLUMN nomad_label TO label,
	RENAME COLUMN nomad_ip TO ip,
	RENAME COLUMN nomad_source TO source;
