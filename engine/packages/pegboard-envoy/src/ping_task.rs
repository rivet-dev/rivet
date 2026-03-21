use anyhow::ensure;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use rand::Rng;
use rivet_envoy_protocol::{self as protocol, versioned};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use tokio::sync::watch;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn};

#[tracing::instrument(name="ping_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut ping_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let update_ping_interval =
		Duration::from_millis(ctx.config().pegboard().envoy_update_ping_interval());
	let ping_timeout_ms = ctx.config().pegboard().envoy_ping_timeout();

	loop {
		// Jitter sleep to prevent stampeding herds
		let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..128));
		tokio::select! {
			_ = tokio::time::sleep(update_ping_interval + jitter) => {}
			_ = ping_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		// Check if the last ping is past the timeout threshold
		let last_ping_ts = conn.last_ping_ts.load(Ordering::Relaxed);
		let now = util::timestamp::now();
		ensure!(
			now - last_ping_ts <= ping_timeout_ms,
			"envoy ws ping timed out"
		);

		// Update ping
		ctx.op(pegboard::ops::envoy::update_ping::Input {
			namespace_id: conn.namespace_id,
			envoy_key: conn.envoy_key.clone(),
			update_lb: !conn.is_serverless,
			rtt: conn.last_rtt.load(Ordering::Relaxed),
		})
		.await?;

		// Send ping to envoy
		let ping_msg = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyPing(
			protocol::ToEnvoyPing {
				ts: util::timestamp::now(),
			},
		));
		let ping_msg_serialized = ping_msg.serialize(conn.protocol_version)?;
		conn.ws_handle
			.send(Message::Binary(ping_msg_serialized.into()))
			.await?;
	}
}
