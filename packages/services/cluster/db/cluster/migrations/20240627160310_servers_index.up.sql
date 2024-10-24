-- For cluster-datacenter-topology-get
CREATE INDEX ON servers (cloud_destroy_ts, taint_ts, nomad_node_id) STORING (datacenter_id);