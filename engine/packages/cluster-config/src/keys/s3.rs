#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, CLUSTER, CONFIG], value = vbare(rivet_data::versioned::RivetClusterConfigData, rivet_data::RIVET_CLUSTER_CONFIG_VERSION))]
pub struct ClusterConfigKey {}
