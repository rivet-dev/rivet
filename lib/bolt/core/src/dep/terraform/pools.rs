// TODO: Move this file to a common place, since this isn't specific to Terraform

use anyhow::Result;
use derive_builder::Builder;
use maplit::hashmap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{
	config::service::RuntimeKind,
	context::ProjectContext,
	dep::{
		self,
		terraform::nebula_firewall_rules::{
			self, Rule as NebulaFirewallRule, RuleProtocol as NebulaFirewallRuleProtocol,
		},
	},
};

// https://www.cloudflare.com/ips-v4
const CLOUDFLARE_IPV4_RANGES: &[&str] = &[
	"173.245.48.0/20",
	"103.21.244.0/22",
	"103.22.200.0/22",
	"103.31.4.0/22",
	"141.101.64.0/18",
	"108.162.192.0/18",
	"190.93.240.0/20",
	"188.114.96.0/20",
	"197.234.240.0/22",
	"198.41.128.0/17",
	"162.158.0.0/15",
	"104.16.0.0/13",
	"104.24.0.0/14",
	"172.64.0.0/13",
	"131.0.72.0/22",
];

// https://www.cloudflare.com/ips-v6
const CLOUDFLARE_IPV6_RANGES: &[&str] = &[
	"2400:cb00::/32",
	"2606:4700::/32",
	"2803:f800::/32",
	"2405:b500::/32",
	"2405:8100::/32",
	"2a06:98c0::/29",
	"2c0f:f248::/32",
];

/// What to do with this pool when running locally.
#[derive(Clone, PartialEq)]
#[allow(unused)]
pub enum PoolLocalMode {
	/// Don't enable this pool when running locally.
	Disable,

	/// Run the pool locally on the main machine.
	Locally,

	/// Run this only when developing locally.
	LocalOnly,

	/// Treat the pool normally.
	Keep,
}

#[derive(Serialize, Clone, Builder)]
#[builder(setter(into))]
pub struct Pool {
	/// Salt roles applied to this pool.
	roles: Vec<&'static str>,

	/// If this pool is part of the VPC.
	pub vpc: bool,
	#[serde(skip_serializing)]
	local_mode: PoolLocalMode,

	/// Volumes attached to this node.
	#[builder(default)]
	volumes: HashMap<String, PoolVolume>,

	/// Cloudflare tunnels to expose for this node.
	#[builder(default)]
	tunnels: Option<HashMap<String, PoolTunnel>>,

	/// Cloud-based firewall rules to apply to this node.
	///
	/// Additional firewall rules are applied by Terraform depending on the use case.
	#[builder(default)]
	firewall_inbound: Vec<FirewallRule>,

	/// Nebula firewall rules to apply to this node.
	nebula_firewall_inbound: Vec<NebulaFirewallRule>,

	/// What redis databases to run on this instance.
	///
	/// This is defined in pools because some situations require multiple Redis instnaces to run on
	/// one machine.
	#[builder(default)]
	redis_dbs: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct PoolTunnel {
	name: String,
	service: String,
	access_groups: Vec<String>,
	service_tokens: Vec<String>,
	app_launcher: bool,
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
	let access = match &ctx.ns().dns.provider {
		bolt_config::ns::DnsProvider::Cloudflare { access, .. } => access.as_ref(),
	};

	let mut pools = HashMap::<String, Pool>::new();

	pools.insert(
		"leader".into(),
		PoolBuilder::default()
			.roles(vec!["nomad-server", "consul-server"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.tunnels(hashmap! {
				"consul".into() => PoolTunnel {
					name: "Consul".into(),
					service: "http://127.0.0.1:8500".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.bolt.clone()]).unwrap_or_default(),
					app_launcher: true,
				},
				"nomad".into() => PoolTunnel {
					name: "Nomad".into(),
					service: "http://127.0.0.1:4646".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.terraform_nomad.clone(), x.services.bolt.clone()]).unwrap_or_default(),
					app_launcher: true,
				},
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::nomad(),
					vec![
						// Consul RPC
						NebulaFirewallRule::group(
							"role:consul-server",
							NebulaFirewallRuleProtocol::TCP,
							"8300",
						),
						NebulaFirewallRule::group(
							"role:consul-client",
							NebulaFirewallRuleProtocol::TCP,
							"8300",
						),
						// Nomad RPC
						NebulaFirewallRule::group(
							"role:nomad-server",
							NebulaFirewallRuleProtocol::TCP,
							"4647",
						),
						NebulaFirewallRule::group(
							"role:nomad-client",
							NebulaFirewallRuleProtocol::TCP,
							"4647",
						),
					],
				]
				.concat(),
			)
			.build()?,
	);

	let mut svc_roles = vec!["docker", "nomad-client", "consul-client"];
	if ctx.ns().logging.is_some() {
		svc_roles.extend(["traefik", "cloudflare-proxy", "docker-plugin-loki"]);
	}
	pools.insert(
		"svc".into(),
		PoolBuilder::default()
			.roles(svc_roles)
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::nomad(),
					nebula_firewall_rules::nomad_dynamic(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"nats".into(),
		PoolBuilder::default()
			.roles(vec!["nats-server", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.tunnels(hashmap! {
				"nats-client".into() => PoolTunnel {
					name: "NATS Client".into(),
					service: "tcp://__NEBULA_IPV4__:4222".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.bolt.clone()]).unwrap_or_default(),
					app_launcher: false,
				}
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::nats(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"ats".into(),
		PoolBuilder::default()
			.roles(vec!["traffic-server", "consul-client"])
			.vpc(true)
			// TODO: We need a new PoolLocalMode for running locally and being able to run remote
			// nodes
			.local_mode(PoolLocalMode::Locally)
			.volumes(hashmap! {
				"ats".into() => PoolVolume {},
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::traffic_server(),
				]
				.concat(),
			)
			.build()?,
	);

	// Create a pool for each Redis server, since each Redis node needs independent servers in
	// production.
	for redis_dep in ctx.all_services().await {
		if !matches!(redis_dep.config().runtime, RuntimeKind::Redis { .. }) {
			continue;
		}

		let name = redis_dep.name();
		let port = dep::redis::server_port(redis_dep);

		pools.insert(
			dep::redis::pool_name(&redis_dep),
			PoolBuilder::default()
				.roles(vec!["redis", "consul-client", "docker"])
				.vpc(true)
				.local_mode(PoolLocalMode::Locally)
				.tunnels(hashmap! {
					format!("{name}") => PoolTunnel {
						name: format!("Redis ({name})"),
						service: format!("tcp://127.0.0.1:{port}"),
						access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
						service_tokens: access.map(|x| vec![x.services.bolt.clone()]).unwrap_or_default(),
						app_launcher: false,
					}
				})
				.nebula_firewall_inbound(
					[
						nebula_firewall_rules::common(),
						nebula_firewall_rules::consul(),
						nebula_firewall_rules::redis(port),
					]
					.concat(),
				)
				.redis_dbs(vec![redis_dep.name()])
				.build()?,
		);
	}

	pools.insert(
		"crdb".into(),
		PoolBuilder::default()
			.roles(vec!["cockroach", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.volumes(hashmap! {
				"crdb".into() => PoolVolume {},
			})
			.tunnels(hashmap! {
				"cockroach-sql".into() => PoolTunnel {
					name: "CockroachDB SQL".into(),
					service: "tcp://__NEBULA_IPV4__:26257".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.bolt.clone()]).unwrap_or_default(),
					app_launcher: false,
				},
				"cockroach-http".into() => PoolTunnel {
					name: "CockroachDB HTTP".into(),
					service: "http://__NEBULA_IPV4__:26300".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: true,
				}
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::cockroach(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"ch".into(),
		PoolBuilder::default()
			.roles(vec!["clickhouse".into(), "consul-client".into()])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.volumes(hashmap! {
				"ch".into() => PoolVolume {},
			})
			.tunnels(hashmap! {
				"clickhouse-http".into() => PoolTunnel {
					name: "ClickHouse HTTP".into(),
					service: "http://__NEBULA_IPV4__:8123".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: true,
				},
				"clickhouse-tcp".into() => PoolTunnel {
					name: "ClickHouse TCP".into(),
					service: "tcp://__NEBULA_IPV4__:9000".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: false,
				}
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::clickhouse(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"prm-svc".into(),
		PoolBuilder::default()
			.roles(vec!["prometheus", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.volumes(hashmap! {
				"prm".into() => PoolVolume {},
			})
			.tunnels(hashmap! {
				"prometheus-svc".into() => PoolTunnel {
					name: format!("Prometheus Services"),
					service: "http://__NEBULA_IPV4__:9090".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.grafana_cloud.clone()]).unwrap_or_default(),
					app_launcher: true,
				}
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::prometheus(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"prm-job".into(),
		PoolBuilder::default()
			.roles(vec!["prometheus", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.volumes(hashmap! {
				"db".into() => PoolVolume {},
			})
			.tunnels(hashmap! {
				"prometheus-job".into() => PoolTunnel {
					name: "Prometheus Jobs".into(),
					service: "http://__NEBULA_IPV4__:9090".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.bolt.clone(), x.services.grafana_cloud.clone()]).unwrap_or_default(),
					app_launcher: true,
				}
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::prometheus(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"min".into(),
		PoolBuilder::default()
			.roles(vec!["minio", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::LocalOnly)
			.tunnels(hashmap! {
				"minio-server".into() => PoolTunnel {
					name: "Minio Server".into(),
					service: "http://127.0.0.1:9200".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: true,
				},
				"minio-console".into() => PoolTunnel {
					name: "Minio Console".into(),
					service: "http://127.0.0.1:9201".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: true,
				},
			})
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::minio(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"ing-px".into(),
		PoolBuilder::default()
			.roles(vec!["traefik", "ingress-proxy", "consul-client"])
			.vpc(true)
			.local_mode(PoolLocalMode::Locally)
			.tunnels(hashmap! {
				"ing-px".into() => PoolTunnel {
					name: "Traefik Proxied".into(),
					service: "http://__NEBULA_IPV4__:9980".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: access.map(|x| vec![x.services.bolt.clone(), x.services.grafana_cloud.clone()]).unwrap_or_default(),
					app_launcher: true,
				}
			})
			.firewall_inbound(vec![
				FirewallRule {
					label: "http-tcp".into(),
					ports: "80".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: CLOUDFLARE_IPV4_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
					inbound_ipv6_cidr: CLOUDFLARE_IPV6_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
				},
				FirewallRule {
					label: "http-udp".into(),
					ports: "80".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: CLOUDFLARE_IPV4_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
					inbound_ipv6_cidr: CLOUDFLARE_IPV6_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
				},
				FirewallRule {
					label: "https-tcp".into(),
					ports: "443".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: CLOUDFLARE_IPV4_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
					inbound_ipv6_cidr: CLOUDFLARE_IPV6_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
				},
				FirewallRule {
					label: "https-udp".into(),
					ports: "443".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: CLOUDFLARE_IPV4_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
					inbound_ipv6_cidr: CLOUDFLARE_IPV6_RANGES
						.iter()
						.map(ToString::to_string)
						.collect(),
				},
			])
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::traefik(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"ing-job".into(),
		PoolBuilder::default()
			.roles(vec!["traefik", "ingress-proxy"])
			.vpc(false)
			.local_mode(PoolLocalMode::Keep)
			.tunnels(hashmap! {
				"ing-job".into() => PoolTunnel {
					name: "Traefik Job".into(),
					service: "http://__NEBULA_IPV4__:9980".into(),
					access_groups: access.map(|x| vec![x.groups.engineering.clone()]).unwrap_or_default(),
					service_tokens: vec![],
					app_launcher: true,
				}
			})
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
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::consul(),
					nebula_firewall_rules::traefik(),
				]
				.concat(),
			)
			.build()?,
	);

	pools.insert(
		"job".into(),
		PoolBuilder::default()
			.roles(vec!["docker", "nomad-client"])
			.vpc(false)
			.local_mode(PoolLocalMode::Keep)
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
			.nebula_firewall_inbound(
				[
					nebula_firewall_rules::common(),
					nebula_firewall_rules::nomad(),
					vec![
						NebulaFirewallRule::group(
							"role:ing-job",
							NebulaFirewallRuleProtocol::UDP,
							"20000-31999",
						),
						NebulaFirewallRule::group(
							"role:ing-job",
							NebulaFirewallRuleProtocol::TCP,
							"20000-31999",
						),
					],
				]
				.concat(),
			)
			.build()?,
	);

	let pools = filter_pools(ctx, pools)?;

	Ok(pools)
}

/// Processes pool map based on `PoolLocalMode` to create the final pool configs.
///
/// If running locally, will merge all pools in to a "local" pool.
fn filter_pools(
	ctx: &ProjectContext,
	pools: HashMap<String, Pool>,
) -> Result<HashMap<String, Pool>> {
	// Apply filters
	match &ctx.ns().cluster.kind {
		// Return pools that run in a cluster
		bolt_config::ns::ClusterKind::Distributed { .. } => Ok(pools
			.into_iter()
			.filter(|(_, x)| x.local_mode != PoolLocalMode::LocalOnly)
			.collect()),

		// Merge pools together to `local` node
		bolt_config::ns::ClusterKind::SingleNode { .. } => {
			let mut new_pools = HashMap::new();

			// Include normal pools
			new_pools.extend(
				pools
					.iter()
					.filter(|(_, x)| x.local_mode == PoolLocalMode::Keep)
					.map(|(k, v)| (k.clone(), v.clone())),
			);

			// Create local pool for the main node
			let local_pools = pools
				.iter()
				.filter(|(_, x)| {
					x.local_mode == PoolLocalMode::Locally
						|| x.local_mode == PoolLocalMode::LocalOnly
				})
				.map(|(_, x)| x)
				.collect::<Vec<_>>();

			// Build pool
			let mut pool = Pool {
				// Aggregate all roles and deduplicate them
				roles: local_pools
					.iter()
					.flat_map(|x| x.roles.iter().cloned())
					.collect::<HashSet<&str>>()
					.into_iter()
					.collect::<Vec<_>>(),
				vpc: true,
				local_mode: PoolLocalMode::Keep,
				// Aggregate all volumes.
				volumes: local_pools
					.iter()
					.flat_map(|x| x.volumes.iter())
					.map(|(k, v)| (k.clone(), v.clone()))
					.collect(),
				// Aggregate all tunnels
				tunnels: Some(
					local_pools
						.iter()
						.filter_map(|x| x.tunnels.as_ref())
						.flat_map(|x| x.iter())
						.map(|(k, v)| (k.clone(), v.clone()))
						.collect(),
				),
				// Aggregate all firewall rules
				// TODO: Deduplicate rules
				firewall_inbound: local_pools
					.iter()
					.flat_map(|x| x.firewall_inbound.iter())
					.cloned()
					.collect(),
				// Aggregate all Nebula firewall rules
				// TODO: Deduplicate rules
				nebula_firewall_inbound: local_pools
					.iter()
					.flat_map(|x| x.nebula_firewall_inbound.iter())
					.cloned()
					.collect(),
				// Aggregate all Redis databases
				redis_dbs: local_pools
					.iter()
					.flat_map(|x| x.redis_dbs.iter())
					.cloned()
					.collect(),
			};

			// Sort the lists so the value is deterministic
			pool.roles.sort();
			pool.firewall_inbound.sort();
			pool.nebula_firewall_inbound.sort();
			pool.redis_dbs.sort();

			new_pools.insert("local".into(), pool);

			Ok(new_pools)
		}
	}
}
