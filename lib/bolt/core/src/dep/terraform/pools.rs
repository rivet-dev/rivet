// TODO: Move this file to a common place, since this isn't specific to Terraform

use anyhow::Result;
use derive_builder::Builder;
use maplit::hashmap;
use serde::Serialize;
use std::{
	collections::{HashMap, HashSet},
	net::Ipv4Addr,
};

use super::net;

use crate::{
	config::service::RuntimeKind,
	context::ProjectContext,
	dep::{self},
};

#[derive(Serialize, Clone, Builder)]
#[builder(setter(into))]
pub struct Pool {
	pub vlan_address: Ipv4Addr,
	pub vlan_prefix_len: u8,

	/// Volumes attached to this node.
	#[builder(default)]
	volumes: HashMap<String, PoolVolume>,

	/// Cloud-based firewall rules to apply to this node.
	///
	/// Additional firewall rules are applied by Terraform depending on the use case.
	#[builder(default)]
	firewall_inbound: Vec<FirewallRule>,
}

#[derive(Serialize, Clone)]
pub struct PoolVolume {}

#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FirewallRule {
	label: String,
	ports: String,
	protocol: String,
	inbound_ipv4_cidr: Vec<String>,
	inbound_ipv6_cidr: Vec<String>,
}

pub async fn build_pools(ctx: &ProjectContext) -> Result<HashMap<String, Pool>> {
	let mut pools = HashMap::<String, Pool>::new();

	pools.insert(
		"gg".into(),
		PoolBuilder::default()
			.vlan_address(net::gg::VLAN_ADDR)
			.vlan_prefix_len(net::gg::VLAN_PREFIX_LEN)
			.firewall_inbound(vec![
				// HTTP(S)
				FirewallRule {
					label: "http-tcp".into(),
					ports: "80".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "http-udp".into(),
					ports: "80".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "https-tcp".into(),
					ports: "443".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "https-udp".into(),
					ports: "443".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				// Dynamic TCP
				FirewallRule {
					label: "dynamic-tcp".into(),
					ports: "20000-20512".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				// Dynamic UDP
				FirewallRule {
					label: "dynamic-udp".into(),
					ports: "26000-26512".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
			])
			.build()?,
	);

	pools.insert(
		"job".into(),
		PoolBuilder::default()
			.vlan_address(net::job::VLAN_ADDR)
			.vlan_prefix_len(net::job::VLAN_PREFIX_LEN)
			// TODO: Add firewall rules for VLAN
			.firewall_inbound(vec![
				// TODO: See below why commented out
				// var.is_prod ? [] : local.firewall_rules.nomad_dynamic_public,

				// TODO: See below why commented out
				// Ports available to Nomad jobs using the host network
				// [
				// 	{
				// 		label = "nomad-host-tcp"
				// 		proto = "tcp"
				// 		ports = [26000, 31999]
				// 		ipv4 = local.firewall_sources.vpc.ipv4
				// 		ipv6 = local.firewall_sources.vpc.ipv6
				// 	},
				// 	{
				// 		label = "nomad-host-udp"
				// 		proto = "udp"
				// 		ports = [26000, 31999]
				// 		ipv4 = local.firewall_sources.vpc.ipv4
				// 		ipv6 = local.firewall_sources.vpc.ipv6
				// 	},
				// ],

				// TODO: Remove this once we have correct firewall rules
				// Allow all dynamic ports from any origin so our ing-job servers can forward these ports
				FirewallRule {
					label: "nomad-dynamic-tcp".into(),
					ports: "20000-31999".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "nomad-dynamic-udp".into(),
					ports: "20000-31999".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
			])
			.build()?,
	);

	pools.insert(
		"ats".into(),
		PoolBuilder::default()
			.vlan_address(net::ats::VLAN_ADDR)
			.vlan_prefix_len(net::ats::VLAN_PREFIX_LEN)
			.build()?,
	);

	Ok(pools)
}
