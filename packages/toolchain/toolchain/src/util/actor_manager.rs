use rivet_api::models;
use anyhow::*;

pub const HTTP_PORT: &str = "http";

pub fn extract_endpoint(actor: &models::ActorActor) -> Result<String> {
	ensure!(actor.started_at.is_some(), "actor manager not started, may be in a crash loop");

	// Get endpoint
	let http_port = actor
		.network
		.ports
		.get(crate::util::actor_manager::HTTP_PORT)
		.context("missing http port")?;
	let protocol = match http_port.protocol {
		models::ActorPortProtocol::Http | models::ActorPortProtocol::Tcp => "http",
		models::ActorPortProtocol::Https => "https",
		models::ActorPortProtocol::TcpTls | models::ActorPortProtocol::Udp => {
			bail!("unsupported protocol")
		}
	};
	let public_hostname = http_port
		.public_hostname
		.as_ref()
		.context("missing public_hostname")?;
	let public_port = http_port
		.public_port
		.as_ref()
		.context("missing public_port")?;
	let endpoint = format!("{protocol}://{public_hostname}:{public_port}");

	Ok(endpoint)
}
