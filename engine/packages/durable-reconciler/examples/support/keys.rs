use super::versioned;

rivet_keys::volume_key! {
	pub CounterPartition(id: u64) => [{id}];
}

rivet_keys::volume_kv_key! {
	pub CounterKey(id: u64) => [{id}];
	value: vbare(versioned::CounterData, versioned::VERSION);
}
