use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{FutureExt, StreamExt};
use gas::prelude::*;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_runner_protocol as protocol;
use rivet_types::runner_configs::RunnerConfigKind;
use std::time::Instant;
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct RescheduleState {
	last_retry_ts: i64,
	retry_count: usize,
}

#[workflow]
pub async fn pegboard_serverless_connection(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	// Run the connection activity, which will handle the full lifecycle
	let send_drain_started = ctx
		.loope(RescheduleState::default(), |ctx, state| {
			let input = input.clone();

			async move {
				let res = ctx
					.activity(OutboundReqInput {
						pool_wf_id: input.pool_wf_id,
						runner_wf_id: input.runner_wf_id,
						namespace_id: input.namespace_id,
						runner_name: input.runner_name.clone(),
					})
					.await?;

				if let OutboundReqOutput::Done(res) = res {
					return Ok(Loop::Break(res.send_drain_started));
				}

				let mut backoff = reconnect_backoff(
					state.retry_count,
					ctx.config().pegboard().serverless_base_retry_timeout(),
					ctx.config().pegboard().serverless_backoff_max_exponent(),
				);

				let retry_res = ctx
					.activity(CompareRetryInput {
						retry_count: state.retry_count,
						last_retry_ts: state.last_retry_ts,
					})
					.await?;

				state.retry_count = if retry_res.should_reset {
					0
				} else {
					state.retry_count + 1
				};
				state.last_retry_ts = retry_res.now;

				let next = backoff.step().expect("should not have max retry");
				if let Some(_sig) = ctx
					.listen_with_timeout::<DrainSignal>(Instant::from(next) - Instant::now())
					.await?
				{
					tracing::debug!("drain received during serverless connection backoff");

					// Notify parent that drain started
					return Ok(Loop::Break(true));
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	// If we failed to send inline during the activity, durably ensure the
	// signal is dispatched here
	if send_drain_started {
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
struct CompareRetryInput {
	retry_count: usize,
	last_retry_ts: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompareRetryOutput {
	should_reset: bool,
	now: i64,
}

#[activity(CompareRetry)]
async fn compare_retry(ctx: &ActivityCtx, input: &CompareRetryInput) -> Result<CompareRetryOutput> {
	let now = util::timestamp::now();

	// If the last retry ts is more than RETRY_RESET_DURATION_MS ago, reset retry count
	let should_reset =
		input.last_retry_ts < now - ctx.config().pegboard().serverless_retry_reset_duration();

	Ok(CompareRetryOutput { should_reset, now })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct OutboundReqInput {
	pool_wf_id: Id,
	runner_wf_id: Id,
	namespace_id: Id,
	runner_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutboundReqInnerOutput {
	send_drain_started: bool,
}

#[derive(Debug, Serialize, Deserialize)]
enum OutboundReqOutput {
	Done(OutboundReqInnerOutput),
	NeedsRetry,
}

#[activity(OutboundReq)]
#[timeout = u64::MAX]
async fn outbound_req(ctx: &ActivityCtx, input: &OutboundReqInput) -> Result<OutboundReqOutput> {
	match outbound_req_inner(ctx, input).await {
		Ok(res) => Ok(OutboundReqOutput::Done(res)),
		Err(error) => {
			tracing::error!(?error, "outbound_req_inner failed, retrying after backoff");
			Ok(OutboundReqOutput::NeedsRetry)
		}
	}
}

async fn outbound_req_inner(
	ctx: &ActivityCtx,
	input: &OutboundReqInput,
) -> Result<OutboundReqInnerOutput> {
	if is_runner_draining(ctx, input.runner_wf_id).await? {
		return Ok(OutboundReqInnerOutput {
			send_drain_started: true,
		});
	}

	let mut drain_sub = ctx
		.subscribe::<DrainMessage>(("workflow_id", ctx.workflow_id()))
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
		return Ok(OutboundReqInnerOutput {
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
		return Ok(OutboundReqInnerOutput {
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
	let mut runner_protocol_version = None;

	let runner_protocol_version2 = &mut runner_protocol_version;
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
							protocol::mk2::ToServerlessServer::ToServerlessServerInit(init) => {
								runner_id =
									Some(Id::parse(&init.runner_id).context("invalid runner id")?);
								*runner_protocol_version2 = Some(init.runner_protocol_version);
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
				Ok(_) => Ok(OutboundReqInnerOutput {
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
		return Ok(OutboundReqInnerOutput {
			send_drain_started: true,
		});
	}

	// After we tell the pool we're draining, any remaining failures
	// don't matter as the pool already stopped caring about us.
	if let Err(err) =
		finish_non_critical_draining(ctx, source, runner_id, runner_protocol_version).await
	{
		tracing::debug!(?err, "failed non critical draining phase");
	}

	Ok(OutboundReqInnerOutput {
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
	mut runner_protocol_version: Option<u16>,
) -> Result<()> {
	if let Some(runner_id) = runner_id {
		drain_runner(ctx, runner_id).await?;
	}

	// Continue waiting on req while draining
	let runner_protocol_version2 = &mut runner_protocol_version;
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
							protocol::mk2::ToServerlessServer::ToServerlessServerInit(init) => {
								let runner_id_local =
									Id::parse(&init.runner_id).context("invalid runner id")?;
								runner_id = Some(runner_id_local);
								*runner_protocol_version2 = Some(init.runner_protocol_version);
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
	if let (Some(runner_id), Some(runner_protocol_version)) = (runner_id, runner_protocol_version) {
		publish_to_client_stop(ctx, runner_id, runner_protocol_version).await?;
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
		.graceful_not_found()
		.send()
		.await?;

	if res.is_none() {
		// Retry with old runner wf
		let res = ctx
			.signal(crate::workflows::runner::Stop {
				reset_actor_rescheduling: true,
			})
			.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
			.to_workflow::<crate::workflows::runner::Workflow>()
			.tag("runner_id", runner_id)
			.graceful_not_found()
			.send()
			.await?;

		if res.is_none() {
			tracing::warn!(
				?runner_id,
				"runner workflow not found, likely already stopped"
			);
		}
	}

	Ok(())
}

/// Send a stop message to the client.
///
/// This will close the runner's WebSocket.
async fn publish_to_client_stop(
	ctx: &ActivityCtx,
	runner_id: Id,
	runner_protocol_version: u16,
) -> Result<()> {
	let receiver_subject = RunnerReceiverSubject::new(runner_id).to_string();

	let message_serialized = if protocol::is_mk2(runner_protocol_version) {
		protocol::versioned::ToRunnerMk2::wrap_latest(protocol::mk2::ToRunner::ToRunnerClose)
			.serialize_with_embedded_version(protocol::PROTOCOL_MK2_VERSION)?
	} else {
		protocol::versioned::ToRunner::wrap_latest(protocol::ToRunner::ToClientClose)
			.serialize_with_embedded_version(protocol::PROTOCOL_MK1_VERSION)?
	};

	ctx.ups()?
		.publish(&receiver_subject, &message_serialized, PublishOpts::one())
		.await?;

	Ok(())
}

#[message("pegboard_serverless_connection_drain_msg")]
pub struct DrainMessage {}

#[signal("pegboard_serverless_connection_drain_sig")]
pub struct DrainSignal {}

fn reconnect_backoff(
	retry_count: usize,
	base_retry_timeout: usize,
	max_exponent: usize,
) -> util::backoff::Backoff {
	util::backoff::Backoff::new_at(max_exponent, None, base_retry_timeout, 500, retry_count)
}
