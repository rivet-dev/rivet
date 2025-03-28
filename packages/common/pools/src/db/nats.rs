use std::str::FromStr;

use anyhow::Context;
use rivet_config::Config;
use service_discovery::ServiceDiscovery;

use crate::Error;

pub type NatsPool = async_nats::Client;

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config, client_name: String) -> Result<NatsPool, Error> {
	let nats = &config.server().map_err(Error::Global)?.nats;

	let addrs = match &nats.addresses {
		rivet_config::config::Addresses::Dynamic { fetch_endpoint } => {
			let sd = ServiceDiscovery::new(fetch_endpoint.clone());

			sd.fetch()
				.await
				.context("failed to fetch services")
				.map_err(Error::BuildNatsAddresses)?
				.into_iter()
				.filter_map(|server| server.lan_ip)
				.map(|lan_ip| format!("nats://{lan_ip}:{}", nats.port()))
				.collect::<Vec<_>>()
		}
		rivet_config::config::Addresses::Static(addresses) => addresses
			.into_iter()
			.map(|addr| format!("nats://{addr}"))
			.collect::<Vec<_>>(),
	};

	// Parse nodes
	let server_addrs = addrs
		.iter()
		.map(|url| async_nats::ServerAddr::from_str(url.as_ref()))
		.collect::<Result<Vec<_>, _>>()
		.map_err(Error::BuildNatsIo)?;

	let mut options = if let (Some(username), Some(password)) = (&nats.username, &nats.password) {
		async_nats::ConnectOptions::with_user_and_password(
			username.clone(),
			password.read().clone(),
		)
	} else {
		async_nats::ConnectOptions::new()
	};

	options = options
		.client_capacity(256)
		.subscription_capacity(8192)
		.event_callback({
			let server_addrs = server_addrs.clone();
			move |event| {
				let server_addrs = server_addrs.clone();
				async move {
					match event {
						async_nats::Event::Connected => {
							tracing::debug!(?server_addrs, "nats reconnected");
						}
						async_nats::Event::Disconnected => {
							tracing::error!(?server_addrs, "nats disconnected");
						}
						async_nats::Event::LameDuckMode => {
							tracing::warn!(?server_addrs, "nats lame duck mode");
						}
						async_nats::Event::SlowConsumer(_) => {
							tracing::warn!(?server_addrs, "nats slow consumer");
						}
						async_nats::Event::ServerError(err) => {
							tracing::error!(?server_addrs, ?err, "nats server error");
						}
						async_nats::Event::ClientError(err) => {
							tracing::error!(?server_addrs, ?err, "nats client error");
						}
					}
				}
			}
		});

	// NATS has built in backoff with jitter (with max of 4s), so
	// once the connection is established, we never have to worry
	// about disconnections that aren't handled by NATS.
	tracing::debug!(?server_addrs, "nats connecting");
	let conn = options
		.connect(&server_addrs[..])
		.await
		.map_err(Error::BuildNats)?;

	tracing::debug!(?server_addrs, "nats connected");

	Ok(conn)
}
