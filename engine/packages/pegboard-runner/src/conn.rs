use anyhow::Context;
use futures_util::StreamExt;
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use rivet_guard_core::WebSocketHandle;
use rivet_runner_protocol as protocol;
use rivet_runner_protocol::*;
use std::{
	collections::HashMap,
	sync::{Arc, atomic::AtomicU32},
	time::Duration,
};
use tokio::sync::Mutex;
use vbare::OwnedVersionedData;

use crate::{errors::WsError, utils::UrlData};

pub struct TunnelActiveRequest {
	/// Subject to send replies to.
	pub gateway_reply_to: String,
}

pub struct Conn {
	pub runner_id: Id,

	pub workflow_id: Id,

	pub protocol_version: u16,

	pub ws_handle: WebSocketHandle,

	pub last_rtt: AtomicU32,

	/// Active HTTP & WebSocket requests. They are separate but use the same mechanism to
	/// maintain state.
	pub tunnel_active_requests: Mutex<HashMap<RequestId, TunnelActiveRequest>>,
}

#[tracing::instrument(skip_all)]
pub async fn init_conn(
	ctx: &StandaloneCtx,
	ws_handle: WebSocketHandle,
	UrlData {
		protocol_version,
		namespace,
		runner_key,
	}: UrlData,
) -> Result<Arc<Conn>> {
	let namespace_name = namespace.clone();
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input { name: namespace })
		.await
		.with_context(|| format!("failed to resolve namespace: {}", namespace_name))?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())
		.with_context(|| format!("namespace not found: {}", namespace_name))?;

	tracing::debug!("new runner connection");

	let ws_rx = ws_handle.recv();
	let mut ws_rx = ws_rx.lock().await;

	// Receive init packet
	let (runner_id, workflow_id) = if let Some(msg) =
		tokio::time::timeout(Duration::from_secs(5), ws_rx.next())
			.await
			.map_err(|_| WsError::TimedOutWaitingForInit.build())?
	{
		let buf = match msg? {
			Message::Binary(buf) => buf,
			Message::Close(_) => return Err(WsError::ConnectionClosed.build()),
			msg => {
				tracing::debug!(?msg, "invalid initial message");
				return Err(WsError::InvalidInitialPacket("must be a binary blob").build());
			}
		};

		let packet = versioned::ToServer::deserialize(&buf, protocol_version)
			.map_err(|err| WsError::InvalidPacket(err.to_string()).build())
			.context("failed to deserialize initial packet from client")?;

		let (runner_id, workflow_id) =
			if let protocol::ToServer::ToServerInit(protocol::ToServerInit {
				name,
				version,
				total_slots,
				..
			}) = &packet
			{
				// Look up existing runner by key
				let existing_runner = ctx
					.op(pegboard::ops::runner::get_by_key::Input {
						namespace_id: namespace.namespace_id,
						name: name.clone(),
						key: runner_key.clone(),
					})
					.await
					.with_context(|| {
						format!(
							"failed to get existing runner by key: {}:{}",
							name, runner_key
						)
					})?;

				let runner_id = if let Some(runner) = existing_runner.runner {
					// IMPORTANT: Before we spawn/get the workflow, we try to update the runner's last ping ts.
					// This ensures if the workflow is currently checking for expiry that it will not expire
					// (because we are about to send signals to it) and if it is already expired (but not
					// completed) we can choose a new runner id.
					let update_ping_res = ctx
						.op(pegboard::ops::runner::update_alloc_idx::Input {
							runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
								runner_id: runner.runner_id,
								action: Action::UpdatePing { rtt: 0 },
							}],
						})
						.await
						.with_context(|| {
							format!("failed to update ping for runner: {}", runner.runner_id)
						})?;

					if update_ping_res
						.notifications
						.into_iter()
						.next()
						.map(|notif| matches!(notif.eligibility, RunnerEligibility::Expired))
						.unwrap_or_default()
					{
						// Runner expired, create a new one
						Id::new_v1(ctx.config().dc_label())
					} else {
						// Use existing runner
						runner.runner_id
					}
				} else {
					// No existing runner for this key, create a new one
					Id::new_v1(ctx.config().dc_label())
				};

				// Spawn a new runner workflow if one doesn't already exist
				let workflow_id = ctx
					.workflow(pegboard::workflows::runner::Input {
						runner_id,
						namespace_id: namespace.namespace_id,
						name: name.clone(),
						key: runner_key.clone(),
						version: version.clone(),
						total_slots: *total_slots,
					})
					.tag("runner_id", runner_id)
					.unique()
					.dispatch()
					.await
					.with_context(|| {
						format!(
							"failed to dispatch runner workflow for runner: {}",
							runner_id
						)
					})?;

				(runner_id, workflow_id)
			} else {
				tracing::debug!(?packet, "invalid initial packet");
				return Err(WsError::InvalidInitialPacket("must be `ToServer::Init`").build());
			};

		// Forward to runner wf
		ctx.signal(pegboard::workflows::runner::Forward { inner: packet })
			.to_workflow_id(workflow_id)
			.send()
			.await
			.with_context(|| {
				format!(
					"failed to forward initial packet to workflow: {}",
					workflow_id
				)
			})?;

		(runner_id, workflow_id)
	} else {
		return Err(WsError::ConnectionClosed.build());
	};

	Ok(Arc::new(Conn {
		runner_id,
		workflow_id,
		protocol_version,
		ws_handle,
		last_rtt: AtomicU32::new(0),
		tunnel_active_requests: Mutex::new(HashMap::new()),
	}))
}
