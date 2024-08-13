
CREATE INDEX ON db_dynamic_servers.servers (datacenter_id, stop_ts);

CREATE INDEX ON db_dynamic_servers.server_nomad (nomad_dispatched_job_id) STORING (nomad_alloc_plan_ts);
DROP INDEX db_dynamic_servers.server_nomad@server_nomad_nomad_dispatched_job_id_idx;

CREATE INDEX ON db_dynamic_servers.server_nomad (nomad_dispatched_job_id)
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
DROP INDEX db_dynamic_servers.server_nomad@server_nomad_nomad_dispatched_job_id_idx;
