ALTER TABLE run_meta_nomad
	ADD COLUMN node_name STRING,
	ADD COLUMN node_public_ipv4 STRING,
	ADD COLUMN node_vlan_ipv4 STRING;
