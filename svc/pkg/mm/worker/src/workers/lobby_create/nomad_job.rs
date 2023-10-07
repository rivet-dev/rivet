use chirp_worker::prelude::*;
use proto::backend::{
	self,
	matchmaker::lobby_runtime::{NetworkMode as LobbyRuntimeNetworkMode, ProxyProtocol},
};
use serde_json::json;
use std::collections::HashMap;

/// What a port is being pointed at.
enum PortTarget {
	Single(u16),
	Range { min: u16, max: u16 },
}

impl PortTarget {
	/// Returns the port to be passed to Nomad's `dynamic_ports` config.
	///
	/// This will return `None` if a port range is provided where `min` and
	/// `max` are not the same.
	fn get_nomad_port(&self) -> Option<u16> {
		match self {
			PortTarget::Single(x) => Some(*x),
			PortTarget::Range { min, max } => {
				if min == max {
					Some(*min)
				} else {
					None
				}
			}
		}
	}
}

/// Helper structure for parsing all of the runtime's ports before building the
/// config.
struct DecodedPort {
	label: String,
	nomad_port_label: String,
	target: PortTarget,
	proxy_protocol: ProxyProtocol,
}

pub fn gen_lobby_docker_job(
	runtime: &backend::matchmaker::lobby_runtime::Docker,
	image_tag: &str,
	tier: &backend::region::Tier,
	lobby_config_json: Option<&String>,
) -> GlobalResult<nomad_client::models::Job> {
	// IMPORTANT: This job spec must be deterministic. Do not pass in parameters
	// that change with every run, such as the lobby ID. Ensure the
	// `reuse_job_id` test passes when changing this function.
	use nomad_client::models::*;

	let network_mode =
		internal_unwrap_owned!(LobbyRuntimeNetworkMode::from_i32(runtime.network_mode));

	let decoded_ports = runtime
		.ports
		.iter()
		.map(|port| {
			let target = if let Some(target_port) = port.target_port {
				PortTarget::Single(target_port as u16)
			} else if let Some(port_range) = &port.port_range {
				PortTarget::Range {
					min: port_range.min as u16,
					max: port_range.max as u16,
				}
			} else {
				internal_panic!("must have either target_port or port_range");
			};

			GlobalResult::Ok(DecodedPort {
				label: port.label.clone(),
				nomad_port_label: util_mm::format_nomad_port_label(&port.label),
				target,
				proxy_protocol: internal_unwrap_owned!(ProxyProtocol::from_i32(
					port.proxy_protocol
				)),
			})
		})
		.collect::<GlobalResult<Vec<DecodedPort>>>()?;

	let dynamic_ports = decoded_ports
		.iter()
		.filter_map(|port| {
			port.target.get_nomad_port().map(|target_port| Port {
				label: Some(port.nomad_port_label.clone()),
				to: match network_mode {
					// Networks are isolated in bridge networking, so we can
					// depend on a consistent target port.
					LobbyRuntimeNetworkMode::Bridge => Some(target_port as i32),

					// We can't use a target port with host networking.
					// Services need to use `PORT_<label>`.
					LobbyRuntimeNetworkMode::Host => None,
				},
				..Port::new()
			})
		})
		.collect::<Vec<_>>();

	let docker_ports = decoded_ports
		.iter()
		.filter(|port| port.target.get_nomad_port().is_some())
		.map(|port| port.nomad_port_label.clone())
		.collect::<Vec<_>>();

	// Also see util_mm:consts::DEFAULT_ENV_KEYS
	let env = runtime
		.env_vars
		.iter()
		.map(|v| {
			(
				v.key.clone(),
				// Escape all interpolation
				v.value.replace('$', "$$"),
			)
		})
		.chain(lobby_config_json.map(|config| {
			(
				"RIVET_LOBBY_CONFIG".to_string(),
				// Escape all interpolation
				config.replace('$', "$$"),
			)
		}))
		.chain(
			[
				("RIVET_CHAT_API_URL", "api-chat"),
				("RIVET_GROUP_API_URL", "api-group"),
				("RIVET_IDENTITY_API_URL", "api-identity"),
				("RIVET_KV_API_URL", "api-kv"),
				("RIVET_MATCHMAKER_API_URL", "api-matchmaker"),
			]
			.iter()
			.map(|(env, service)| {
				(
					env.to_string(),
					format!("{}/{service}", util::env::origin_api()),
				)
			}),
		)
		.chain(
			[
				("RIVET_NAMESPACE_NAME", "${NOMAD_META_NAMESPACE_NAME}"),
				("RIVET_NAMESPACE_ID", "${NOMAD_META_NAMESPACE_ID}"),
				("RIVET_VERSION_NAME", "${NOMAD_META_VERSION_NAME}"),
				("RIVET_VERSION_ID", "${NOMAD_META_VERSION_ID}"),
				("RIVET_GAME_MODE_ID", "${NOMAD_META_LOBBY_GROUP_ID}"),
				("RIVET_GAME_MODE_NAME", "${NOMAD_META_LOBBY_GROUP_NAME}"),
				("RIVET_LOBBY_ID", "${NOMAD_META_LOBBY_ID}"),
				("RIVET_TOKEN", "${NOMAD_META_LOBBY_TOKEN}"),
				("RIVET_REGION_ID", "${NOMAD_META_REGION_ID}"),
				("RIVET_REGION_NAME", "${NOMAD_META_REGION_NAME}"),
				(
					"RIVET_MAX_PLAYERS_NORMAL",
					"${NOMAD_META_MAX_PLAYERS_NORMAL}",
				),
				(
					"RIVET_MAX_PLAYERS_DIRECT",
					"${NOMAD_META_MAX_PLAYERS_DIRECT}",
				),
				("RIVET_MAX_PLAYERS_PARTY", "${NOMAD_META_MAX_PLAYERS_PARTY}"),
				// DEPRECATED:
				("RIVET_LOBBY_TOKEN", "${NOMAD_META_LOBBY_TOKEN}"),
				("RIVET_LOBBY_GROUP_ID", "${NOMAD_META_LOBBY_GROUP_ID}"),
				("RIVET_LOBBY_GROUP_NAME", "${NOMAD_META_LOBBY_GROUP_NAME}"),
			]
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string())),
		)
		// Ports
		.chain(decoded_ports.iter().filter_map(|port| {
			if port.target.get_nomad_port().is_some() {
				Some((
					format!("PORT_{}", port.label),
					format!("${{NOMAD_PORT_{}}}", port.nomad_port_label),
				))
			} else {
				None
			}
		}))
		// Port ranges
		.chain(
			decoded_ports
				.iter()
				.filter_map(|port| {
					if let PortTarget::Range { min, max } = &port.target {
						Some([
							(format!("PORT_RANGE_MIN_{}", port.label), min.to_string()),
							(format!("PORT_RANGE_MAX_{}", port.label), max.to_string()),
						])
					} else {
						None
					}
				})
				.flatten(),
		)
		.collect::<HashMap<String, String>>();

	let services = decoded_ports
		.iter()
		.map(|port| {
			if port.target.get_nomad_port().is_some() {
				let service_name = format!("${{NOMAD_META_LOBBY_ID}}-{}", port.label);
				GlobalResult::Ok(Some(Service {
					provider: Some("nomad".into()),
					ID: Some(service_name.clone()),
					name: Some(service_name),
					tags: Some(vec!["game".into()]),
					port_label: Some(port.nomad_port_label.clone()),
					..Service::new()
				}))
			} else {
				Ok(None)
			}
		})
		.filter_map(|x| x.transpose())
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Job {
		_type: Some("batch".into()),
		constraints: Some(vec![Constraint {
			l_target: Some("${node.class}".into()),
			r_target: Some("job".into()),
			operand: Some("=".into()),
		}]),
		parameterized_job: Some(Box::new(ParameterizedJobConfig {
			payload: Some("forbidden".into()),
			meta_required: Some(vec![
				"image_artifact_url".into(),
				"namespace_id".into(),
				"namespace_name".into(),
				"version_id".into(),
				"version_name".into(),
				"lobby_group_id".into(),
				"lobby_group_name".into(),
				"lobby_id".into(),
				"lobby_token".into(),
				"region_id".into(),
				"region_name".into(),
				"max_players_normal".into(),
				"max_players_direct".into(),
				"max_players_party".into(),
			]),
			meta_optional: None,
		})),
		task_groups: Some(vec![TaskGroup {
			name: Some(util_job::GAME_TASK_NAME.into()),
			constraints: None, // TODO: Use parameter meta to specify the hardware
			affinities: None,  // TODO:
			// Allows for jobs to keep running and receiving players in the
			// event of a disconnection from the Nomad server.
			max_client_disconnect: Some(5 * 60 * 1_000_000_000),
			restart_policy: Some(Box::new(RestartPolicy {
				attempts: Some(0),
				mode: Some("fail".into()),
				..RestartPolicy::new()
			})),
			reschedule_policy: Some(Box::new(ReschedulePolicy {
				attempts: Some(0),
				unlimited: Some(false),
				..ReschedulePolicy::new()
			})),
			networks: Some(vec![NetworkResource {
				mode: match network_mode {
					LobbyRuntimeNetworkMode::Bridge => Some("bridge".into()),
					LobbyRuntimeNetworkMode::Host => Some("host".into()),
				},
				dynamic_ports: Some(dynamic_ports),
				..NetworkResource::new()
			}]),
			// Configure ephemeral disk for logs
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				size_mb: Some(tier.disk as i32),
				..EphemeralDisk::new()
			})),
			tasks: Some(vec![Task {
				name: Some("game".into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("load".into(), json!("image.tar"));
					config.insert("image".into(), json!(image_tag));
					config.insert("args".into(), json!(runtime.args));
					config.insert("ports".into(), json!(docker_ports));
					match network_mode {
						LobbyRuntimeNetworkMode::Bridge => {
							// Don't change this property when using bridge
							// networking, see:
							// https://developer.hashicorp.com/nomad/docs/drivers/docker?optInFrom=nomad-io#network_mode
						}
						LobbyRuntimeNetworkMode::Host => {
							config.insert("network_mode".into(), json!("host"));
						}
					}

					config
				}),
				artifacts: Some(vec![TaskArtifact {
					getter_source: Some("${NOMAD_META_IMAGE_ARTIFACT_URL}".into()),
					getter_mode: Some("file".into()),
					getter_options: Some({
						let mut opts = HashMap::new();
						// Disable automatic unarchiving, see https://www.nomadproject.io/docs/job-specification/artifact#download-and-unarchive
						opts.insert("archive".into(), "false".into());
						opts
					}),
					relative_dest: Some("local/image.tar".into()),
				}]),
				env: Some(env),
				services: Some(services),
				resources: Some(Box::new(Resources {
					// TODO: Configure this per-provider
					CPU: if tier.rivet_cores_numerator < tier.rivet_cores_denominator {
						Some(tier.cpu as i32 - util_job::TASK_CLEANUP_CPU)
					} else {
						None
					},
					cores: if tier.rivet_cores_numerator >= tier.rivet_cores_denominator {
						Some((tier.rivet_cores_numerator / tier.rivet_cores_denominator) as i32)
					} else {
						None
					},
					memory_mb: Some(tier.memory as i32 - util_job::TASK_CLEANUP_MEMORY),
					// Allow oversubscribing memory by 50% of the reserved
					// memory if using less than the node's total memory
					memory_max_mb: Some(tier.memory_max as i32 - util_job::TASK_CLEANUP_MEMORY),
					disk_mb: Some(tier.disk as i32), // TODO: Is this deprecated?
					..Resources::new()
				})),
				// Gives the game processes time to shut down gracefully.
				kill_timeout: Some(60 * 1_000_000_000),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(4),
					max_file_size_mb: Some(4),
				})),
				..Task::new()
			}]),
			..TaskGroup::new()
		}]),
		..Job::new()
	})
}
