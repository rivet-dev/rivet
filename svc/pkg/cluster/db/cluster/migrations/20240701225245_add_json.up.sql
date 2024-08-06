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
	provider TEXT, -- cluster::types::Provider
	install_hash TEXT,
	datacenter_id UUID,
	pool_type TEXT, -- cluster::types::PoolType

	create_ts INT NOT NULL,
	-- After the image expires and is destroyed
	destroy_ts INT,
	provider_image_id TEXT,

	PRIMARY KEY (provider, install_hash, datacenter_id, pool_type)
);

-- Backfill
UPDATE datacenters
SET provider2 = '"Linode"'
WHERE provider = 0;

UPDATE servers
SET pool_type2 = '"Job"'
WHERE pool_type = 0;
UPDATE servers
SET pool_type2 = '"Gg"'
WHERE pool_type = 1;
UPDATE servers
SET pool_type2 = '"Ats"'
WHERE pool_type = 2;

UPDATE datacenter_tls
SET state2 = '"Creating"'
WHERE state = 0;
UPDATE datacenter_tls
SET state2 = '"Active"'
WHERE state = 1;
UPDATE datacenter_tls
SET state2 = '"Renewing"'
WHERE state = 2;
