use anyhow::{Result, ensure};
use gas::prelude::*;
use std::borrow::Cow;
use universalpubsub::NextOutput;
use universalpubsub::PublishOpts;
use universalpubsub::Subject;

mod sim;

pub const BROADCAST_TOPIC: &str = "rivet.ups.broadcast";

pub struct BroadcastSubject;

impl std::fmt::Display for BroadcastSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		BROADCAST_TOPIC.fmt(f)
	}
}

impl Subject for BroadcastSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(BROADCAST_TOPIC))
	}

	fn as_str(&self) -> Option<&str> {
		Some(BROADCAST_TOPIC)
	}
}

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let ups = pools.ups()?;
	let mut sub = ups.subscribe(BroadcastSubject).await?;

	tracing::debug!(subject=%BROADCAST_TOPIC, "subscribed to broadcast");

	// Process incoming messages
	let handle =
		tokio::spawn(async move { while let Ok(NextOutput::Message(_)) = sub.next().await {} });

	if let Some(sim_config) = sim::Config::from_env()? {
		let sim_udb = pools.udb().ok();
		let sim_ups = sim::pubsub_for_sim(
			&config,
			&ups,
			sim_config.force_driver,
			sim_config.disable_memory_optimization,
		)
		.await?;
		sim::spawn(sim_ups, sim_udb, sim_config);
	}

	loop {
		if let Err(err) = ups
			.publish(BroadcastSubject, &[], PublishOpts::broadcast())
			.await
		{
			tracing::error!(?err, "failed to send broadcast");
		}

		ensure!(!handle.is_finished(), "broadcast sub task finished");

		tokio::time::sleep(std::time::Duration::from_secs(5)).await;
	}
}
