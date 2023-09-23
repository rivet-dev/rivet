use std::{fmt, process::Command, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream, task::JoinSet};

use crate::{config, context::ProjectContext, dep::cloudflare, utils};

/// Represents an open cloudflared tunnel.
#[derive(Debug)]
pub struct TunnelInstance {
	pub config: TunnelConfig,
	pub child: std::process::Child,
	pub tempfiles: Vec<tempfile::NamedTempFile>,
}

impl TunnelInstance {
	fn new(
		ctx: &ProjectContext,
		config: TunnelConfig,
		access_secret: &cloudflare::AccessSecret,
	) -> Arc<Self> {
		let tunnel_hostname = format!("{}.{}", config.tunnel_name, ctx.domain_main());
		let (child, tempfiles) = match config.protocol {
			TunnelProtocol::Tcp => {
				// Spawn forwarding process
				let mut cmd = Command::new("cloudflared");
				cmd.arg("access").arg("tcp");
				cmd.arg("--hostname").arg(tunnel_hostname);
				cmd.arg("--url")
					.arg(format!("{}:{}", config.local_host, config.local_port));
				cmd.arg("--service-token-id").arg(&access_secret.client_id);
				cmd.arg("--service-token-secret")
					.arg(&access_secret.client_secret);
				let child = cmd.spawn().unwrap();

				(child, Vec::new())
			}
			TunnelProtocol::Http => {
				// Spawn proxy that includes the correct credentials. We have to
				// use a proxy instead of cloudflared since cloudflared won't
				// proxy normal HTTP requests.

				let mut traefik_dynamic_config =
					tempfile::Builder::new().suffix(".json").tempfile().unwrap();
				serde_json::to_writer(
					&mut traefik_dynamic_config,
					&serde_json::json!({
						"http": {
							"services": {
								"cloudflare-tunnel": {
									"loadBalancer": {
										"servers": [{
											"url": format!("https://{tunnel_hostname}")
										}],
									},
								},
							},
							"routers": {
								"cloudflare-tunnel": {
									"rule": "PathPrefix(`/`)",
									"service": "cloudflare-tunnel",
									"middlewares": ["cloudflare-tunnel-auth"],
								},
							},
							"middlewares": {
								"cloudflare-tunnel-auth": {
									"headers": {
										"customRequestHeaders": {
											"Host": tunnel_hostname,
											"CF-Access-Client-Id": access_secret.client_id,
											"CF-Access-Client-Secret": access_secret.client_secret,
										},
									},
								},
							},
						},
					}),
				)
				.unwrap();

				let mut traefik_config =
					tempfile::Builder::new().suffix(".json").tempfile().unwrap();
				serde_json::to_writer(
					&mut traefik_config,
					&serde_json::json!({
						"entryPoints": {
							"tunnel": {
								"address": format!("127.0.0.1:{}", config.local_port)
							}
						},
						"providers": {
							"file": {
								"filename": traefik_dynamic_config.path().display().to_string(),
							}
						}
					}),
				)
				.unwrap();

				let mut cmd = Command::new("traefik");
				cmd.arg(format!("--configFile={}", traefik_config.path().display()));
				let child = cmd.spawn().unwrap();

				(child, vec![traefik_config, traefik_dynamic_config])
			}
		};

		Arc::new(Self {
			config,
			child,
			tempfiles,
		})
	}

	async fn wait_for_open(&self) {
		loop {
			match TcpStream::connect((self.config.local_host.as_str(), self.config.local_port))
				.await
			{
				Ok(mut stream) => {
					// println!("  * Connected {port}");
					stream.shutdown().await.unwrap();
					break;
				}
				Err(_) => {
					// eprintln!("  * Failed to connect to {port}: {err:?}");
					tokio::time::sleep(Duration::from_millis(50)).await;
					continue;
				}
			}
		}
	}

	pub fn hostname(&self) -> &str {
		&self.config.local_host
	}

	pub fn port(&self) -> u16 {
		self.config.local_port
	}

	pub fn host(&self) -> String {
		format!("{}:{}", self.hostname(), self.port())
	}
}

impl Drop for TunnelInstance {
	fn drop(&mut self) {
		// rivet_term::status::info(
		// 	"Closing",
		// 	format!(
		// 		"{}:{} -> {}",
		// 		self.config.local_host, self.config.local_port, self.config.tunnel_name,
		// 	),
		// );

		// Kill the child
		self.child.kill().expect("failed to kill child");
	}
}

impl fmt::Display for TunnelInstance {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}:{} -> {}",
			self.config.local_host, self.config.local_port, self.config.tunnel_name,
		)
	}
}

#[derive(Debug)]
pub enum TunnelProtocol {
	Tcp,
	Http,
}

#[derive(Debug)]
pub struct TunnelConfig {
	pub tunnel_name: String,
	pub protocol: TunnelProtocol,
	pub local_host: String,
	pub local_port: u16,
}

impl TunnelConfig {
	pub fn new(protocol: TunnelProtocol, tunnel: impl ToString) -> Self {
		TunnelConfig {
			tunnel_name: tunnel.to_string(),
			protocol,
			local_host: "127.0.0.1".into(),
			local_port: utils::pick_port(),
		}
	}

	pub fn new_with_port(protocol: TunnelProtocol, tunnel: impl ToString, local_port: u16) -> Self {
		TunnelConfig {
			tunnel_name: tunnel.to_string(),
			protocol,
			local_host: "127.0.0.1".into(),
			local_port,
		}
	}
}

/// Opens tunnels with cloudflared.
#[derive(Debug)]
pub struct Tunnel {
	tunnels: Vec<Arc<TunnelInstance>>,
}

impl Tunnel {
	pub async fn open(ctx: &ProjectContext, tunnels: Vec<TunnelConfig>) -> Tunnel {
		assert!(
			matches!(
				ctx.ns().dns,
				Some(config::ns::Dns {
					provider: Some(config::ns::DnsProvider::Cloudflare {
						access: Some(_),
						..
					}),
					..
				})
			),
			"cloudflare access not enabled"
		);

		let access_secret = cloudflare::fetch_access_secret(ctx, &["cloudflare", "access", "bolt"])
			.await
			.unwrap();

		let tunnels = tunnels
			.into_iter()
			.map(|config| TunnelInstance::new(ctx, config, &access_secret))
			.collect::<Vec<_>>();

		// Wait for all tunnels to open
		let mut test_join_set = JoinSet::new();
		for port in &tunnels {
			// rivet_term::status::progress("Opening", format!("{port}"));
			let port = port.clone();
			test_join_set.spawn(async move { port.wait_for_open().await });
		}
		while let Some(x) = test_join_set.join_next().await {
			x.unwrap();
		}
		rivet_term::status::success("Tunnel open", "");

		Tunnel { tunnels }
	}

	pub fn tunnels(&self) -> &[Arc<TunnelInstance>] {
		&self.tunnels
	}
}
