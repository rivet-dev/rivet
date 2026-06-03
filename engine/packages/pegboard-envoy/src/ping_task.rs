use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use rand::Rng;
use rivet_envoy_protocol::{self as protocol, versioned};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use tokio::sync::watch;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, errors::WsError, metrics, ws_to_tunnel_task};

#[tracing::instrument(name = "ping_task", skip_all)]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut ping_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let update_ping_interval =
		Duration::from_millis(ctx.config().pegboard().envoy_update_ping_interval());
	let ping_timeout_ms = ctx.config().pegboard().envoy_ping_timeout();

	send_ping(&ctx, &conn).await?;

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
		let last_ping_ts = conn.last_ping_ts.load(Ordering::SeqCst);
		let now = util::timestamp::now();
		let time_since_last_pong_ms = now - last_ping_ts;
		metrics::ENVOY_TIME_SINCE_LAST_PONG_SECONDS
			.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
			.observe(time_since_last_pong_ms as f64 / 1000.0);
		if time_since_last_pong_ms > ping_timeout_ms {
			tracing::warn!(
				envoy_key = %conn.envoy_key,
				time_since_last_pong_ms,
				ping_timeout_ms,
				"engine declaring envoy timed out (no pong within threshold)"
			);
			return Err(WsError::TimedOut.build());
		}

		send_ping(&ctx, &conn).await?;
	}
}

async fn send_ping(ctx: &StandaloneCtx, conn: &Conn) -> Result<()> {
	ctx.op(pegboard::ops::envoy::update_ping::Input {
		namespace_id: conn.namespace_id,
		envoy_key: conn.envoy_key.clone(),
		update_lb: !conn.is_serverless(),
		rtt: conn.last_rtt.load(Ordering::Relaxed),
	})
	.await?;

	let ping_msg =
		versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyPing(protocol::ToEnvoyPing {
			ts: util::timestamp::now(),
		}));
	let ping_msg_serialized = ping_msg.serialize(conn.protocol_version)?;
	let _in_flight = ws_to_tunnel_task::WsResponseInFlightGuard::new();
	conn.ws_handle
		.send(Message::Binary(ping_msg_serialized.into()))
		.await?;

	Ok(())
}
