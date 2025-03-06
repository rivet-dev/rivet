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
