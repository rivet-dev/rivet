use std::collections::HashMap;

use super::runner;
use crate::pubsub_subjects::RunnerReceiverSubject;
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::StreamExt;
use gas::prelude::*;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_runner_protocol as protocol;
use tokio::time::Duration;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

const X_RIVET_ENDPOINT: HeaderName = HeaderName::from_static("x-rivet-endpoint");
const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
const X_RIVET_TOTAL_SLOTS: HeaderName = HeaderName::from_static("x-rivet-total-slots");
const X_RIVET_RUNNER_NAME: HeaderName = HeaderName::from_static("x-rivet-runner-name");
const X_RIVET_NAMESPACE_NAME: HeaderName = HeaderName::from_static("x-rivet-namespace-name");

const DRAIN_GRACE_PERIOD: Duration = Duration::from_secs(5);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub runner_wf_id: Id,
	pub namespace_id: Id,
	pub runner_name: String,
	pub namespace_name: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub request_lifespan: u32,
	pub slots_per_runner: u32,
}

#[workflow]
pub async fn pegboard_serverless_connection(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	// Run the connection activity, which will handle the full lifecycle
	let res = ctx
		.activity(OutboundReqInput {
			runner_wf_id: input.runner_wf_id,
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
			namespace_name: input.namespace_name.clone(),
			url: input.url.clone(),
			headers: input.headers.clone(),
			request_lifespan: input.request_lifespan,
			slots_per_runner: input.slots_per_runner,
		})
		.await?;

	// If we failed to send inline during the activity, durably ensure the
	// signal is dispatched here
	if res.send_drain_started {
		ctx.signal(runner::ConnectionDrainStarted {})
			.to_workflow_id(input.runner_wf_id)
			.send()
			.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct OutboundReqInput {
	runner_wf_id: Id,
	namespace_id: Id,
	runner_name: String,
	namespace_name: String,
	url: String,
	headers: HashMap<String, String>,
	request_lifespan: u32,
	slots_per_runner: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutboundReqOutput {
	send_drain_started: bool,
}

impl std::hash::Hash for OutboundReqInput {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.namespace_id.hash(state);
		self.runner_name.hash(state);
		self.namespace_name.hash(state);
		self.url.hash(state);
		// Sort and hash headers for deterministic hashing
		let mut headers_vec: Vec<_> = self.headers.iter().collect();
		headers_vec.sort();
		for (k, v) in headers_vec {
			k.hash(state);
			v.hash(state);
		}
		self.request_lifespan.hash(state);
		self.slots_per_runner.hash(state);
	}
}

#[activity(OutboundReq)]
#[timeout = u64::MAX]
#[max_retries = usize::MAX]
async fn outbound_req(ctx: &ActivityCtx, input: &OutboundReqInput) -> Result<OutboundReqOutput> {
	if is_runner_draining(ctx, input.runner_wf_id).await? {
		return Ok(OutboundReqOutput {
			send_drain_started: true,
		});
	}

	let mut drain_sub = ctx
		.subscribe::<Drain>(("workflow_id", ctx.workflow_id()))
		.await?;

	let current_dc = ctx.config().topology().current_dc()?;

	let client = rivet_pools::reqwest::client_no_timeout().await?;

	let token = if let Some(auth) = &ctx.config().auth {
		Some((
			X_RIVET_TOKEN,
			HeaderValue::try_from(auth.admin_token.read())?,
		))
	} else {
		None
	};

	let headers = input
		.headers
		.clone()
		.into_iter()
		.flat_map(|(k, v)| {
			// NOTE: This will filter out invalid headers without warning
			Some((
				k.parse::<HeaderName>().ok()?,
				v.parse::<HeaderValue>().ok()?,
			))
		})
		.chain([
			(
				X_RIVET_ENDPOINT,
				HeaderValue::try_from(current_dc.public_url.to_string())?,
			),
			(
				X_RIVET_TOTAL_SLOTS,
				HeaderValue::try_from(input.slots_per_runner)?,
			),
			(
				X_RIVET_RUNNER_NAME,
				HeaderValue::try_from(input.runner_name.clone())?,
			),
			(
				X_RIVET_NAMESPACE_NAME,
				HeaderValue::try_from(input.namespace_name.clone())?,
			),
			// Deprecated
			(
				HeaderName::from_static("x-rivet-namespace-id"),
				HeaderValue::try_from(input.namespace_name.clone())?,
			),
		])
		.chain(token)
		.collect();

	let endpoint_url = format!("{}/start", input.url.trim_end_matches('/'));
	tracing::debug!(%endpoint_url, "sending outbound req");
	let req = client.get(endpoint_url).headers(headers);

	let mut source = sse::EventSource::new(req).context("failed creating event source")?;
	let mut runner_id = None;

	let request_lifespan = Duration::from_secs(input.request_lifespan as u64);

	let stream_handler = async {
		while let Some(event) = source.next().await {
			match event {
				Ok(sse::Event::Open) => {}
				Ok(sse::Event::Message(msg)) => {
					tracing::debug!(%msg.data, "received outbound req message");

					if runner_id.is_none() {
						let data = BASE64.decode(msg.data).context("invalid base64 message")?;
						let payload =
							protocol::versioned::ToServerlessServer::deserialize_with_embedded_version(&data)
								.context("invalid payload")?;

						match payload {
							protocol::ToServerlessServer::ToServerlessServerInit(init) => {
								runner_id =
									Some(Id::parse(&init.runner_id).context("invalid runner id")?);
							}
						}
					}
				}
				Err(sse::Error::StreamEnded) => {
					tracing::debug!("outbound req stopped early");

					return Ok(());
				}
				Err(sse::Error::InvalidStatusCode(code, res)) => {
					let body = res
						.text()
						.await
						.unwrap_or_else(|_| "<could not read body>".to_string());
					bail!(
						"invalid status code ({code}):\n{}",
						util::safe_slice(&body, 0, 512)
					);
				}
				Err(err) => return Err(err.into()),
			}
		}

		anyhow::Ok(())
	};

	let sleep_until_drain = request_lifespan.saturating_sub(DRAIN_GRACE_PERIOD);
	tokio::select! {
		res = stream_handler => {
			return match res {
				Err(e) => Err(e.into()),
				// TODO:
				// For unexpected closes, we don’t know if the runner connected
				// or not bc we can’t correlate the runner id.
				//
				// Lifecycle state falls apart
				Ok(_) => Ok(OutboundReqOutput {
					send_drain_started: false
				})
			};
		},
		_ = tokio::time::sleep(sleep_until_drain) => {}
		_ = drain_sub.next() => {}
	};

	tracing::debug!(?runner_id, "connection reached lifespan, needs draining");

	if let Err(e) = ctx
		.signal(runner::ConnectionDrainStarted {})
		// This is ok, because we only send DrainStarted once
		.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
		.to_workflow_id(input.runner_wf_id)
		.send()
		.await
	{
		tracing::warn!(
			runner_name=%input.runner_name.clone(),
			namespace_id=%input.namespace_id,
			workflow_id=%ctx.workflow_id(),
			"failed to send signal: {}", e
		);

		// If we failed to send, get the workflow to send it durably
		return Ok(OutboundReqOutput {
			send_drain_started: true,
		});
	}

	// After we tell the pool we're draining, any remaining failures
	// don't matter as the pool already stopped caring about us.
	finish_non_critical_draining(ctx, source, runner_id)
		.await
		.ok();

	Ok(OutboundReqOutput {
		send_drain_started: false,
	})
}

async fn is_runner_draining(ctx: &ActivityCtx, runner_wf_id: Id) -> Result<bool> {
	let res = ctx.get_workflows(vec![runner_wf_id]).await?;
	let Some(runner_wf) = res.first() else {
		// HACK: This is undefined state, but we have no way to mark the workflow as dead
		// so we return true, and the DrainStarted signal call will attempt to send
		// the signal back to this unexistant parent.
		//
		// Eventually it will fail too many times, and the wf will die.
		tracing::error!(
			?runner_wf_id,
			"couldn't find serverless connection's parent runner wf"
		);
		return Ok(true);
	};

	let state = runner_wf.parse_state::<runner::State>()?;
	Ok(state.is_draining)
}

async fn finish_non_critical_draining(
	ctx: &ActivityCtx,
	mut source: sse::EventSource,
	mut runner_id: Option<Id>,
) -> Result<()> {
	if let Some(runner_id) = runner_id {
		drain_runner(ctx, runner_id).await?;
	}

	// Continue waiting on req while draining
	let wait_for_shutdown_fut = async move {
		while let Some(event) = source.next().await {
			match event {
				Ok(sse::Event::Open) => {}
				Ok(sse::Event::Message(msg)) => {
					tracing::debug!(%msg.data, ?runner_id, "received outbound req message");

					// If runner_id is none at this point it means we did not send the stopping signal yet, so
					// send it now
					if runner_id.is_none() {
						let data = BASE64.decode(msg.data).context("invalid base64 message")?;
						let payload =
						protocol::versioned::ToServerlessServer::deserialize_with_embedded_version(
							&data,
						)
						.context("invalid payload")?;

						match payload {
							protocol::ToServerlessServer::ToServerlessServerInit(init) => {
								let runner_id_local =
									Id::parse(&init.runner_id).context("invalid runner id")?;
								runner_id = Some(runner_id_local);
								drain_runner(ctx, runner_id_local).await?;
							}
						}
					}
				}
				Err(sse::Error::StreamEnded) => break,
				Err(err) => return Err(err.into()),
			}
		}

		Result::<()>::Ok(())
	};

	// Wait for runner to shut down
	tokio::select! {
		res = wait_for_shutdown_fut => return res.map_err(Into::into),
		_ = tokio::time::sleep(DRAIN_GRACE_PERIOD) => {
			tracing::debug!(?runner_id, "reached drain grace period before runner shut down")
		}
	}

	// Close connection
	//
	// This will force the runner to stop the request in order to avoid hitting the serverless
	// timeout threshold
	if let Some(runner_id) = runner_id {
		publish_to_client_stop(ctx, runner_id).await?;
	}

	tracing::debug!(?runner_id, "outbound req stopped");

	Ok(())
}

async fn drain_runner(ctx: &ActivityCtx, runner_id: Id) -> Result<()> {
	let res = ctx
		.signal(crate::workflows::runner::Stop {
			reset_actor_rescheduling: true,
		})
		// This is ok, because runner_id changes every retry of outbound_req
		.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
		.to_workflow::<crate::workflows::runner::Workflow>()
		.tag("runner_id", runner_id)
		.send()
		.await;

	if let Some(WorkflowError::WorkflowNotFound) = res
		.as_ref()
		.err()
		.and_then(|x| x.chain().find_map(|x| x.downcast_ref::<WorkflowError>()))
	{
		tracing::warn!(
			?runner_id,
			"runner workflow not found, likely already stopped"
		);
	} else {
		res?;
	}

	Ok(())
}

/// Send a stop message to the client.
///
/// This will close the runner's WebSocket.
async fn publish_to_client_stop(ctx: &ActivityCtx, runner_id: Id) -> Result<()> {
	let receiver_subject = RunnerReceiverSubject::new(runner_id).to_string();

	let message_serialized = rivet_runner_protocol::versioned::ToClient::wrap_latest(
		rivet_runner_protocol::ToClient::ToClientClose,
	)
	.serialize_with_embedded_version(rivet_runner_protocol::PROTOCOL_VERSION)?;

	ctx.ups()?
		.publish(&receiver_subject, &message_serialized, PublishOpts::one())
		.await?;

	Ok(())
}

#[message("pegboard_serverless_connection_drain")]
pub struct Drain {}
