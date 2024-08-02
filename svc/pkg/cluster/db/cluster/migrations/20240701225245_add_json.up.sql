ALTER TABLE datacenters
	ADD COLUMN pools2 JSONB, -- Vec<cluster::types::Pool>
	ADD COLUMN provider2 JSONB, -- cluster::types::Provider
	ADD COLUMN build_delivery_method2 JSONB; -- cluster::types::BuildDeliveryMethod

ALTER TABLE servers
	ADD COLUMN pool_type2 JSONB; -- cluster::types::PoolType

-- No longer needed
DROP TABLE server_images_linode;

ALTER TABLE datacenter_tls
	ADD COLUMN state2 JSONB; -- cluster::types::TlsState

CREATE TABLE server_images2 (
	provider JSONB,
	install_hash TEXT,
	datacenter_id UUID,
	pool_type JSONB,

	create_ts INT NOT NULL,
	-- After the image expires and is destroyed
	destroy_ts INT,
	provider_image_id TEXT,

	PRIMARY KEY (provider, install_hash, datacenter_id, pool_type)
);
