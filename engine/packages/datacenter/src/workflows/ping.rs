use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::{FutureExt, StreamExt};
use gas::prelude::*;

use crate::keys;

pub const TICK_RATE: Duration = Duration::from_secs(7 * 60);

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[workflow]
pub async fn datacenter_ping(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.repeat(|ctx| {
		async move {
			ctx.activity(RecordPingInput {}).await?;

			ctx.sleep(TICK_RATE).await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct RecordPingInput {}

#[activity(RecordPing)]
async fn record_ping(ctx: &ActivityCtx, _input: &RecordPingInput) -> Result<()> {
	let client = rivet_pools::reqwest::client().await?;

	let dcs = ctx
		.config()
		.topology()
		.datacenters
		.iter()
		// Exclude current dc
		.filter(|dc| dc.datacenter_label != ctx.config().dc_label())
		.cloned()
		.collect::<Vec<_>>();

	let responses = futures_util::stream::iter(dcs)
		.map(|dc| {
			let client = client.clone();

			async move { (dc.datacenter_label, record_ping(ctx, &client, &dc).await) }
		})
		.buffer_unordered(128)
		.collect::<Vec<_>>()
		.await;

	for (dc_label, res) in responses {
		if let Err(err) = res {
			tracing::warn!(?dc_label, ?err, "failed to ping dc");
		}
	}

	Ok(())
}

async fn record_ping(
	ctx: &ActivityCtx,
	client: &reqwest::Client,
	dc: &rivet_config::config::topology::Datacenter,
) -> Result<()> {
	let peer_url = dc.peer_url.join("/health")?;
	let start = Instant::now();

	let peer_res = client
		.get(peer_url)
		.timeout(std::time::Duration::from_secs(5))
		.send()
		.await?;

	if !peer_res.status().is_success() {
		bail!("Peer health check returned status: {}", peer_res.status())
	}

	let rtt = u32::try_from(start.elapsed().as_millis())?;

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			tx.write(
				&keys::LastPingTsKey::new(dc.datacenter_label),
				util::timestamp::now(),
			)?;
			tx.write(&keys::LastRttKey::new(dc.datacenter_label), rtt)?;

			Ok(())
		})
		.await?;

	Ok(())
}
