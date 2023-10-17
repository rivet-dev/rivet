use chirp_worker::prelude::*;
use proto::backend::{
	self,
	matchmaker::lobby_runtime::{NetworkMode as LobbyRuntimeNetworkMode, ProxyProtocol},
};
use regex::Regex;
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

	// The container will set up port forwarding manually from the Nomad-defined ports on the host
	// to the CNI container
	let dynamic_ports = decoded_ports
		.iter()
		.filter_map(|port| {
			port.target.get_nomad_port().map(|target_port| Port {
				label: Some(port.nomad_port_label.clone()),
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
	let mut env = runtime
		.env_vars
		.iter()
		.map(|v| (v.key.clone(), v.value.clone()))
		.chain(lobby_config_json.map(|config| ("RIVET_LOBBY_CONFIG".to_string(), config.clone())))
		.chain([(
			"RIVET_API_ENDPOINT".to_string(),
			util::env::origin_api().to_string(),
		)])
		.chain(
			// DEPRECATED:
			[
				("RIVET_CHAT_API_URL", "chat"),
				("RIVET_GROUP_API_URL", "group"),
				("RIVET_IDENTITY_API_URL", "identity"),
				("RIVET_KV_API_URL", "kv"),
				("RIVET_MATCHMAKER_API_URL", "matchmaker"),
			]
			.iter()
			.filter(|_| util::env::support_deprecated_subdomains())
			.map(|(env, service)| {
				(
					env.to_string(),
					util::env::origin_api().replace("://", &format!("://{}.", service)),
				)
			}),
		)
		.chain(
			[
				(
					"RIVET_NAMESPACE_NAME",
					template_env_var("NOMAD_META_NAMESPACE_NAME"),
				),
				(
					"RIVET_NAMESPACE_ID",
					template_env_var("NOMAD_META_NAMESPACE_ID"),
				),
				(
					"RIVET_VERSION_NAME",
					template_env_var("NOMAD_META_VERSION_NAME"),
				),
				(
					"RIVET_VERSION_ID",
					template_env_var("NOMAD_META_VERSION_ID"),
				),
				(
					"RIVET_GAME_MODE_ID",
					template_env_var("NOMAD_META_LOBBY_GROUP_ID"),
				),
				(
					"RIVET_GAME_MODE_NAME",
					template_env_var("NOMAD_META_LOBBY_GROUP_NAME"),
				),
				("RIVET_LOBBY_ID", template_env_var("NOMAD_META_LOBBY_ID")),
				("RIVET_TOKEN", template_env_var("NOMAD_META_LOBBY_TOKEN")),
				("RIVET_REGION_ID", template_env_var("NOMAD_META_REGION_ID")),
				(
					"RIVET_REGION_NAME",
					template_env_var("NOMAD_META_REGION_NAME"),
				),
				(
					"RIVET_MAX_PLAYERS_NORMAL",
					template_env_var("NOMAD_META_MAX_PLAYERS_NORMAL"),
				),
				(
					"RIVET_MAX_PLAYERS_DIRECT",
					template_env_var("NOMAD_META_MAX_PLAYERS_DIRECT"),
				),
				(
					"RIVET_MAX_PLAYERS_PARTY",
					template_env_var("NOMAD_META_MAX_PLAYERS_PARTY"),
				),
				// DEPRECATED:
				(
					"RIVET_LOBBY_TOKEN",
					template_env_var("NOMAD_META_LOBBY_TOKEN"),
				),
				(
					"RIVET_LOBBY_GROUP_ID",
					template_env_var("NOMAD_META_LOBBY_GROUP_ID"),
				),
				(
					"RIVET_LOBBY_GROUP_NAME",
					template_env_var("NOMAD_META_LOBBY_GROUP_NAME"),
				),
			]
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string())),
		)
		// Ports
		.chain(decoded_ports.iter().filter_map(|port| {
			if port.target.get_nomad_port().is_some() {
				Some((
					format!("PORT_{}", port.label),
					format!("{{{{ env \"NOMAD_PORT_{}\" }}}}", port.nomad_port_label),
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
		.map(|(k, v)| format!("{k}={v}"))
		.collect::<Vec<String>>();
	env.sort();

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
				// The container will set up a CNI network if using bridge networking
				mode: Some("host".into()),
				dynamic_ports: Some(dynamic_ports),
				..NetworkResource::new()
			}]),
			services: Some(services),
			// Configure ephemeral disk for logs
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				size_mb: Some(tier.disk as i32),
				..EphemeralDisk::new()
			})),
			tasks: Some(vec![Task {
				name: Some("game".into()),
				driver: Some("raw_exec".into()),
				config: Some({
					let mut x = HashMap::new();
					x.insert("command".into(), json!("${NOMAD_TASK_DIR}/run.sh"));
					x
				}),
				artifacts: Some(vec![TaskArtifact {
					getter_source: Some("${NOMAD_META_IMAGE_ARTIFACT_URL}".into()),
					getter_mode: Some("file".into()),
					getter_options: Some({
						let mut opts = HashMap::new();
						// Disable automatic unarchiving since the Docker archive needs to be
						// consumed in the original tar format
						opts.insert("archive".into(), "false".into());
						opts
					}),
					relative_dest: Some("${NOMAD_TASK_DIR}/docker-image.tar".into()),
				}]),
				templates: Some(vec![
					Template {
						embedded_tmpl: Some(include_str!("./scripts/run.sh").into()),
						dest_path: Some("${NOMAD_TASK_DIR}/run.sh".into()),
						perms: Some("744".into()),
						..Template::new()
					},
					Template {
						embedded_tmpl: Some(include_str!("./scripts/setup.sh").into()),
						dest_path: Some("${NOMAD_TASK_DIR}/setup.sh".into()),
						perms: Some("744".into()),
						..Template::new()
					},
					Template {
						embedded_tmpl: Some(gen_oci_bundle_config(env)?),
						dest_path: Some("${NOMAD_TASK_DIR}/oci-bundle-config.base.json".into()),
						..Template::new()
					},
				]),
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

/// Build base config used to generate the OCI bundle's config.json.
fn gen_oci_bundle_config(env: Vec<String>) -> GlobalResult<String> {
	// This is a modified version of the default config.json generated by containerd
	//
	// Some values will be overridden at runtime by the values in the OCI bundle's config.json.
	let config = json!({
		"ociVersion": "1.0.2-dev",
		"process": {
			// user, args, and cwd will be injected at runtime

			// Will be merged with the OCI bundle's env
			//
			// These will take priority over the OCI bundle's env
			"env": env,

			"terminal": false,
			"capabilities": {
				"bounding": [
					"CAP_AUDIT_WRITE",
					"CAP_KILL",
					"CAP_NET_BIND_SERVICE"
				],
				"effective": [
					"CAP_AUDIT_WRITE",
					"CAP_KILL",
					"CAP_NET_BIND_SERVICE"
				],
				"permitted": [
					"CAP_AUDIT_WRITE",
					"CAP_KILL",
					"CAP_NET_BIND_SERVICE"
				],
				"ambient": [
					"CAP_AUDIT_WRITE",
					"CAP_KILL",
					"CAP_NET_BIND_SERVICE"
				]
			},
			"rlimits": [
				{
					"type": "RLIMIT_NOFILE",
					"hard": 1024,
					"soft": 1024
				}
			],
			"noNewPrivileges": true
		},
		"root": {
			"path": "rootfs",
			"readonly": true
		},
		"hostname": "rivet-job",
		"mounts": [
			{
				"destination": "/proc",
				"type": "proc",
				"source": "proc"
			},
			{
				"destination": "/dev",
				"type": "tmpfs",
				"source": "tmpfs",
				"options": [
					"nosuid",
					"strictatime",
					"mode=755",
					"size=65536k"
				]
			},
			{
				"destination": "/dev/pts",
				"type": "devpts",
				"source": "devpts",
				"options": [
					"nosuid",
					"noexec",
					"newinstance",
					"ptmxmode=0666",
					"mode=0620",
					"gid=5"
				]
			},
			{
				"destination": "/dev/shm",
				"type": "tmpfs",
				"source": "shm",
				"options": [
					"nosuid",
					"noexec",
					"nodev",
					"mode=1777",
					"size=65536k"
				]
			},
			{
				"destination": "/dev/mqueue",
				"type": "mqueue",
				"source": "mqueue",
				"options": [
					"nosuid",
					"noexec",
					"nodev"
				]
			},
			{
				"destination": "/sys",
				"type": "sysfs",
				"source": "sysfs",
				"options": [
					"nosuid",
					"noexec",
					"nodev",
					"ro"
				]
			},
			{
				"destination": "/sys/fs/cgroup",
				"type": "cgroup",
				"source": "cgroup",
				"options": [
					"nosuid",
					"noexec",
					"nodev",
					"relatime",
					"ro"
				]
			}
		],
		"linux": {
			"resources": {
				"devices": [
					{
						"allow": false,
						"access": "rwm"
					}
				]
			},
			// `network` namespace will be added at runitme including the CNI namespace
			"namespaces": [
				{ "type": "pid" },
				{ "type": "ipc" },
				{ "type": "uts" },
				{ "type": "mount" },
				{ "type": "cgroup" }
			],
			"maskedPaths": [
				"/proc/acpi",
				"/proc/asound",
				"/proc/kcore",
				"/proc/keys",
				"/proc/latency_stats",
				"/proc/timer_list",
				"/proc/timer_stats",
				"/proc/sched_debug",
				"/sys/firmware",
				"/proc/scsi"
			],
			"readonlyPaths": [
				"/proc/bus",
				"/proc/fs",
				"/proc/irq",
				"/proc/sys",
				"/proc/sysrq-trigger"
			]
		}
	});
	let config_str = serde_json::to_string(&config)?;

	// Escape Go template syntax
	let config_str = escape_go_template(&config_str);

	// Replace env vars with Consul template syntax
	let re = Regex::new(r"###ENV:(\w+)###")?;
	let config_str = re.replace_all(
		&config_str,
		r#"{{ env "$1" | regexReplaceAll "\"" "\\\"" }}"#,
	);

	Ok(config_str.to_string())
}

// TODO: Escape uses of `###`
/// Makes user-generated string safe to inject in to a Go template.
fn escape_go_template(input: &str) -> String {
	input
		.replace("{{", r#"{{"{{"}}"#)
		.replace("}}", r#"{{"}}"}}"#)
}

/// Generates a template string that we can substitute with the real environment variable
///
/// This must be safe to inject in to a JSON string so it can be substituted after rendering the
/// JSON object.
fn template_env_var(name: &str) -> String {
	format!("###ENV:{name}###")
}
