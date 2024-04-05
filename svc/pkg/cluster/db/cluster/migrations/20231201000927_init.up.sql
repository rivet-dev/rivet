CREATE TABLE clusters (
    cluster_id UUID PRIMARY KEY,
	name_id TEXT NOT NULL,
	owner_team_id UUID,
	create_ts INT NOT NULL
);

CREATE TABLE datacenters (
    datacenter_id UUID PRIMARY KEY,
	cluster_id UUID NOT NULL REFERENCES clusters (cluster_id),
	name_id TEXT NOT NULL,
	display_name TEXT NOT NULL,
	provider INT NOT NULL,
	provider_datacenter_id TEXT NOT NULL,
	provider_api_token TEXT,
	pools BYTES NOT NULL,
	build_delivery_method INT NOT NULL,
	drain_timeout INT NOT NULL,

	UNIQUE (cluster_id, name_id),
	INDEX (cluster_id)
);

CREATE TABLE servers (
	server_id UUID PRIMARY KEY,
	datacenter_id UUID NOT NULL,
    cluster_id UUID NOT NULL REFERENCES clusters (cluster_id),
	pool_type INT NOT NULL,

	-- Null until actual server is provisioned
	provider_server_id TEXT,
	provider_hardware TEXT,
	vlan_ip TEXT,
	network_idx INT,
	public_ip TEXT,

	-- Null until nomad node successfully registers
	nomad_node_id TEXT,

	create_ts INT NOT NULL,
	nomad_join_ts INT,
	-- Null if not draining
	drain_ts INT,
	-- When the server was marked to be deleted by rivet
	cloud_destroy_ts INT,
	taint_ts INT,

	-- Used when determining which server this ip belongs to
	INDEX (public_ip)
);

-- Stores data for destroying linode resources
CREATE TABLE linode_misc (
	server_id UUID PRIMARY KEY REFERENCES servers (server_id),
	ssh_key_id INT NOT NULL,
	linode_id INT,
	firewall_id INT
);

-- Stores data for destroying cloudflare resources
CREATE TABLE cloudflare_misc (
	server_id UUID PRIMARY KEY REFERENCES servers (server_id),
	dns_record_id TEXT NOT NULL,
	secondary_dns_record_id TEXT
);

CREATE TABLE server_images (
	-- A string denoting what type of image this is (ex. "linode-us-southeast-job")
	variant TEXT PRIMARY KEY,
	create_ts INT NOT NULL,
	image_id TEXT
);

CREATE TABLE server_images_linode_misc (
	variant TEXT PRIMARY KEY,
	ssh_key_id INT NOT NULL,
	linode_id INT,
	firewall_id INT,
	disk_id INT,
	public_ip TEXT,
	image_id TEXT,

	INDEX (public_ip),
	INDEX (image_id)
);
