rivet_keys::s3_key! {
	pub ClusterConfigKey() => [RIVET, CLUSTER, CONFIG];
	value: vbare(rivet_data::versioned::RivetClusterConfigData, rivet_data::RIVET_CLUSTER_CONFIG_VERSION);
}
