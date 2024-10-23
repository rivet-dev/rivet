use rand::seq::SliceRandom;
use rivet_config::Config;
use std::str::FromStr;

use crate::Error;

pub type NatsPool = async_nats::Client;

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config, client_name: String) -> Result<NatsPool, Error> {
	let nats = &config.server().map_err(Error::Global)?.nats;

	// Randomize the URLs in order to randomize the node priority and load
	// balance connections across nodes.
	let mut shuffled_urls = nats.urls.clone();
	shuffled_urls.shuffle(&mut rand::thread_rng());

	// Parse nodes
	let server_addrs = shuffled_urls
		.iter()
		.map(|url| async_nats::ServerAddr::from_str(&url.to_string()))
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
							tracing::info!(?server_addrs, "nats reconnected");
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
	tracing::info!(?server_addrs, "nats connecting");
	let conn = options
		.connect(&server_addrs[..])
		.await
		.map_err(Error::BuildNats)?;
	tracing::info!(?server_addrs, "nats connected");

	Ok(conn)
}
