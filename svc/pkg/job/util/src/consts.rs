// Port ranges for the load balancer hosts
//
// Also see tf/infra/firewall_pools.tf for ing-job pool
pub const MIN_INGRESS_PORT_TCP: u16 = 20000;
pub const MAX_INGRESS_PORT_TCP: u16 = 20512; // Needs to be 25999 once
											 // https://linear.app/rivet-gg/issue/RIV-1802
											 // is resolved
pub const MIN_INGRESS_PORT_UDP: u16 = 26000;
pub const MAX_INGRESS_PORT_UDP: u16 = 26512; // Needs to be 31999 once
											 // https://linear.app/rivet-gg/issue/RIV-1802
											 // is resolved
