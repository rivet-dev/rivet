use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	process::Stdio,
};

use anyhow::*;
use futures_util::StreamExt;
use indoc::indoc;
use pegboard::protocol;
use rand::Rng;
use serde_json::{json, Value};
use tokio::{
	fs::{self, File},
	io::{AsyncReadExt, AsyncWriteExt},
	process::Command,
};

use super::{oci_config, Actor};
use crate::{ctx::Ctx, utils};

const NETWORK_NAME: &str = "rivet-pegboard";
const MIN_INGRESS_PORT: u16 = 20000;
const MAX_INGRESS_PORT: u16 = 31999;

impl Actor {
	pub async fn download_image(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "downloading artifact");

		let actor_path = ctx.actor_path(self.actor_id);
		let oci_bundle_path = actor_path.join("oci-bundle");

		let mut stream = reqwest::get(&self.config.image.artifact_url)
			.await?
			.error_for_status()?
			.bytes_stream();

		match self.config.image.kind {
			protocol::ImageKind::DockerImage => {
				let docker_image_path = actor_path.join("docker-image.tar");

				match self.config.image.compression {
					protocol::ImageCompression::None => {
						tracing::info!(actor_id=?self.actor_id, "saving artifact to file");

						let mut output_file = File::create(&docker_image_path).await?;

						// Write from stream to file
						while let Some(chunk) = stream.next().await {
							output_file.write_all(&chunk?).await?;
						}
					}
					protocol::ImageCompression::Lz4 => {
						tracing::info!(actor_id=?self.actor_id, "decompressing artifact");

						// Spawn the lz4 process
						let mut lz4_child = Command::new("lz4")
							.arg("-d")
							.arg("-")
							.arg(&docker_image_path)
							.stdin(Stdio::piped())
							.spawn()?;

						// Take the stdin of lz4
						let mut lz4_stdin = lz4_child.stdin.take().context("lz4 stdin")?;

						use std::result::Result::{Err, Ok};
						tokio::try_join!(
							// Pipe the response body to lz4 stdin
							async move {
								while let Some(chunk) = stream.next().await {
									let data = chunk?;
									lz4_stdin.write_all(&data).await?;
								}
								lz4_stdin.shutdown().await?;

								anyhow::Ok(())
							},
							// Wait for child process
							async {
								let cmd_out = lz4_child.wait_with_output().await?;
								ensure!(
									cmd_out.status.success(),
									"failed `lz4` command\n{}",
									std::str::from_utf8(&cmd_out.stderr)?
								);

								Ok(())
							},
						)?;
					}
				}
			}
			protocol::ImageKind::OciBundle => {
				tracing::info!(actor_id=?self.actor_id, "decompressing and unarchiving artifact");

				fs::create_dir(&oci_bundle_path).await?;

				// Spawn the lz4 process
				let mut lz4_child = Command::new("lz4")
					.arg("-d")
					.stdin(Stdio::piped())
					.stdout(Stdio::piped())
					.spawn()?;

				// Spawn the tar process
				let mut tar_child = Command::new("tar")
					.arg("-x")
					.arg("-C")
					.arg(&oci_bundle_path)
					.stdin(Stdio::piped())
					.spawn()?;

				// Take the stdin of lz4 and tar processes
				let mut lz4_stdin = lz4_child.stdin.take().context("lz4 stdin")?;
				let mut lz4_stdout = lz4_child.stdout.take().context("lz4 stdout")?;
				let mut tar_stdin = tar_child.stdin.take().context("tar stdin")?;

				use std::result::Result::{Err, Ok};
				tokio::try_join!(
					// Pipe the response body to lz4 stdin
					async move {
						while let Some(chunk) = stream.next().await {
							let data = chunk?;
							lz4_stdin.write_all(&data).await?;
						}
						lz4_stdin.shutdown().await?;

						anyhow::Ok(())
					},
					// Pipe lz4 stdout to tar stdin
					async move {
						let mut buffer = [0; 8192];
						loop {
							let n = lz4_stdout.read(&mut buffer).await?;
							if n == 0 {
								break;
							}
							tar_stdin.write_all(&buffer[..n]).await?;
						}
						tar_stdin.shutdown().await?;

						anyhow::Ok(())
					},
					// Wait for child processes
					async {
						let cmd_out = lz4_child.wait_with_output().await?;
						ensure!(
							cmd_out.status.success(),
							"failed `lz4` command\n{}",
							std::str::from_utf8(&cmd_out.stderr)?
						);

						Ok(())
					},
					async {
						let cmd_out = tar_child.wait_with_output().await?;
						ensure!(
							cmd_out.status.success(),
							"failed `lz4` command\n{}",
							std::str::from_utf8(&cmd_out.stderr)?
						);

						Ok(())
					},
				)?;
			}
			protocol::ImageKind::JavaScript => {
				let script_path = actor_path.join("index.js");

				match self.config.image.compression {
					protocol::ImageCompression::None => {
						tracing::info!(actor_id=?self.actor_id, "saving artifact to file");

						let mut output_file = File::create(&script_path).await?;

						// Write from stream to file
						while let Some(chunk) = stream.next().await {
							output_file.write_all(&chunk?).await?;
						}
					}
					protocol::ImageCompression::Lz4 => {
						tracing::info!(actor_id=?self.actor_id, "decompressing artifact");

						// Spawn the lz4 process
						let mut lz4_child = Command::new("lz4")
							.arg("-d")
							.arg("-")
							.arg(&script_path)
							.stdin(Stdio::piped())
							.spawn()?;

						// Take the stdin of lz4
						let mut lz4_stdin = lz4_child.stdin.take().context("lz4 stdin")?;

						use std::result::Result::{Err, Ok};
						tokio::try_join!(
							// Pipe the response body to lz4 stdin
							async move {
								while let Some(chunk) = stream.next().await {
									let data = chunk?;
									lz4_stdin.write_all(&data).await?;
								}
								lz4_stdin.shutdown().await?;

								anyhow::Ok(())
							},
							// Wait for child process
							async {
								let cmd_out = lz4_child.wait_with_output().await?;
								ensure!(
									cmd_out.status.success(),
									"failed `lz4` command\n{}",
									std::str::from_utf8(&cmd_out.stderr)?
								);

								Ok(())
							},
						)?;
					}
				}
			}
		}

		Ok(())
	}

	pub async fn setup_oci_bundle(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "setting up oci bundle");

		let actor_path = ctx.actor_path(self.actor_id);
		let oci_bundle_path = actor_path.join("oci-bundle");
		let netns_path = self.netns_path();

		// We need to convert the Docker image to an OCI bundle in order to run it.
		// Allows us to work with the build with umoci
		if let protocol::ImageKind::DockerImage = self.config.image.kind {
			let docker_image_path = actor_path.join("docker-image.tar");
			let oci_image_path = actor_path.join("oci-image");

			tracing::info!(actor_id=?self.actor_id, "converting Docker image -> OCI image");
			let cmd_out = Command::new("skopeo")
				.arg("copy")
				.arg(format!("docker-archive:{}", docker_image_path.display()))
				.arg(format!("oci:{}:default", oci_image_path.display()))
				.output()
				.await?;
			ensure!(
				cmd_out.status.success(),
				"failed `skopeo` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);

			// Allows us to run the bundle natively with runc
			tracing::info!(actor_id=?self.actor_id, "converting OCI image -> OCI bundle");

			let cmd_out = Command::new("umoci")
				.arg("unpack")
				.arg("--image")
				.arg(format!("{}:default", oci_image_path.display()))
				.arg(&oci_bundle_path)
				.output()
				.await?;
			ensure!(
				cmd_out.status.success(),
				"failed `umoci` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
		}

		// Write base config
		fs::write(
			actor_path.join("oci-bundle-config.base.json"),
			serde_json::to_vec(&oci_config::config(
				self.config.resources.cpu,
				self.config.resources.memory,
				self.config.resources.memory_max,
				vec![format!("RIVET_API_ENDPOINT={}", ctx.config().api_endpoint)],
			))?,
		)
		.await?;

		// resolv.conf
		//
		// See also rivet-job.conflist in lib/bolt/core/src/dep/terraform/install_scripts/files/nomad.sh
		fs::write(
			actor_path.join("resolv.conf"),
			indoc!(
				"
				nameserver 8.8.8.8
				nameserver 8.8.4.4
				nameserver 2001:4860:4860::8888
				nameserver 2001:4860:4860::8844
				options rotate
				options edns0
				options attempts:2
				"
			),
		)
		.await?;

		// MARK: Config
		//
		// Sanitize the config.json by copying safe properties from the provided bundle in to our base config.
		//
		// This way, we enforce our own capabilities on the actor instead of trusting the
		// provided config.json
		tracing::info!(actor_id=?self.actor_id, "templating config.json");
		let config_path = oci_bundle_path.join("config.json");
		let override_config_path = actor_path.join("oci-bundle-config.overrides.json");
		fs::rename(&config_path, &override_config_path).await?;

		// TODO: get new envb in here somehow
		// TODO: check bounds

		// Template new config
		let base_config_path = actor_path.join("oci-bundle-config.base.json");
		let base_config_json = fs::read_to_string(&base_config_path).await?;
		let mut base_config = serde_json::from_str::<Value>(&base_config_json)?;

		let override_config_json = fs::read_to_string(&override_config_path).await?;
		let override_config = serde_json::from_str::<Value>(&override_config_json)?;
		let override_process = override_config["process"].clone();

		base_config["process"]["args"] = override_process["args"].clone();
		base_config["process"]["env"] = Value::Array(
			self.config
				.env
				.iter()
				.map(|(k, v)| serde_json::Value::String(format!("{k}={v}")))
				.chain(
					override_process["env"]
						.as_array()
						.context("override_process.env")?
						.iter()
						.cloned(),
				)
				.chain(
					base_config["process"]["env"]
						.as_array()
						.context("process.env")?
						.iter()
						.cloned(),
				)
				.collect(),
		);
		base_config["process"]["user"] = override_process["user"].clone();
		base_config["process"]["cwd"] = override_process["cwd"].clone();
		base_config["linux"]["namespaces"]
			.as_array_mut()
			.context("config.linux.namespaces")?
			.push(json!({
				"type": "network",
				"path": netns_path.to_str().context("netns_path")?,
			}));
		base_config["mounts"]
			.as_array_mut()
			.context("config.mounts")?
			.push(json!({
				"destination": "/etc/resolv.conf",
				"type": "bind",
				"source": actor_path.join("resolv.conf").to_str().context("resolv.conf path")?,
				"options": ["rbind", "rprivate"]
			}));

		fs::write(&config_path, serde_json::to_vec(&base_config)?).await?;

		Ok(())
	}

	pub async fn setup_isolate(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::BoundPort>,
	) -> Result<()> {
		let actor_path = ctx.actor_path(self.actor_id);

		// Schema in v8-isolate-runner
		let config = json!({
			"resources": {
				"memory": self.config.resources.memory,
				"memory_max": self.config.resources.memory_max,
			},
			// TODO:
			"ports": ports
				.values()
				.map(|port| {
					json!({
						// Host ports don't have a target
						"target": port.target.unwrap_or(port.source),
						"protocol": port.protocol,
					})
				})
				.collect::<Vec<_>>(),
			"env": self
				.config
				.env
				.iter()
				.map(|(k, v)| (k.clone(), v.clone()))
				.chain(ports.iter().map(|(label, port)| {
					(
						format!("PORT_{}", label),
						port.target.unwrap_or(port.source).to_string(),
					)
				}))
				.chain(std::iter::once(("RIVET_API_ENDPOINT".to_string(), ctx.config().api_endpoint.clone())))
				.collect::<HashMap<_, _>>(),
			"stakeholder": self.config.stakeholder,
		});
		fs::write(
			actor_path.join("config.json"),
			&serde_json::to_vec(&config)?,
		)
		.await?;

		Ok(())
	}

	// Only ran for bridge networking
	pub async fn setup_cni_network(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::BoundPort>,
	) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "setting up cni network");

		let actor_path = ctx.actor_path(self.actor_id);
		let netns_path = self.netns_path();

		tracing::info!(actor_id=?self.actor_id, "writing cni params");

		let cni_port_mappings = ports
			.iter()
			.map(|(_, port)| {
				json!({
					"HostPort": port.source,
					"ContainerPort": port.target,
					"Protocol": port.protocol.to_string(),
				})
			})
			.collect::<Vec<_>>();

		// MARK: Generate CNI parameters
		//
		// See https://github.com/actornetworking/cni/blob/b62753aa2bfa365c1ceaff6f25774a8047c896b5/cnitool/cnitool.go#L31

		// See Nomad capabilities equivalent:
		// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_cni.go#L105C46-L105C46
		//
		// See supported args:
		// https://github.com/actord/go-cni/blob/6603d5bd8941d7f2026bb5627f6aa4ff434f859a/namespace_opts.go#L22
		let cni_params = json!({
			"portMappings": cni_port_mappings,
		});
		let cni_params_json = serde_json::to_string(&cni_params)?;
		fs::write(
			actor_path.join("cni-cap-args.json"),
			cni_params_json.as_bytes(),
		)
		.await?;

		// MARK: Create network
		//
		// See Nomad network creation:
		// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/network_manager_linux.go#L119

		// Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
		tracing::info!(actor_id=?self.actor_id, "creating network");

		let cmd_out = Command::new("ip")
			.arg("netns")
			.arg("add")
			.arg(self.actor_id.to_string())
			.output()
			.await?;
		ensure!(
			cmd_out.status.success(),
			"failed `ip netns` command\n{}",
			std::str::from_utf8(&cmd_out.stderr)?
		);

		tracing::info!(actor_id=?self.actor_id, "adding network {NETWORK_NAME} to namespace {}", netns_path.display());
		tracing::debug!(
			"Adding network {} to namespace {}",
			NETWORK_NAME,
			netns_path.display(),
		);
		let cmd_out = Command::new("cnitool")
			.arg("add")
			.arg(NETWORK_NAME)
			.arg(netns_path)
			.env("CNI_PATH", "/opt/cni/bin")
			.env("NETCONFPATH", "/opt/cni/config")
			.env("CNI_IFNAME", "eth0")
			.env("CAP_ARGS", cni_params_json)
			.output()
			.await?;
		ensure!(
			cmd_out.status.success(),
			"failed `cnitool` command\n{}",
			std::str::from_utf8(&cmd_out.stderr)?
		);

		Ok(())
	}

	pub(crate) async fn bind_ports(
		&self,
		ctx: &Ctx,
	) -> Result<protocol::HashableMap<String, protocol::BoundPort>> {
		let ports = self
			.config
			.ports
			.iter()
			.map(|(label, port)| match port {
				protocol::Port::GameGuard { target, protocol } => {
					Ok((label, Some(*target), *protocol))
				}
				protocol::Port::Host { protocol } => Ok((label, None, *protocol)),
			})
			.collect::<Result<Vec<_>>>()?;

		let mut tcp_count = 0;
		let mut udp_count = 0;

		// Count ports
		for (_, _, protocol) in &ports {
			match protocol {
				protocol::TransportProtocol::Tcp => tcp_count += 1,
				protocol::TransportProtocol::Udp => udp_count += 1,
			}
		}

		let max = MAX_INGRESS_PORT - MIN_INGRESS_PORT;
		// Add random spread to port selection
		let tcp_offset = rand::thread_rng().gen_range(0..max);
		let udp_offset = rand::thread_rng().gen_range(0..max);

		// Selects available TCP and UDP ports
		let rows = utils::query(|| async {
			sqlx::query_as::<_, (i64, i64)>(indoc!(
				"
				-- NOTE: This does not store the actual port, but an offset from the minimum (see MIN_INGRESS_PORT above)
				INSERT INTO actor_ports (actor_id, port, protocol)
				-- Select TCP ports
				SELECT ?1, port, protocol
				FROM (
					WITH RECURSIVE
						nums(n, i) AS (
							SELECT ?4, ?4
							UNION ALL
							SELECT (n + 1) % (?6 + 1), i + 1
							FROM nums
							WHERE i < ?6 + ?4
						),
						available_ports(port) AS (
							SELECT nums.n
							FROM nums
							LEFT JOIN actor_ports AS p
							ON
								nums.n = p.port AND
								p.protocol = 0 AND
								delete_ts IS NULL
							WHERE
								p.port IS NULL OR
								delete_ts IS NOT NULL
							LIMIT ?2
						)
					SELECT port, 0 AS protocol FROM available_ports
				)
				UNION ALL
				-- Select UDP ports
				SELECT ?1, port, protocol
				FROM (
					WITH RECURSIVE
						nums(n, i) AS (
							SELECT ?5, ?5
							UNION ALL
							SELECT (n + 1) % (?6 + 1), i + 1
							FROM nums
							WHERE i < ?6 + ?5
						),
						available_ports(port) AS (
							SELECT nums.n
							FROM nums
							LEFT JOIN actor_ports AS p
							ON
								nums.n = p.port AND
								p.protocol = 1 AND
								delete_ts IS NULL
							WHERE
								p.port IS NULL OR
								delete_ts IS NOT NULL
							LIMIT ?3
						)
					SELECT port, 1 AS protocol FROM available_ports
				)
				RETURNING port, protocol
				",
			))
			.bind(self.actor_id)
			.bind(tcp_count as i64) // ?2
			.bind(udp_count as i64) // ?3
			.bind(tcp_offset as i64) // ?4
			.bind(udp_offset as i64) // ?5
			.bind(max as i64) // ?6
			.fetch_all(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		if rows.len() != tcp_count + udp_count {
			bail!("not enough available ports");
		}

		let proxied_ports = ports
			.iter()
			.filter(|(_, _, protocol)| matches!(protocol, protocol::TransportProtocol::Tcp))
			.zip(
				rows.iter()
					.filter(|(_, protocol)| *protocol == protocol::TransportProtocol::Tcp as i64),
			)
			.map(|((label, target, protocol), (host_port, _))| {
				(
					(*label).clone(),
					protocol::BoundPort {
						source: MIN_INGRESS_PORT + *host_port as u16,
						target: *target,
						ip: ctx.config().network_ip,
						protocol: *protocol,
					},
				)
			})
			// Chain UDP ports
			.chain(
				ports
					.iter()
					.filter(|(_, _, protocol)| matches!(protocol, protocol::TransportProtocol::Udp))
					.zip(rows.iter().filter(|(_, protocol)| {
						*protocol == protocol::TransportProtocol::Udp as i64
					}))
					.map(|((label, target, protocol), (host_port, _))| {
						(
							(*label).clone(),
							protocol::BoundPort {
								source: MIN_INGRESS_PORT + *host_port as u16,
								target: *target,
								ip: ctx.config().network_ip,
								protocol: *protocol,
							},
						)
					}),
			)
			.collect();

		Ok(proxied_ports)
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup_setup(&self, ctx: &Ctx) -> Result<()> {
		use std::result::Result::{Err, Ok};

		match Command::new("runc")
			.arg("delete")
			.arg("--force")
			.arg(self.actor_id.to_string())
			.output()
			.await
		{
			Ok(cmd_out) => {
				if !cmd_out.status.success() {
					tracing::error!(
						stdout=%std::str::from_utf8(&cmd_out.stdout)?,
						stderr=%std::str::from_utf8(&cmd_out.stderr)?,
						"failed `runc` delete command",
					);
				}
			}
			Err(err) => tracing::error!(?err, "failed to run `runc` command"),
		}

		let actor_path = ctx.actor_path(self.actor_id);
		let netns_path = self.netns_path();

		match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				if let protocol::NetworkMode::Bridge = self.config.network_mode {
					match fs::read_to_string(actor_path.join("cni-cap-args.json")).await {
						Ok(cni_params_json) => {
							match Command::new("cnitool")
								.arg("del")
								.arg(NETWORK_NAME)
								.arg(netns_path)
								.env("CNI_PATH", "/opt/cni/bin")
								.env("NETCONFPATH", "/opt/cni/config")
								.env("CNI_IFNAME", "eth0")
								.env("CAP_ARGS", cni_params_json)
								.output()
								.await
							{
								Ok(cmd_out) => {
									if !cmd_out.status.success() {
										tracing::error!(
											stdout=%std::str::from_utf8(&cmd_out.stdout)?,
											stderr=%std::str::from_utf8(&cmd_out.stderr)?,
											"failed `cnitool del` command",
										);
									}
								}
								Err(err) => tracing::error!(?err, "failed to run `cnitool` command"),
							}
						}
						Err(err) => tracing::error!(?err, "failed to read `cni-cap-args.json`"),
					}
		
					match Command::new("ip")
						.arg("netns")
						.arg("del")
						.arg(self.actor_id.to_string())
						.output()
						.await
					{
						Ok(cmd_out) => {
							if !cmd_out.status.success() {
								tracing::error!(
									stdout=%std::str::from_utf8(&cmd_out.stdout)?,
									stderr=%std::str::from_utf8(&cmd_out.stderr)?,
									"failed `ip netns` command",
								);
							}
						}
						Err(err) => tracing::error!(?err, "failed to run `ip` command"),
					}
				}
			}
			protocol::ImageKind::JavaScript => {}
		}

		Ok(())
	}

	// Path to the created namespace
	fn netns_path(&self) -> PathBuf {
		if let protocol::NetworkMode::Host = self.config.network_mode {
			// Host network
			Path::new("/proc/1/ns/net").to_path_buf()
		} else {
			// CNI network that will be created
			Path::new("/var/run/netns").join(self.actor_id.to_string())
		}
	}
}
