use crate::generated::rivet_cluster_config_v1;

/// `rivet/cluster/config`. Every worker, stateless ones included, reads it directly, and
/// `Topology::version_for` counts only storage workers. After release, a new version ships in
/// two deploys: the first binary reads it but still writes the previous version, and only the
/// second writes it. Skipping the first deploy is unsupported.
pub enum RivetClusterConfigData {
	V1(rivet_cluster_config_v1::Data),
}

impl_versioned_v1!(RivetClusterConfigData, rivet_cluster_config_v1::Data);
