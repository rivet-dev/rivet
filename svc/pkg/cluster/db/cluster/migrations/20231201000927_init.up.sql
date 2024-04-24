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
	pools BYTES NOT NULL, -- rivet.backend.pkg.cluster.msg.datacenter_create.Pools
	build_delivery_method INT NOT NULL,
	create_ts INT NOT NULL,

	UNIQUE (cluster_id, name_id),
	INDEX (cluster_id)
);

CREATE TABLE datacenter_tls (
	datacenter_id UUID PRIMARY KEY REFERENCES datacenters (datacenter_id),

	-- Null until TLS cert is fully created. DB record needs to exist to prevent race condition
	gg_cert_pem TEXT,
	gg_private_key_pem TEXT,
	job_cert_pem TEXT,
	job_private_key_pem TEXT,

	state INT NOT NULL,
	expire_ts INT NOT NULL
);

CREATE TABLE servers (
	server_id UUID PRIMARY KEY,
	datacenter_id UUID NOT NULL REFERENCES datacenters (datacenter_id),
	pool_type INT NOT NULL, -- rivet.backend.cluster.PoolType

	-- Null until actual server is provisioned
	provider_server_id TEXT,
	provider_hardware TEXT,
	vlan_ip INET,
	network_idx INT,
	public_ip INET,

	-- Null until nomad node successfully registers
	nomad_node_id TEXT,

	create_ts INT NOT NULL,
	provision_complete_ts INT,
	install_complete_ts INT,
	nomad_join_ts INT,
	-- Null if not draining
	drain_ts INT,
	drain_complete_ts INT,
	-- When the server was marked to be deleted by rivet
	cloud_destroy_ts INT,
	taint_ts INT
);

-- Used when determining which server this ip belongs to
CREATE UNIQUE INDEX idx_servers_public_ip
ON servers (public_ip)
WHERE cloud_destroy_ts IS NULL;

-- Stores data for destroying linode resources
CREATE TABLE servers_linode (
	server_id UUID NOT NULL REFERENCES servers (server_id),
	ssh_key_id INT NOT NULL,
	linode_id INT,
	firewall_id INT,

	destroy_ts INT
);

-- Effectively a conditional primary key
CREATE UNIQUE INDEX idx_servers_linode_pkey
ON servers_linode (server_id)
WHERE destroy_ts IS NULL;

-- Stores data for destroying cloudflare resources
CREATE TABLE servers_cloudflare (
	server_id UUID NOT NULL REFERENCES servers (server_id),
	dns_record_id TEXT,
	-- Secondary DNS route which doesn't have a wildcard. Used for discord activities.
	secondary_dns_record_id TEXT,

	destroy_ts INT
);

-- Effectively a conditional primary key
CREATE UNIQUE INDEX idx_servers_cloudflare_pkey
ON servers_cloudflare (server_id)
WHERE destroy_ts IS NULL;

CREATE TABLE server_images (
	provider INT,
	install_hash TEXT,
	datacenter_id UUID,
	pool_type INT,

	create_ts INT NOT NULL,
	provider_image_id TEXT,

	PRIMARY KEY (provider, install_hash, datacenter_id, pool_type)
);

-- Stores data for destroying linode prebake resources and creating custom images
CREATE TABLE server_images_linode (
	install_hash TEXT,
	datacenter_id UUID,
	pool_type INT,

	ssh_key_id INT NOT NULL,
	linode_id INT,
	firewall_id INT,
	disk_id INT,
	public_ip INET,
	image_id TEXT,

	destroy_ts INT
);

-- Effectively a conditional primary key
CREATE UNIQUE INDEX idx_server_images_linode_pkey
ON server_images_linode (install_hash, datacenter_id, pool_type)
WHERE destroy_ts IS NULL;

CREATE INDEX idx_server_images_linode_public_ip
ON server_images_linode (public_ip)
WHERE destroy_ts IS NULL;

CREATE INDEX idx_server_images_linode_image_id
ON server_images_linode (image_id)
WHERE destroy_ts IS NULL;

-- Dictates which cluster a game's lobbies will be created in
CREATE TABLE games (
	game_id UUID PRIMARY KEY,
	cluster_id UUID NOT NULL
);
