pub mod types;

// See module.traefik_job resources
pub const RESERVE_LB_MEMORY: u32 = 512;
// NOTE: We don't reserve CPU because Nomad is running as a higher priority process than the rest and
// shouldn't be doing much heavy lifting.
pub const NOMAD_RESERVE_MEMORY: u32 = 512;
pub const PEGBOARD_RESERVE_MEMORY: u32 = 0;

pub const CPU_PER_CORE: u32 = 1999;
pub const DISK_PER_CORE: u32 = 8192;
