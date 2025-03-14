use anyhow::*;
use rivet_api::models;

pub const HTTP_PORT: &str = "http";

pub fn extract_endpoint(actor: &models::ActorsActor) -> Result<String> {
	ensure!(
		actor.started_at.is_some(),
		"actor manager not started, may be in a crash loop"
	);

	// Get endpoint
	let http_port = actor
		.network
		.ports
		.get(crate::util::actor_manager::HTTP_PORT)
		.context("missing http port")?;
	let protocol = match http_port.protocol {
		models::ActorsPortProtocol::Http | models::ActorsPortProtocol::Tcp => "http",
		models::ActorsPortProtocol::Https => "https",
		models::ActorsPortProtocol::TcpTls | models::ActorsPortProtocol::Udp => {
			bail!("unsupported protocol")
		}
	};
	let hostname = http_port.hostname.as_ref().context("missing hostname")?;
	let port = http_port.port.as_ref().context("missing port")?;
	let path = http_port.path.clone().unwrap_or_default();
	let endpoint = format!("{protocol}://{hostname}:{port}{path}");

	Ok(endpoint)
}
