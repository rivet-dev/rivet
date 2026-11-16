use super::versioned;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [id])]
pub struct CounterPartition {
	pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [id], value = vbare(versioned::CounterData, versioned::VERSION))]
pub struct CounterKey {
	pub id: u64,
}
