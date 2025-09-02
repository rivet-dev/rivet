use std::{str::FromStr, sync::Arc};

use anyhow::*;
use rivet_config::{Config, config};
use universalpubsub as ups;

use crate::Error;

pub type UpsPool = ups::PubSub;

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config, client_name: String) -> Result<UpsPool> {
	let driver = match config.pubsub() {
		config::PubSub::Nats(nats) => {
			// Parse nodes
			let server_addrs = nats
				.addresses
				.iter()
				.map(|addr| format!("nats://{addr}"))
				.map(|url| async_nats::ServerAddr::from_str(url.as_ref()))
				.collect::<Result<Vec<_>, _>>()
				.map_err(Error::BuildNatsIo)?;

			let mut options =
				if let (Some(username), Some(password)) = (&nats.username, &nats.password) {
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
			Arc::new(ups::driver::nats::NatsDriver::connect(options, &server_addrs[..]).await?)
				as ups::PubSubDriverHandle
		}
		config::PubSub::PostgresNotify(pg) => {
			tracing::debug!("creating postgres pubsub driver");
			Arc::new(
				ups::driver::postgres::PostgresDriver::connect(pg.url.read().clone(), true).await?,
			) as ups::PubSubDriverHandle
		}
		config::PubSub::Memory(memory) => {
			tracing::debug!(channel = ?memory.channel, "creating memory pubsub driver");
			Arc::new(ups::driver::memory::MemoryDriver::new(
				memory.channel.clone(),
			)) as ups::PubSubDriverHandle
		}
	};

	Ok(ups::PubSub::new(driver))
}
