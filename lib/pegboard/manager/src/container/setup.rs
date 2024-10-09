use std::{
	os::unix::process::CommandExt,
	path::{Path, PathBuf},
	process::Stdio,
};

use anyhow::*;
use futures_util::StreamExt;
use indoc::indoc;
use nix::{
	sys::wait::{waitpid, WaitStatus},
	unistd::{fork, pipe, read, write, ForkResult, Pid},
};
use pegboard::protocol;
use rand::Rng;
use serde_json::{json, Value};
use tokio::{
	fs::{self, File},
	io::{AsyncReadExt, AsyncWriteExt},
	process::Command,
};

use super::{oci_config, Container};
use crate::{ctx::Ctx, utils};

const NETWORK_NAME: &str = "rivet-pegboard";
const MIN_INGRESS_PORT: u16 = 20000;
const MAX_INGRESS_PORT: u16 = 31999;

impl Container {
	pub async fn setup_oci_bundle(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "setting up oci bundle");

		let container_path = ctx.container_path(self.container_id);
		let oci_bundle_path = container_path.join("oci-bundle");
		let netns_path = self.netns_path();

		// Write base config
		fs::write(
			container_path.join("oci-bundle-config.base.json"),
			serde_json::to_vec(&oci_config::config(
				self.config.resources.cpu,
				self.config.resources.memory,
				self.config.resources.memory_max,
				vec!["RIVET_API_ENDPOINT".to_string(), {
					ctx.api_endpoint
						.read()
						.await
						.clone()
						.context("missing api endpoint")?
				}],
			))?,
		)
		.await?;

		tracing::info!(container_id=?self.container_id, "downloading artifact");

		let mut stream = reqwest::get(&self.config.image.artifact_url)
			.await?
			.error_for_status()?
			.bytes_stream();

		match self.config.image.kind {
			protocol::ImageKind::DockerImage => {
				let docker_image_path = container_path.join("docker-image.tar");
				let oci_image_path = container_path.join("oci-image");

				match self.config.image.compression {
					protocol::ImageCompression::None => {
						tracing::info!(container_id=?self.container_id, "saving artifact to file");

						let mut output_file = File::create(&docker_image_path).await?;

						// Write from stream to file
						while let Some(chunk) = stream.next().await {
							output_file.write_all(&chunk?).await?;
						}
					}
					protocol::ImageCompression::Lz4 => {
						tracing::info!(container_id=?self.container_id, "decompressing artifact");

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
							async { lz4_child.wait().await.map_err(Into::into) },
						)?;
					}
				}

				// We need to convert the Docker image to an OCI bundle in order to run it.
				// Allows us to work with the build with umoci
				tracing::info!(container_id=?self.container_id, "converting Docker image -> OCI image");
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
				tracing::info!(container_id=?self.container_id, "converting OCI image -> OCI bundle");

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
			protocol::ImageKind::OciBundle => {
				tracing::info!(container_id=?self.container_id, "decompressing and unarchiving artifact");

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
					async { lz4_child.wait().await.map_err(Into::into) },
					async { tar_child.wait().await.map_err(Into::into) },
				)?;

				// // Wait for lz4 and tar to finish
				// lz4_child.wait().await?;
				// tar_child.wait().await?;

				// // Write from stream to file
				// while let Some(chunk) = stream.next().await {
				// 	output_file.write_all(&chunk?).await?;
				// }

				// fs::create_dir(&oci_bundle_path).await?;

				// let oci_bundle_path = oci_bundle_path.clone();
				// let tmp_path2 = tmp_path.clone();
				// let compression = self.config.image.compression;
				// let container_id = self.container_id;
				// task::spawn_blocking(move || {
				// 	let input_file = std::fs::File::open(tmp_path2)?;

				// 	match compression {
				// 		protocol::ImageCompression::None => {
				// 			tracing::info!(?container_id, "unzipping archive");

				// 			let mut archive = Archive::new(input_file);

				// 			archive.unpack(oci_bundle_path)?;
				// 		}
				// 		protocol::ImageCompression::Lz4 => {
				// 			tracing::info!(?container_id, "decompressing and unzipping archive");

				// 			let decoder = FrameDecoder::new(input_file);
				// 			let mut archive = Archive::new(decoder);

				// 			archive.unpack(oci_bundle_path)?;
				// 		}
				// 	}

				// 	Ok(())
				// })
				// .await??;

				// // Delete tmp file
				// fs::remove_file(&tmp_path).await?;
			}
		}

		// resolv.conf
		//
		// See also rivet-job.conflist in lib/bolt/core/src/dep/terraform/install_scripts/files/nomad.sh
		fs::write(
			container_path.join("resolv.conf"),
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
		// This way, we enforce our own capabilities on the container instead of trusting the
		// provided config.json
		tracing::info!(container_id=?self.container_id, "templating config.json");
		let config_path = oci_bundle_path.join("config.json");
		let override_config_path = container_path.join("oci-bundle-config.overrides.json");
		fs::rename(&config_path, &override_config_path).await?;

		// TODO: get new envb in here somehow
		// TODO: check bounds

		// Template new config
		let base_config_path = container_path.join("oci-bundle-config.base.json");
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
				"source": container_path.join("resolv.conf").to_str().context("resolv.conf path")?,
				"options": ["rbind", "rprivate"]
			}));

		fs::write(&config_path, serde_json::to_vec(&base_config)?).await?;

		Ok(())
	}

	// Only ran for bridge networking
	pub async fn setup_cni_network(
		&self,
		ctx: &Ctx,
		proxied_ports: &protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "setting up cni network");

		let container_path = ctx.container_path(self.container_id);
		let netns_path = self.netns_path();

		tracing::info!(container_id=?self.container_id, "writing cni params");

		let cni_port_mappings = proxied_ports
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
		// See https://github.com/containernetworking/cni/blob/b62753aa2bfa365c1ceaff6f25774a8047c896b5/cnitool/cnitool.go#L31

		// See Nomad capabilities equivalent:
		// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_cni.go#L105C46-L105C46
		//
		// See supported args:
		// https://github.com/containerd/go-cni/blob/6603d5bd8941d7f2026bb5627f6aa4ff434f859a/namespace_opts.go#L22
		let cni_params = json!({
			"portMappings": cni_port_mappings,
		});
		let cni_params_json = serde_json::to_string(&cni_params)?;
		fs::write(
			container_path.join("cni-cap-args.json"),
			cni_params_json.as_bytes(),
		)
		.await?;

		// MARK: Create network
		//
		// See Nomad network creation:
		// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/network_manager_linux.go#L119

		// Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
		tracing::info!(container_id=?self.container_id, "creating network");

		let cmd_out = Command::new("ip")
			.arg("netns")
			.arg("add")
			.arg(self.container_id.to_string())
			.output()
			.await?;
		ensure!(
			cmd_out.status.success(),
			"failed `ip netns` command\n{}",
			std::str::from_utf8(&cmd_out.stderr)?
		);

		tracing::info!(container_id=?self.container_id, "adding network {NETWORK_NAME} to namespace {}", netns_path.display());
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

	// Used for CNI network creation
	pub(crate) async fn bind_ports(
		&self,
		ctx: &Ctx,
	) -> Result<protocol::HashableMap<String, protocol::ProxiedPort>> {
		let ports = self
			.config
			.ports
			.iter()
			.map(|(label, port)| match port {
				protocol::Port::GameGuard { target, protocol } => Ok((label, *target, *protocol)),
				protocol::Port::Host { .. } => {
					// TODO:
					bail!("host ports not implemented");
				}
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
		let tcp_offset = rand::thread_rng().gen_range(0..max);
		let udp_offset = rand::thread_rng().gen_range(0..max);

		// Selects available TCP and UDP ports
		let rows = utils::query(|| async {
			sqlx::query_as::<_, (i64, i64)>(indoc!(
				"
				INSERT INTO container_ports (container_id, port, protocol)
				SELECT ?1, port, protocol
				-- Select TCP ports
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
							LEFT JOIN container_ports AS p
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
				SELECT ?1, port, protocol
				-- Select UDP ports
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
							LEFT JOIN container_ports AS p
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
			.bind(self.container_id)
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
					protocol::ProxiedPort {
						source: *host_port as u16,
						target: *target,
						ip: ctx.network_ip,
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
							protocol::ProxiedPort {
								source: *host_port as u16,
								target: *target,
								ip: ctx.network_ip,
								protocol: *protocol,
							},
						)
					}),
			)
			.collect();

		Ok(proxied_ports)
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup(&self, ctx: &Ctx) -> Result<()> {
		use std::result::Result::{Err, Ok};

		tracing::info!(container_id=?self.container_id, "cleaning up");

		{
			// Cleanup ctx
			let mut containers = ctx.containers.write().await;
			containers.remove(&self.container_id);
		}

		match Command::new("runc")
			.arg("delete")
			.arg("--force")
			.arg(self.container_id.to_string())
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

		let container_path = ctx.container_path(self.container_id);
		let netns_path = self.netns_path();

		if let protocol::NetworkMode::Bridge = self.config.network_mode {
			match fs::read_to_string(container_path.join("cni-cap-args.json")).await {
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
				.arg(self.container_id.to_string())
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

		Ok(())
	}

	// Path to the created namespace
	fn netns_path(&self) -> PathBuf {
		if let protocol::NetworkMode::Bridge = self.config.network_mode {
			// Host network
			Path::new("/proc/1/ns/net").to_path_buf()
		} else {
			// CNI network that will be created
			Path::new("/var/run/netns").join(self.container_id.to_string())
		}
	}
}

pub fn spawn_orphaned_container_runner(
	container_runner_path: PathBuf,
	container_path: PathBuf,
	env: &[(&str, String)],
) -> Result<Pid> {
	// Prepare the arguments for the container runner
	let runner_args = vec![container_path.to_str().context("bad path")?];

	// Pipe communication between processes
	let (pipe_read, pipe_write) = pipe()?;

	// NOTE: This is why we fork the process twice: https://stackoverflow.com/a/5386753
	match unsafe { fork() }.context("process first fork failed")? {
		ForkResult::Parent { child } => {
			// Close the writing end of the pipe in the parent
			nix::unistd::close(pipe_write)?;

			// Ensure that the child process spawned successfully
			match waitpid(child, None).context("waitpid failed")? {
				WaitStatus::Exited(_, 0) => {
					// Read the second child's PID from the pipe
					let mut buf = [0u8; 4];
					read(pipe_read, &mut buf)?;
					let second_child_pid = Pid::from_raw(i32::from_le_bytes(buf));

					Ok(second_child_pid)
				}
				WaitStatus::Exited(_, status) => {
					bail!("Child process exited with status {}", status)
				}
				_ => bail!("Unexpected wait status for child process"),
			}
		}
		ForkResult::Child => {
			// Child process
			match unsafe { fork() } {
				Result::Ok(ForkResult::Parent { child }) => {
					// Write the second child's PID to the pipe
					let child_pid_bytes = child.as_raw().to_le_bytes();
					write(pipe_write, &child_pid_bytes)?;

					// Exit the intermediate child
					std::process::exit(0);
				}
				Result::Ok(ForkResult::Child) => {
					// Exit immediately on fail in order to not leak process
					let err = std::process::Command::new(&container_runner_path)
						.args(&runner_args)
						.envs(env.iter().cloned())
						.stdin(Stdio::null())
						.stdout(Stdio::null())
						.stderr(Stdio::null())
						.exec();
					eprintln!("exec failed: {err:?}");
					std::process::exit(1);
				}
				Err(err) => {
					// Exit immediately in order to not leak child process.
					//
					// The first fork doesn't need to exit on error since it
					eprintln!("process second fork failed: {err:?}");
					std::process::exit(1);
				}
			}
		}
	}
}
