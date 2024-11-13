use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ClientConfig {
	// Millicores
	pub reserved_cpu: u64,
	// MiB
	pub reserved_memory: u64,
}
