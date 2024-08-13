CREATE TABLE servers (
	server_id UUID PRIMARY KEY,
	game_id UUID NOT NULL,
	datacenter_id UUID NOT NULL,
	-- The server will be locked to a certain cluster, but a game might change
	-- clusters, and therefore the server will be moved to a new cluster.
	cluster_id UUID NOT NULL,
	-- This represents a map<string, json>
	tags JSONB NOT NULL,
	resources_cpu_millicores INT NOT NULL,
	resources_memory_mib INT NOT NULL,
	kill_timeout_ms INT NOT NULL,
	
	create_ts INT NOT NULL,
	start_ts INT,
	stop_ts INT,
	finish_ts INT,
	cleanup_ts INT,
	-- When the server was marked to be deleted by Rivet
	destroy_ts INT,

	-- Docker
	image_id UUID NOT NULL,
	args STRING[],
	network_mode INT NOT NULL, -- rivet.backend.ds.DockerNetworkMode
	-- This is a map<string, string>
	environment JSONB NOT NULL,
	
	INDEX (game_id)
);


CREATE TABLE docker_ports_protocol_game_guard (  
    server_id UUID NOT NULL REFERENCES servers,
    port_name STRING NOT NULL,
	port_number INT NOT NULL,
	gg_port INT NOT NULL,
    protocol INT NOT NULL, -- rivet.backend.ds.GameGuardProtocol

    PRIMARY KEY (server_id, port_name)
);

CREATE TABLE docker_ports_host (  
	server_id UUID NOT NULL REFERENCES servers,
	port_name STRING NOT NULL,
	port_number INT NOT NULL,
    protocol INT NOT NULL, -- rivet.backend.ds.HostProtocol

	PRIMARY KEY (server_id, port_name)
);

-- TODO make all nomad stucc clear
CREATE TABLE server_nomad (
	server_id UUID PRIMARY KEY REFERENCES servers,
	nomad_dispatched_job_id STRING,
	nomad_alloc_id STRING,
	nomad_node_id STRING,
	nomad_alloc_plan_ts INT,
	nomad_alloc_state JSONB,
	nomad_eval_plan_ts INT,
	nomad_node_name STRING,
	nomad_node_public_ipv4 STRING,
	nomad_node_vlan_ipv4 STRING,

	INDEX (nomad_dispatched_job_id)
);


CREATE TABLE internal_ports (
	server_id UUID NOT NULL REFERENCES servers,
	nomad_label STRING NOT NULL,
	nomad_ip STRING NOT NULL,
	nomad_source INT NOT NULL,

	PRIMARY KEY (server_id, nomad_label)
);
