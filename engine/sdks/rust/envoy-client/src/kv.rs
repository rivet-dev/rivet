use rivet_envoy_protocol as protocol;
use tokio::sync::oneshot;

use crate::connection::ws_send;
use crate::deadlines::DeadlineKey;
use crate::envoy::EnvoyContext;
use crate::metrics::METRICS;

pub struct KvRequestEntry {
	pub actor_id: String,
	pub data: protocol::KvRequestData,
	pub response_tx: oneshot::Sender<anyhow::Result<protocol::KvResponseData>>,
	pub sent: bool,
	pub timestamp: crate::time::Instant,
	/// Counts this request on `kv_requests_inflight` while it is tracked.
	pub _inflight: rivet_metrics::IntGaugeGuard,
}

pub const KV_EXPIRE_MS: u64 = 30_000;

fn kv_deadline(timestamp: crate::time::Instant) -> crate::time::Instant {
	timestamp + std::time::Duration::from_millis(KV_EXPIRE_MS)
}

pub async fn handle_kv_request(
	ctx: &mut EnvoyContext,
	actor_id: String,
	data: protocol::KvRequestData,
	response_tx: oneshot::Sender<anyhow::Result<protocol::KvResponseData>>,
) {
	let request_id = ctx.next_kv_request_id;
	ctx.next_kv_request_id += 1;

	let timestamp = crate::time::Instant::now();
	let entry = KvRequestEntry {
		actor_id,
		data,
		response_tx,
		sent: false,
		timestamp,
		_inflight: rivet_metrics::GaugeGuardExt::inc_guard(&METRICS.kv_requests_inflight),
	};

	ctx.kv_requests.insert(request_id, entry);
	ctx.deadlines
		.schedule(DeadlineKey::Kv(request_id), kv_deadline(timestamp));

	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if ws_available {
		send_single_kv_request(ctx, request_id).await;
	}
}

pub async fn handle_kv_response(ctx: &mut EnvoyContext, response: protocol::ToEnvoyKvResponse) {
	let request = ctx.kv_requests.remove(&response.request_id);
	ctx.deadlines.cancel(&DeadlineKey::Kv(response.request_id));

	if let Some(request) = request {
		match response.data {
			protocol::KvResponseData::KvErrorResponse(ref e) => {
				let _ = request
					.response_tx
					.send(Err(anyhow::anyhow!("{}", e.message)));
			}
			_ => {
				let _ = request.response_tx.send(Ok(response.data));
			}
		}
	} else {
		tracing::error!(
			request_id = response.request_id,
			"received kv response for unknown request id"
		);
	}
}

pub async fn send_single_kv_request(ctx: &mut EnvoyContext, request_id: u32) {
	let request = ctx.kv_requests.get_mut(&request_id);
	let Some(request) = request else { return };
	if request.sent {
		return;
	}

	ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetKvRequest(protocol::ToRivetKvRequest {
			actor_id: request.actor_id.clone().into(),
			request_id,
			data: request.data.clone(),
		}),
	)
	.await;

	// Re-get after async call
	if let Some(request) = ctx.kv_requests.get_mut(&request_id) {
		request.sent = true;
		request.timestamp = crate::time::Instant::now();
		ctx.deadlines
			.schedule(DeadlineKey::Kv(request_id), kv_deadline(request.timestamp));
	}
}

pub async fn process_unsent_kv_requests(ctx: &mut EnvoyContext) {
	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if !ws_available {
		return;
	}

	let unsent: Vec<u32> = ctx
		.kv_requests
		.iter()
		.filter(|(_, req)| !req.sent)
		.map(|(id, _)| *id)
		.collect();

	for request_id in unsent {
		send_single_kv_request(ctx, request_id).await;
	}
}

/// Fails one KV request whose deadline passed. A request whose timestamp moved since its
/// deadline was set is scheduled again instead.
pub(crate) fn expire_kv_request(
	ctx: &mut EnvoyContext,
	request_id: u32,
	now: crate::time::Instant,
) {
	let Some(request) = ctx.kv_requests.get(&request_id) else {
		return;
	};
	let deadline = kv_deadline(request.timestamp);
	if deadline > now {
		ctx.deadlines
			.schedule(DeadlineKey::Kv(request_id), deadline);
		return;
	}
	let request = ctx
		.kv_requests
		.remove(&request_id)
		.expect("checked kv request");
	tracing::warn!(
		request_id,
		was_sent = request.sent,
		age_ms = now.duration_since(request.timestamp).as_millis() as u64,
		"kv request expired at its deadline"
	);
	let _ = request
		.response_tx
		.send(Err(anyhow::anyhow!("KV request timed out")));
}
