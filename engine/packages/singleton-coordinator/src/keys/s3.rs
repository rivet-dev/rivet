rivet_keys::s3_key! {
	pub CoordinatorLeaseKey() => [RIVET, CLUSTER, COORDINATOR];
	value: vbare(rivet_data::versioned::RivetSingletonCoordinatorData, rivet_data::RIVET_SINGLETON_COORDINATOR_VERSION);
}
