use anyhow::*;
use futures_util::StreamExt;
use indoc::indoc;
use pegboard::protocol;
use pegboard_config::isolate_runner::actor as actor_config;
use rand::Rng;
use serde_json::json;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	process::Stdio,
	result::Result::{Err, Ok},
};
use tokio::{
	fs::{self, File},
	io::{AsyncReadExt, AsyncWriteExt},
	process::Command,
};
use uuid::Uuid;

use super::{oci_config, Actor};
use crate::{ctx::Ctx, utils};

impl Actor {
	pub async fn make_fs(&self, ctx: &Ctx) -> Result<()> {
		let actor_path = ctx.actor_path(self.actor_id);
		let fs_img_path = actor_path.join("fs.img");
		let fs_path = actor_path.join("fs");

		tracing::info!(actor_id=?self.actor_id, "creating fs");

		fs::create_dir(&fs_path).await?;

		if ctx.config().runner.use_mounts() {
			// Create a zero-filled file
			let fs_img = File::create(&fs_img_path).await?;
			fs_img
				.set_len(self.config.resources.disk as u64 * 1024 * 1024)
				.await?;

			// Format file as ext4
			let cmd_out = Command::new("mkfs.ext4").arg(&fs_img_path).output().await?;

			ensure!(
				cmd_out.status.success(),
				"failed `mkfs.ext4` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);

			// Mount fs img as loop mount
			let cmd_out = Command::new("mount")
				.arg("-o")
				.arg("loop")
				.arg(&fs_img_path)
				.arg(&fs_path)
				.output()
				.await?;

			ensure!(
				cmd_out.status.success(),
				"failed `mount` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
		}

		Ok(())
	}

	pub async fn download_image(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "downloading artifact");

		let actor_path = ctx.actor_path(self.actor_id);
		let fs_path = actor_path.join("fs");

		let mut stream = reqwest::get(&self.config.image.artifact_url)
			.await?
			.error_for_status()?
			.bytes_stream();

		match self.config.image.kind {
			protocol::ImageKind::DockerImage => {
				let docker_image_path = fs_path.join("docker-image.tar");

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
			protocol::ImageKind::OciBundle | protocol::ImageKind::JavaScript => {
				match self.config.image.compression {
					protocol::ImageCompression::None => {
						tracing::info!(actor_id=?self.actor_id, "unarchiving artifact");

						// Spawn the tar process
						let mut tar_child = Command::new("tar")
							.arg("-x")
							.arg("-C")
							.arg(&fs_path)
							.stdin(Stdio::piped())
							.spawn()?;

						// Take the stdin of tar process
						let mut tar_stdin = tar_child.stdin.take().context("tar stdin")?;

						tokio::try_join!(
							// Pipe the response body to lz4 stdin
							async move {
								while let Some(chunk) = stream.next().await {
									let data = chunk?;
									tar_stdin.write_all(&data).await?;
								}
								tar_stdin.shutdown().await?;

								anyhow::Ok(())
							},
							// Wait for child process
							async {
								let cmd_out = tar_child.wait_with_output().await?;
								ensure!(
									cmd_out.status.success(),
									"failed `tar` command\n{}",
									std::str::from_utf8(&cmd_out.stderr)?
								);

								Ok(())
							},
						)?;
					}
					protocol::ImageCompression::Lz4 => {
						tracing::info!(actor_id=?self.actor_id, "decompressing and unarchiving artifact");

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
							.arg(&fs_path)
							.stdin(Stdio::piped())
							.spawn()?;

						// Take the stdin of lz4 and tar processes
						let mut lz4_stdin = lz4_child.stdin.take().context("lz4 stdin")?;
						let mut lz4_stdout = lz4_child.stdout.take().context("lz4 stdout")?;
						let mut tar_stdin = tar_child.stdin.take().context("tar stdin")?;

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
									"failed `tar` command\n{}",
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

	pub async fn setup_oci_bundle(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "setting up oci bundle");

		let actor_path = ctx.actor_path(self.actor_id);
		let fs_path = actor_path.join("fs");
		let netns_path = self.netns_path();

		// We need to convert the Docker image to an OCI bundle in order to run it.
		// Allows us to work with the build with umoci
		if let protocol::ImageKind::DockerImage = self.config.image.kind {
			let docker_image_path = fs_path.join("docker-image.tar");
			let oci_image_path = fs_path.join("oci-image");

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
				.arg(&fs_path)
				.output()
				.await?;
			ensure!(
				cmd_out.status.success(),
				"failed `umoci` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);

			// Remove artifacts
			tokio::try_join!(
				fs::remove_file(&docker_image_path),
				fs::remove_dir_all(&oci_image_path),
			)?;
		}

		// Read the config.json from the user-provided OCI bundle
		let oci_bundle_config_path = fs_path.join("config.json");
		let user_config_json = fs::read_to_string(&oci_bundle_config_path).await?;
		let user_config =
			serde_json::from_str::<super::partial_oci_config::PartialOciConfig>(&user_config_json)?;

		// Build env
		let env = user_config
			.process
			.env
			.into_iter()
			.chain(
				self.build_default_env(ctx, &ports)
					.into_iter()
					.map(|(k, v)| format!("{k}={v}")),
			)
			.collect::<Vec<String>>();

		// Replace the config.json with a new config
		//
		// This config selectively uses parts from the user's OCI bundle in order to maintain security
		let config = oci_config::config(oci_config::ConfigOpts {
			actor_path: &actor_path,
			netns_path: &netns_path,
			args: user_config.process.args,
			env,
			user: user_config.process.user,
			cwd: user_config.process.cwd,
			cpu: self.config.resources.cpu,
			memory: self.config.resources.memory,
			memory_max: self.config.resources.memory_max,
		})?;
		fs::write(oci_bundle_config_path, serde_json::to_vec(&config)?).await?;

		// resolv.conf
		//
		// See also rivet-actor.conflist in packages/services/cluster/src/workflows/server/install/install_scripts/files/pegboard_configure.sh
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

		// hosts
		fs::write(
			fs_path.join("hosts"),
			indoc!(
				"
				127.0.0.1	localhost
				::1			localhost ip6-localhost ip6-loopback
				"
			),
		)
		.await?;

		Ok(())
	}

	pub async fn setup_isolate(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		let actor_path = ctx.actor_path(self.actor_id);

		let config = actor_config::Config {
			resources: actor_config::Resources {
				memory: self.config.resources.memory,
				memory_max: self.config.resources.memory_max,
			},
			// TODO:
			ports: ports
				.values()
				.map(|port| actor_config::Port {
					target: port.target,
					protocol: port.protocol,
				})
				.collect::<Vec<_>>(),
			env: self.build_default_env(ctx, &ports),
			owner: self.config.owner.clone(),
			vector_socket_addr: ctx.config().vector.clone().map(|x| x.address),
		};

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
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
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

		let cni_network_name = &ctx.config().cni.network_name();
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

		tracing::info!(actor_id=?self.actor_id, "adding network {cni_network_name} to namespace {}", netns_path.display());
		tracing::debug!(
			"Adding network {} to namespace {}",
			cni_network_name,
			netns_path.display(),
		);
		let cmd_out = Command::new("cnitool")
			.arg("add")
			.arg(cni_network_name)
			.arg(netns_path)
			.env("CNI_PATH", &ctx.config().cni.bin_path())
			.env("NETCONFPATH", &ctx.config().cni.config_path())
			.env("CNI_IFNAME", &ctx.config().cni.network_interface)
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
	) -> Result<protocol::HashableMap<String, protocol::ProxiedPort>> {
		let (mut gg_ports, mut host_ports): (Vec<_>, Vec<_>) = self
			.config
			.ports
			.iter()
			.partition(|(_, port)| matches!(port.routing, protocol::PortRouting::GameGuard));

		// TODO: Could combine these into one query
		let (mut gg_port_rows, mut host_port_rows) = tokio::try_join!(
			bind_ports_inner(
				ctx,
				self.actor_id,
				&gg_ports,
				ctx.config().network.lan_port_range_min()
					..=ctx.config().network.lan_port_range_max()
			),
			bind_ports_inner(
				ctx,
				self.actor_id,
				&host_ports,
				ctx.config().network.wan_port_range_min()
					..=ctx.config().network.wan_port_range_max()
			),
		)?;

		// The SQL query returns a list of TCP ports then UDP ports. We sort the input ports here to match
		// that order.
		gg_ports.sort_by_key(|(_, port)| port.protocol);
		host_ports.sort_by_key(|(_, port)| port.protocol);
		// We sort the SQL results here also, just in case
		gg_port_rows.sort_by_key(|(_, protocol)| *protocol);
		host_port_rows.sort_by_key(|(_, protocol)| *protocol);

		let proxied_ports =
			gg_ports
				.iter()
				.zip(gg_port_rows)
				.map(|((label, port), (host_port, _))| {
					let host_port = host_port as u16;

					(
						(*label).clone(),
						protocol::ProxiedPort {
							source: host_port,
							// When no target port was selected, default to randomly selected host port
							target: port.target.unwrap_or(host_port),
							ip: ctx.config().network.bind_ip,
							protocol: port.protocol,
						},
					)
				})
				// Chain host ports
				.chain(host_ports.iter().zip(host_port_rows).map(
					|((label, port), (host_port, _))| {
						let host_port = host_port as u16;

						(
							(*label).clone(),
							protocol::ProxiedPort {
								source: host_port,
								// When no target port was selected, default to randomly selected host port
								target: port.target.unwrap_or(host_port),
								ip: ctx.config().network.bind_ip,
								protocol: port.protocol,
							},
						)
					},
				))
				.collect();

		Ok(proxied_ports)
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup_setup(&self, ctx: &Ctx) -> Result<()> {
		let actor_path = ctx.actor_path(self.actor_id);
		let netns_path = self.netns_path();

		if ctx.config().runner.use_mounts() {
			match Command::new("umount")
				.arg("-dl")
				.arg(actor_path.join("fs"))
				.output()
				.await
			{
				Result::Ok(cmd_out) => {
					if !cmd_out.status.success() {
						tracing::error!(
							stdout=%std::str::from_utf8(&cmd_out.stdout)?,
							stderr=%std::str::from_utf8(&cmd_out.stderr)?,
							"failed `umount` command",
						);
					}
				}
				Err(err) => tracing::error!(?err, "failed to run `umount` command"),
			}
		}

		match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				match Command::new("runc")
					.arg("delete")
					.arg("--force")
					.arg(self.actor_id.to_string())
					.output()
					.await
				{
					Result::Ok(cmd_out) => {
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

				if let protocol::NetworkMode::Bridge = self.config.network_mode {
					match fs::read_to_string(actor_path.join("cni-cap-args.json")).await {
						Result::Ok(cni_params_json) => {
							match Command::new("cnitool")
								.arg("del")
								.arg(&ctx.config().cni.network_name())
								.arg(netns_path)
								.env("CNI_PATH", &ctx.config().cni.bin_path())
								.env("NETCONFPATH", &ctx.config().cni.config_path())
								.env("CNI_IFNAME", &ctx.config().cni.network_interface)
								.env("CAP_ARGS", cni_params_json)
								.output()
								.await
							{
								Result::Ok(cmd_out) => {
									if !cmd_out.status.success() {
										tracing::error!(
											stdout=%std::str::from_utf8(&cmd_out.stdout)?,
											stderr=%std::str::from_utf8(&cmd_out.stderr)?,
											"failed `cnitool del` command",
										);
									}
								}
								Err(err) => {
									tracing::error!(?err, "failed to run `cnitool` command")
								}
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
						Result::Ok(cmd_out) => {
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

	fn build_default_env(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> HashMap<String, String> {
		self.config
			.env
			.iter()
			.map(|(k, v)| (k.clone(), v.clone()))
			// Add port env vars and api endpoint
			.chain(ports.iter().map(|(label, port)| {
				(
					format!("PORT_{}", label.replace('-', "_")),
					port.target.to_string(),
				)
			}))
			.chain([
				(
					"RIVET_API_ENDPOINT".to_string(),
					ctx.config().cluster.api_endpoint.to_string(),
				),
				(
					"RIVET_METADATA".to_string(),
					self.config.metadata.get().to_string(),
				),
			])
			.collect()
	}
}

async fn bind_ports_inner(
	ctx: &Ctx,
	actor_id: Uuid,
	ports: &[(&String, &protocol::Port)],
	range: std::ops::RangeInclusive<u16>,
) -> Result<Vec<(i64, i64)>> {
	if ports.is_empty() {
		return Ok(Vec::new());
	}

	let mut tcp_count = 0;
	let mut udp_count = 0;

	// Count ports
	for (_, port) in ports {
		match port.protocol {
			protocol::TransportProtocol::Tcp => tcp_count += 1,
			protocol::TransportProtocol::Udp => udp_count += 1,
		}
	}

	let truncated_max = range.end() - range.start();
	// Add random spread to port selection
	let tcp_offset = rand::thread_rng().gen_range(0..truncated_max);
	let udp_offset = rand::thread_rng().gen_range(0..truncated_max);

	// Selects available TCP and UDP ports
	let rows = utils::query(|| async {
		sqlx::query_as::<_, (i64, i64)>(indoc!(
			"
			INSERT INTO actor_ports (actor_id, port, protocol)
			-- Select TCP ports
			SELECT ?1, port, protocol
			FROM (
				WITH RECURSIVE
					nums(n, i) AS (
						SELECT ?4, ?4
						UNION ALL
						SELECT (n + 1) % (?7 + 1), i + 1
						FROM nums
						WHERE i < ?7 + ?4
					),
					available_ports(port) AS (
						SELECT nums.n + ?6
						FROM nums
						LEFT JOIN actor_ports AS p
						ON
							nums.n + ?6 = p.port AND
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
						SELECT (n + 1) % (?7 + 1), i + 1
						FROM nums
						WHERE i < ?7 + ?5
					),
					available_ports(port) AS (
						SELECT nums.n + ?6
						FROM nums
						LEFT JOIN actor_ports AS p
						ON
							nums.n + ?6 = p.port AND
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
		.bind(actor_id)
		.bind(tcp_count as i64) // ?2
		.bind(udp_count as i64) // ?3
		.bind(tcp_offset as i64) // ?4
		.bind(udp_offset as i64) // ?5
		.bind(*range.start() as i64) // ?6
		.bind(truncated_max as i64) // ?7
		.fetch_all(&mut *ctx.sql().await?)
		.await
	})
	.await?;

	if rows.len() != tcp_count + udp_count {
		bail!("not enough available ports");
	}

	Ok(rows)
}
