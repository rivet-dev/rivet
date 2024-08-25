CREATE INDEX ON server_nomad (nomad_dispatched_job_id) STORING (nomad_alloc_plan_ts);
DROP INDEX server_nomad@server_nomad_nomad_dispatched_job_id_idx;

CREATE INDEX ON server_nomad (nomad_dispatched_job_id)
STORING (
	nomad_alloc_id,
	nomad_node_id,
	nomad_alloc_plan_ts,
	nomad_alloc_state,
	nomad_eval_plan_ts,
	nomad_node_name,
	nomad_node_public_ipv4,
	nomad_node_vlan_ipv4
);
DROP INDEX server_nomad@server_nomad_nomad_dispatched_job_id_idx;

CREATE INDEX ON servers (datacenter_id, stop_ts) STORING (kill_timeout_ms);
