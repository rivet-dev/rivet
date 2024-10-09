-- Backfill
UPDATE datacenters
SET provider = 0
WHERE provider2 = '"Linode"';

UPDATE datacenters
SET build_delivery_method = 0
WHERE build_delivery_method2 = '"TrafficServer"';

UPDATE datacenters
SET build_delivery_method = 1
WHERE build_delivery_method2 = '"S3Direct"';

ALTER TABLE datacenters
	DROP COLUMN provider2,
	DROP COLUMN build_delivery_method2;

UPDATE servers
SET pool_type = 0
WHERE pool_type2 = '"Job"';
UPDATE servers
SET pool_type = 1
WHERE pool_type2 = '"Gg"';
UPDATE servers
SET pool_type = 2
WHERE pool_type2 = '"Ats"';

ALTER TABLE servers
	DROP COLUMN pool_type2;

UPDATE datacenter_tls
SET state = 0
WHERE state2 = '"Creating"';
UPDATE datacenter_tls
SET state = 1
WHERE state2 = '"Active"';
UPDATE datacenter_tls
SET state = 2
WHERE state2 = '"Renewing"';

ALTER TABLE datacenter_tls
	DROP COLUMN state2;
