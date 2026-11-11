#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, CLUSTER, COORDINATOR], value = vbare(rivet_data::versioned::RivetSingletonCoordinatorData, rivet_data::RIVET_SINGLETON_COORDINATOR_VERSION))]
pub struct CoordinatorLeaseKey {}
