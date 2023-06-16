CREATE TABLE runs (
	run_id UUID PRIMARY KEY,
	region_id UUID NOT NULL,
	create_ts INT NOT NULL,
	start_ts INT,
	stop_ts INT,
	finish_ts INT,
	cleanup_ts INT,
	token_session_id UUID NOT NULL
);

CREATE TABLE run_networks (
	run_id UUID NOT NULL REFERENCES runs,
	ip STRING NOT NULL,
	mode STRING NOT NULL,
	PRIMARY KEY (run_id, ip, mode),
	INDEX (run_id)
);

CREATE TABLE run_ports (
	run_id UUID NOT NULL REFERENCES runs,
	label STRING NOT NULL,
	ip STRING NOT NULL,
	source INT NOT NULL,
	target INT NOT NULL,
	PRIMARY KEY (run_id, label)
);

CREATE TABLE run_meta_nomad (
	run_id UUID PRIMARY KEY REFERENCES runs,
	dispatched_job_id STRING,
	alloc_id STRING,
	node_id STRING,
	alloc_plan_ts INT,
	alloc_state JSONB,
	eval_plan_ts INT,
	INDEX (dispatched_job_id)
);

CREATE TABLE run_proxied_ports (
	run_id UUID NOT NULL REFERENCES runs,
	target_nomad_port_label STRING,
	ingress_port INT NOT NULL,
	-- Sorted alphabetically before insertion
	ingress_hostnames STRING[] NOT NULL,
	-- Sorted comma concatenated hostnames that can be used in the PK
	ingress_hostnames_str STRING NOT NULL,
	proxy_protocol INT NOT NULL,
	ssl_domain_mode INT NOT NULL,
	PRIMARY KEY (run_id, target_nomad_port_label, ingress_port, ingress_hostnames_str, proxy_protocol)
);

