pub mod types;

// See module.traefik_job resources
pub const RESERVE_LB_MEMORY_MIB: u32 = 512;
// NOTE: We don't reserve CPU because Nomad is running as a higher priority process than the rest and
// shouldn't be doing much heavy lifting.
pub const NOMAD_RESERVE_MEMORY_MIB: u32 = 512;
pub const PEGBOARD_CONTAINER_RESERVE_MEMORY_MIB: u32 = 32;
pub const PEGBOARD_ISOLATE_RESERVE_MEMORY_MIB: u32 = 64;

pub const LINODE_CPU_PER_CORE: u32 = 1999;
pub const LINODE_DISK_PER_CORE: u32 = 8192;
