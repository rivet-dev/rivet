use chirp_workflow::prelude::*;

#[derive(Debug, Clone)]
pub struct Tier {
	pub tier_name_id: String,
	pub rivet_cores_numerator: u32,
	pub rivet_cores_denominator: u32,
	// MHz
	pub cpu: u64,
	// MB
	pub memory: u64,
	// MB
	pub memory_max: u64,
	// MB
	pub disk: u64,
	// MB
	pub bandwidth: u64,
}
