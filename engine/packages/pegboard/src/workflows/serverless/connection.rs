use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::StreamExt;
use gas::prelude::*;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_runner_protocol as protocol;
use rivet_types::runner_configs::RunnerConfigKind;
use tokio::time::Duration;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use super::{pool, runner};
use crate::pubsub_subjects::RunnerReceiverSubject;

const X_RIVET_ENDPOINT: HeaderName = HeaderName::from_static("x-rivet-endpoint");
const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
const X_RIVET_TOTAL_SLOTS: HeaderName = HeaderName::from_static("x-rivet-total-slots");
const X_RIVET_RUNNER_NAME: HeaderName = HeaderName::from_static("x-rivet-runner-name");
const X_RIVET_NAMESPACE_NAME: HeaderName = HeaderName::from_static("x-rivet-namespace-name");

const DRAIN_GRACE_PERIOD: Duration = Duration::from_secs(5);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub pool_wf_id: Id,
	pub runner_wf_id: Id,
	pub namespace_id: Id,
	pub runner_name: String,
}

#[workflow]
pub async fn pegboard_serverless_connection(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	// Run the connection activity, which will handle the full lifecycle
	let res = ctx
		.activity(OutboundReqInput {
			pool_wf_id: input.pool_wf_id,
			runner_wf_id: input.runner_wf_id,
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
		})
		.await?;

	// If we failed to send inline during the activity, durably ensure the
	// signal is dispatched here
	if res.send_drain_started {
		ctx.signal(pool::RunnerDrainStarted {
			runner_wf_id: input.runner_wf_id,
		})
		.to_workflow_id(input.pool_wf_id)
		.send()
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct OutboundReqInput {
	pool_wf_id: Id,
	runner_wf_id: Id,
	namespace_id: Id,
	runner_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutboundReqOutput {
	send_drain_started: bool,
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

	let (runner_config_res, namespace_res) = tokio::try_join!(
		ctx.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		}),
		ctx.op(namespace::ops::get_global::Input {
			namespace_ids: vec![input.namespace_id],
		})
	)?;
	let Some(runner_config) = runner_config_res.into_iter().next() else {
		tracing::debug!("runner config does not exist, ending outbound req");
		return Ok(OutboundReqOutput {
			send_drain_started: true,
		});
	};

	let RunnerConfigKind::Serverless {
		url,
		headers,
		slots_per_runner,
		request_lifespan,
		..
	} = runner_config.config.kind
	else {
		tracing::debug!("runner config is not serverless, ending outbound req");
		return Ok(OutboundReqOutput {
			send_drain_started: true,
		});
	};

	let namespace = namespace_res
		.into_iter()
		.next()
		.context("runner namespace not found")?;

	let current_dc = ctx.config().topology().current_dc()?;

	let token = if let Some(auth) = &ctx.config().auth {
		Some((
			X_RIVET_TOKEN,
			HeaderValue::try_from(auth.admin_token.read())?,
		))
	} else {
		None
	};

	let headers = headers
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
				HeaderValue::try_from(slots_per_runner)?,
			),
			(
				X_RIVET_RUNNER_NAME,
				HeaderValue::try_from(input.runner_name.clone())?,
			),
			(
				X_RIVET_NAMESPACE_NAME,
				HeaderValue::try_from(namespace.name.clone())?,
			),
			// Deprecated
			(
				HeaderName::from_static("x-rivet-namespace-id"),
				HeaderValue::try_from(namespace.name)?,
			),
		])
		.chain(token)
		.collect();

	let endpoint_url = format!("{}/start", url.trim_end_matches('/'));

	tracing::debug!(%endpoint_url, "sending outbound req");

	let client = rivet_pools::reqwest::client_no_timeout().await?;
	let req = client.get(endpoint_url).headers(headers);

	let mut source = sse::EventSource::new(req).context("failed creating event source")?;
	let mut runner_id = None;

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

	let sleep_until_drain =
		Duration::from_secs(request_lifespan as u64).saturating_sub(DRAIN_GRACE_PERIOD);
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
		.signal(pool::RunnerDrainStarted {
			runner_wf_id: input.runner_wf_id,
		})
		// This is ok, because we only send DrainStarted once
		.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
		.to_workflow_id(input.pool_wf_id)
		.send()
		.await
	{
		tracing::warn!(
			runner_name=%input.runner_name.clone(),
			namespace_id=%input.namespace_id,
			workflow_id=%ctx.workflow_id(),
			"failed to send signal: {}", e
		);

		// If we failed to send, have the workflow send it durably
		return Ok(OutboundReqOutput {
			send_drain_started: true,
		});
	}

	// After we tell the pool we're draining, any remaining failures
	// don't matter as the pool already stopped caring about us.
	if let Err(err) = finish_non_critical_draining(ctx, source, runner_id).await {
		tracing::debug!(?err, "failed non critical draining phase");
	}

	Ok(OutboundReqOutput {
		send_drain_started: false,
	})
}

async fn is_runner_draining(ctx: &ActivityCtx, runner_wf_id: Id) -> Result<bool> {
	let runner_wf = ctx
		.get_workflows(vec![runner_wf_id])
		.await?
		.into_iter()
		.next()
		.context("cannot find own runner wf")?;
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
