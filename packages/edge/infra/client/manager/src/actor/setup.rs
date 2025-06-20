use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	result::Result::{Err, Ok},
	time::Instant,
};

use anyhow::*;
use indoc::indoc;
use pegboard::protocol;
use pegboard_config::isolate_runner::actor as actor_config;
use rand::Rng;
use serde_json::json;
use tokio::{
	fs::{self, File},
	process::Command,
};
use uuid::Uuid;

use super::{oci_config, Actor};
use crate::{ctx::Ctx, utils};

impl Actor {
	pub async fn make_fs(&self, ctx: &Ctx) -> Result<()> {
		let timer = Instant::now();
		let actor_path = ctx.actor_path(self.actor_id, self.generation);

		let fs_img_path = actor_path.join("fs.img");
		let fs_path = actor_path.join("fs");
		let fs_upper_path = fs_path.join("upper");
		let fs_work_path = fs_path.join("work");
		let image_path = ctx.image_path(self.config.image.id);

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "creating fs");

		fs::create_dir(&fs_path)
			.await
			.context("failed to create actor fs dir")?;

		if ctx.config().runner.use_mounts() {
			tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "creating disk image");
			// Create a zero-filled file
			let fs_img = File::create(&fs_img_path)
				.await
				.context("failed to create disk image")?;
			fs_img
				.set_len(self.config.resources.disk as u64 * 1024 * 1024)
				.await
				.context("failed to set disk image length")?;

			tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "formatting disk image");
			// Format file as ext4
			let cmd_out = Command::new("mkfs.ext4")
				.arg(&fs_img_path)
				.output()
				.await
				.context("failed to run `mkfs.ext4`")?;

			ensure!(
				cmd_out.status.success(),
				"failed `mkfs.ext4` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);

			tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "mounting disk image");

			// Mount fs img as loop mount
			let cmd_out = Command::new("mount")
				.arg("-o")
				.arg("loop")
				.arg(&fs_img_path)
				.arg(&fs_path)
				.output()
				.await
				.context("failed to run `mount`")?;

			ensure!(
				cmd_out.status.success(),
				"failed `mount` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);

			// Create folders on disk
			fs::create_dir(&fs_upper_path)
				.await
				.context("failed to create actor fs upper dir")?;
			fs::create_dir(&fs_work_path)
				.await
				.context("failed to create actor fs work dir")?;

			tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "mounting overlay");

			ensure!(
				fs::metadata(&image_path).await.is_ok(),
				"image dir does not exist"
			);

			// Overlay mount setup:
			// lowerdir = extracted build in manager's builds cache folder
			// upperdir = {actor dir}/fs/upper folder
			// workdir = {actor dir}/fs/work folder
			// merged dir = also fs/upper folder, it mounts to the upperdir
			let cmd_out = Command::new("mount")
				.arg("-t")
				.arg("overlay")
				// Arbitrary device name
				.arg(format!("{}-{}", self.actor_id, self.generation))
				.arg("-o")
				.arg(format!(
					"lowerdir={},upperdir={},workdir={}",
					image_path.display(),
					fs_upper_path.display(),
					fs_work_path.display()
				))
				.arg(&fs_upper_path)
				.output()
				.await
				.context("failed to run overlay `mount`")?;

			ensure!(
				cmd_out.status.success(),
				"failed overlay `mount` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
		} else {
			// Create folder on host
			fs::create_dir(&fs_upper_path)
				.await
				.context("failed to create actor fs upper dir")?;

			tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "copying image contents to fs");

			// Copy everything from the image (lowerdir) to the upperdir
			utils::copy_dir_all(image_path, &fs_upper_path)
				.await
				.context("failed to copy image contents to fs upper dir")?;
		}

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_MAKE_FS_DURATION.observe(duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=duration,
			"fs creation completed",
		);

		Ok(())
	}

	pub async fn download_image(&self, ctx: &Ctx) -> Result<()> {
		let timer = Instant::now();

		ctx.image_download_handler
			.download(ctx, &self.config.image)
			.await?;

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_DOWNLOAD_IMAGE_DURATION.observe(duration);

		Ok(())
	}

	pub async fn setup_oci_bundle(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		let timer = Instant::now();
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "setting up oci bundle");

		let actor_path = ctx.actor_path(self.actor_id, self.generation);
		let fs_path = actor_path.join("fs").join("upper");
		let netns_path = self.netns_path();

		// Read the config.json from the user-provided OCI bundle
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"reading OCI bundle configuration",
		);
		let oci_bundle_config_path = fs_path.join("config.json");
		let user_config_json = fs::read_to_string(&oci_bundle_config_path)
			.await
			.context("failed to read oci config")?;
		let user_config =
			serde_json::from_str::<super::partial_oci_config::PartialOciConfig>(&user_config_json)?;

		// Build env
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"building environment variables",
		);
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
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"generating OCI configuration",
		);
		let config = oci_config::config(oci_config::ConfigOpts {
			actor_path: &actor_path,
			netns_path: &netns_path,
			args: user_config.process.args,
			env,
			user: user_config.process.user,
			cwd: user_config.process.cwd,
			use_resource_constraints: ctx.config().runner.use_resource_constraints(),
			cpu: self.config.resources.cpu,
			memory: self.config.resources.memory,
			memory_max: self.config.resources.memory_max,
		})?;
		// Parallelize file writes for better performance
		// Prepare content for all files before writing
		let config_json = serde_json::to_vec(&config)?;

		// resolv.conf content
		// See also rivet-actor.conflist in packages/services/cluster/src/workflows/server/install/install_scripts/files/pegboard_configure.sh
		let resolv_conf = indoc!(
			"
			nameserver 8.8.8.8
			nameserver 8.8.4.4
			nameserver 2001:4860:4860::8888
			nameserver 2001:4860:4860::8844
			options rotate
			options edns0
			options attempts:2
			"
		);

		// hosts file content
		let hosts_content = indoc!(
			"
			127.0.0.1	localhost
			::1			localhost ip6-localhost ip6-loopback
			"
		);

		// Write all files in parallel
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"writing configuration files",
		);
		tokio::try_join!(
			fs::write(oci_bundle_config_path, config_json),
			fs::write(actor_path.join("resolv.conf"), resolv_conf),
			fs::write(fs_path.join("hosts"), hosts_content)
		)?;

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_OCI_BUNDLE_DURATION.observe(duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=duration,
			"OCI bundle setup completed"
		);

		Ok(())
	}

	pub async fn setup_isolate(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		let timer = Instant::now();
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "setting up isolate environment");

		let actor_path = ctx.actor_path(self.actor_id, self.generation);

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"generating isolate configuration"
		);
		let config = actor_config::Config {
			resources: actor_config::Resources {
				memory: self.config.resources.memory,
				memory_max: self.config.resources.memory_max,
			},
			ports: ports
				.iter()
				.map(|(name, port)| {
					(
						name.clone(),
						actor_config::Port {
							target: port.target,
							protocol: port.protocol,
						},
					)
				})
				.collect(),
			env: self.build_default_env(ctx, &ports),
			metadata: self.config.metadata.clone(),
			vector_socket_addr: ctx.config().vector.clone().map(|x| x.address),
		};

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"writing isolate configuration"
		);
		fs::write(
			actor_path.join("config.json"),
			&serde_json::to_vec(&config)?,
		)
		.await?;

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_ISOLATE_DURATION.observe(duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=duration,
			"isolate setup completed"
		);

		Ok(())
	}

	// Only ran for bridge networking
	pub async fn setup_cni_network(
		&self,
		ctx: &Ctx,
		ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		let timer = Instant::now();
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "setting up cni network");

		let actor_path = ctx.actor_path(self.actor_id, self.generation);
		let netns_path = self.netns_path();

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "preparing cni port mappings");

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
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "generating and writing cni parameters");
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
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "creating network namespace");

		let cni_network_name = &ctx.config().cni.network_name();
		let cmd_out = Command::new("ip")
			.arg("netns")
			.arg("add")
			.arg(netns_path.file_name().context("bad netns path")?)
			.output()
			.await
			.context("failed to run `ip`")?;
		ensure!(
			cmd_out.status.success(),
			"failed `ip netns` command\n{}",
			std::str::from_utf8(&cmd_out.stderr)?
		);

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			cni_network_name=cni_network_name,
			"adding network to namespace",
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
			.await
			.context("failed to run `cnitool`")?;
		ensure!(
			cmd_out.status.success(),
			"failed `cnitool` command\n{}",
			std::str::from_utf8(&cmd_out.stderr)?
		);

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_CNI_NETWORK_DURATION.observe(duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=duration,
			"cni network setup completed"
		);

		Ok(())
	}

	pub(crate) async fn bind_ports(
		&self,
		ctx: &Ctx,
	) -> Result<protocol::HashableMap<String, protocol::ProxiedPort>> {
		let timer = Instant::now();
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "binding ports");

		let (mut gg_ports, mut host_ports): (Vec<_>, Vec<_>) = self
			.config
			.ports
			.iter()
			.partition(|(_, port)| matches!(port.routing, protocol::PortRouting::GameGuard));

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			gg_ports_count=gg_ports.len(),
			host_ports_count=host_ports.len(),
			"partitioned ports for binding"
		);

		// TODO: Could combine these into one query
		let (mut gg_port_rows, mut host_port_rows) = tokio::try_join!(
			bind_ports_inner(
				ctx,
				self.actor_id,
				self.generation,
				&gg_ports,
				ctx.config().network.lan_port_range_min()
					..=ctx.config().network.lan_port_range_max()
			),
			bind_ports_inner(
				ctx,
				self.actor_id,
				self.generation,
				&host_ports,
				ctx.config().network.wan_port_range_min()
					..=ctx.config().network.wan_port_range_max()
			),
		)?;

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"sorting ports"
		);

		// The SQL query returns a list of TCP ports then UDP ports. We sort the input ports here to match
		// that order.
		gg_ports.sort_by_key(|(_, port)| port.protocol);
		host_ports.sort_by_key(|(_, port)| port.protocol);
		// We sort the SQL results here also, just in case
		gg_port_rows.sort_by_key(|(_, protocol)| *protocol);
		host_port_rows.sort_by_key(|(_, protocol)| *protocol);

		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			"mapping proxied ports"
		);

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
							lan_hostname: ctx.config().network.lan_hostname.clone(),
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
								lan_hostname: ctx.config().network.lan_hostname.clone(),
								protocol: port.protocol,
							},
						)
					},
				))
				.collect::<protocol::HashableMap<_, _>>();

		let duration = timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_BIND_PORTS_DURATION.observe(duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=duration,
			ports_count=proxied_ports.len(),
			"ports binding completed"
		);

		Ok(proxied_ports)
	}

	// This function is meant to run gracefully-handled fallible steps to clean up every part of the setup
	// process
	#[tracing::instrument(skip_all)]
	pub async fn cleanup_setup(&self, ctx: &Ctx) {
		let actor_path = ctx.actor_path(self.actor_id, self.generation);
		let netns_path = self.netns_path();

		// Clean up fs mounts
		if ctx.config().runner.use_mounts() {
			match Command::new("umount")
				.arg("-dl")
				.arg(actor_path.join("fs").join("upper"))
				.output()
				.await
			{
				Result::Ok(cmd_out) => {
					if !cmd_out.status.success() {
						tracing::error!(
							actor_id=?self.actor_id,
							generation=?self.generation,
							stdout=%std::str::from_utf8(&cmd_out.stdout).unwrap_or("<failed to parse stdout>"),
							stderr=%std::str::from_utf8(&cmd_out.stderr).unwrap_or("<failed to parse stderr>"),
							"failed overlay `umount` command",
						);
					}
				}
				Err(err) => {
					tracing::error!(
						actor_id=?self.actor_id,
						generation=?self.generation,
						?err,
						"failed to run overlay `umount` command",
					);
				}
			}

			match Command::new("umount")
				.arg("-dl")
				.arg(actor_path.join("fs"))
				.output()
				.await
			{
				Result::Ok(cmd_out) => {
					if !cmd_out.status.success() {
						tracing::error!(
							actor_id=?self.actor_id,
							generation=?self.generation,
							stdout=%std::str::from_utf8(&cmd_out.stdout).unwrap_or("<failed to parse stdout>"),
							stderr=%std::str::from_utf8(&cmd_out.stderr).unwrap_or("<failed to parse stderr>"),
							"failed `umount` command",
						);
					}
				}
				Err(err) => {
					tracing::error!(
						actor_id=?self.actor_id,
						generation=?self.generation,
						?err,
						"failed to run `umount` command",
					);
				}
			}
		}

		match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				// Clean up runc
				match Command::new("runc")
					.arg("delete")
					.arg("--force")
					.arg(format!("{}-{}", self.actor_id, self.generation))
					.output()
					.await
				{
					Result::Ok(cmd_out) => {
						if !cmd_out.status.success() {
							tracing::error!(
								actor_id=?self.actor_id,
								generation=?self.generation,
								stdout=%std::str::from_utf8(&cmd_out.stdout).unwrap_or("<failed to parse stdout>"),
								stderr=%std::str::from_utf8(&cmd_out.stderr).unwrap_or("<failed to parse stderr>"),
								"failed `runc` delete command",
							);
						}
					}
					Err(err) => {
						tracing::error!(
							actor_id=?self.actor_id,
							generation=?self.generation,
							?err,
							"failed to run `runc` command",
						);
					}
				}

				if let protocol::NetworkMode::Bridge = self.config.network_mode {
					match fs::read_to_string(actor_path.join("cni-cap-args.json")).await {
						Result::Ok(cni_params_json) => {
							match Command::new("cnitool")
								.arg("del")
								.arg(&ctx.config().cni.network_name())
								.arg(&netns_path)
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
											actor_id=?self.actor_id,
											generation=?self.generation,
											stdout=%std::str::from_utf8(&cmd_out.stdout).unwrap_or("<failed to parse stdout>"),
											stderr=%std::str::from_utf8(&cmd_out.stderr).unwrap_or("<failed to parse stderr>"),
											"failed `cnitool del` command",
										);
									}
								}
								Err(err) => {
									tracing::error!(
										actor_id=?self.actor_id,
										generation=?self.generation,
										?err,
										"failed to run `cnitool` command",
									);
								}
							}
						}
						Err(err) => {
							tracing::error!(
								actor_id=?self.actor_id,
								generation=?self.generation,
								?err,
								"failed to read `cni-cap-args.json`",
							);
						}
					}

					if let Some(netns_name) = netns_path.file_name() {
						// Clean up network
						match Command::new("ip")
							.arg("netns")
							.arg("del")
							.arg(netns_name)
							.output()
							.await
						{
							Result::Ok(cmd_out) => {
								if !cmd_out.status.success() {
									tracing::error!(
										actor_id=?self.actor_id,
										generation=?self.generation,
										stdout=%std::str::from_utf8(&cmd_out.stdout).unwrap_or("<failed to parse stdout>"),
										stderr=%std::str::from_utf8(&cmd_out.stderr).unwrap_or("<failed to parse stderr>"),
										"failed `ip netns` command",
									);
								}
							}
							Err(err) => {
								tracing::error!(
									actor_id=?self.actor_id,
									generation=?self.generation,
									?err,
									"failed to run `ip` command",
								);
							}
						}
					} else {
						tracing::error!(
							actor_id=?self.actor_id,
							generation=?self.generation,
							?netns_path,
							"invalid netns path",
						);
					}
				}
			}
			protocol::ImageKind::JavaScript => {}
		}

		// Delete entire actor dir. Note that for actors using KV storage, it is persisted elsewhere and will
		// not be deleted by this (see `persist_storage` in the runner protocol).
		if let Err(err) = tokio::fs::remove_dir_all(&actor_path).await {
			tracing::error!(
				actor_id=?self.actor_id,
				generation=?self.generation,
				?err,
				"failed to delete actor dir",
			);
		}
	}

	// Path to the created namespace
	fn netns_path(&self) -> PathBuf {
		if let protocol::NetworkMode::Host = self.config.network_mode {
			// Host network
			Path::new("/proc/1/ns/net").to_path_buf()
		} else {
			// CNI network that will be created
			Path::new("/var/run/netns").join(format!("{}-{}", self.actor_id, self.generation))
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
					format!("PORT_{}", label.to_uppercase().replace('-', "_")),
					port.target.to_string(),
				)
			}))
			.chain([(
				"RIVET_API_ENDPOINT".to_string(),
				ctx.config().cluster.api_endpoint.to_string(),
			)])
			.collect()
	}
}

async fn bind_ports_inner(
	ctx: &Ctx,
	actor_id: Uuid,
	generation: u32,
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
	let rows = utils::sql::query(|| async {
		sqlx::query_as::<_, (i64, i64)>(indoc!(
			"
			INSERT INTO actor_ports (actor_id, generation, port, protocol)
			-- Select TCP ports
			SELECT ?1, ?2, port, protocol
			FROM (
				WITH RECURSIVE
					nums(n, i) AS (
						SELECT ?5, ?5
						UNION ALL
						SELECT (n + 1) % (?8 + 1), i + 1
						FROM nums
						WHERE i < ?8 + ?5
					),
					available_ports(port) AS (
						SELECT nums.n + ?7
						FROM nums
						LEFT JOIN actor_ports AS p
						ON
							nums.n + ?7 = p.port AND
							p.protocol = 0 AND
							delete_ts IS NULL
						WHERE
							p.port IS NULL OR
							delete_ts IS NOT NULL
						LIMIT ?3
					)
				SELECT port, 0 AS protocol FROM available_ports
			)
			UNION ALL
			-- Select UDP ports
			SELECT ?1, ?2, port, protocol
			FROM (
				WITH RECURSIVE
					nums(n, i) AS (
						SELECT ?6, ?6
						UNION ALL
						SELECT (n + 1) % (?8 + 1), i + 1
						FROM nums
						WHERE i < ?8 + ?6
					),
					available_ports(port) AS (
						SELECT nums.n + ?7
						FROM nums
						LEFT JOIN actor_ports AS p
						ON
							nums.n + ?7 = p.port AND
							p.protocol = 1 AND
							delete_ts IS NULL
						WHERE
							p.port IS NULL OR
							delete_ts IS NOT NULL
						LIMIT ?4
					)
				SELECT port, 1 AS protocol FROM available_ports
			)
			RETURNING port, protocol
			",
		))
		.bind(actor_id)
		.bind(generation as i64)
		.bind(tcp_count as i64) // ?3
		.bind(udp_count as i64) // ?4
		.bind(tcp_offset as i64) // ?5
		.bind(udp_offset as i64) // ?6
		.bind(*range.start() as i64) // ?7
		.bind(truncated_max as i64) // ?8
		.fetch_all(&mut *ctx.sql().await?)
		.await
	})
	.await?;

	if rows.len() != tcp_count + udp_count {
		bail!("not enough available ports");
	}

	Ok(rows)
}
