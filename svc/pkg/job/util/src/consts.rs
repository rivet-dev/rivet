// Port ranges for the load balancer hosts
//
// Also see lib/bolt/core/src/dep/terraform/pools.rs and
// lib/bolt/core/src/dep/terraform/install_scripts/mod.rs
pub const MIN_INGRESS_PORT_TCP: u16 = 20000;
pub const MAX_INGRESS_PORT_TCP: u16 = 31999;
pub const MIN_INGRESS_PORT_UDP: u16 = 20000;
pub const MAX_INGRESS_PORT_UDP: u16 = 31999;
