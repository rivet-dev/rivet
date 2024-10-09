#[derive(Debug, Clone)]
pub struct Tier {
	pub tier_name_id: String,
	pub rivet_cores_numerator: u32,
	pub rivet_cores_denominator: u32,
	// MHz
	pub cpu: u32,
	// Millicores
	pub cpu_millicores: u32,
	// MiB
	pub memory: u32,
	// MiB
	pub memory_max: u32,
	// MiB
	pub disk: u32,
	// MiB
	pub bandwidth: u32,
}
