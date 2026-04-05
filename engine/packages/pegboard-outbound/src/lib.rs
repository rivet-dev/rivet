use anyhow::Result;
use futures_util::StreamExt;
use gas::prelude::*;
use pegboard::pubsub_subjects::ServerlessOutboundSubject;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use rivet_runtime::TermSignal;
use rivet_types::actor::RunnerPoolError;
use rivet_types::runner_configs::RunnerConfigKind;
use std::collections::HashMap;
use std::time::Duration;
use tokio::task::JoinHandle;
use universalpubsub::NextOutput;
use vbare::OwnedVersionedData;

mod metrics;

const X_RIVET_ENDPOINT: HeaderName = HeaderName::from_static("x-rivet-endpoint");
const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
const X_RIVET_POOL_NAME: HeaderName = HeaderName::from_static("x-rivet-pool-name");
const X_RIVET_NAMESPACE_NAME: HeaderName = HeaderName::from_static("x-rivet-namespace-name");

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"pegboard_outbound",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let mut conns = Vec::new();

	let res = inner(&ctx, &mut conns).await;

	if res.is_err() {
		// Abort all futures in the event of a fatal error. This is not ideal
		for conn in &conns {
			conn.handle.abort();
		}
	}

	// Wait for remaining conns to stop
	futures_util::future::join_all(conns.into_iter().map(|c| c.handle).collect::<Vec<_>>()).await;

	res
}

async fn inner(ctx: &StandaloneCtx, conns: &mut Vec<OutboundHandler>) -> Result<()> {
	let mut sub = ctx
		.ups()?
		.queue_subscribe(&ServerlessOutboundSubject.to_string(), "service")
		.await?;
	let mut term_signal = TermSignal::get();

	loop {
		tokio::select! {
			msg = sub.next() => {
				match msg? {
					NextOutput::Message(msg) => {
						match protocol::versioned::ToOutbound::deserialize_with_embedded_version(&msg.payload) {
							Ok(packet) => {
								// Clean up finished conns
								conns.retain(|c| !c.handle.is_finished());

								conns.push(OutboundHandler::new(ctx, packet))
							}
							Err(err) => {
								tracing::error!(?err, "received invalid outbound message");
							}
						}
					},
					NextOutput::Unsubscribed => bail!("outbound sub unsubscribed"),
				}
			}
			_ = term_signal.recv() => return Ok(()),
		}
	}
}

// Handles serverless (/start request)
struct OutboundHandler {
	handle: JoinHandle<()>,
}

impl OutboundHandler {
	fn new(ctx: &StandaloneCtx, packet: protocol::ToOutbound) -> Self {
		let ctx = ctx.clone();
		let handle = tokio::spawn(async move {
			if let Err(err) = handle(&ctx, packet).await {
				tracing::error!(?err, "outbound handler failed");
			}
		});

		OutboundHandler { handle }
	}
}

async fn handle(ctx: &StandaloneCtx, packet: protocol::ToOutbound) -> Result<()> {
	let (namespace_id, pool_name, checkpoint, actor_config) = match packet {
		protocol::ToOutbound::ToOutboundActorStart(protocol::ToOutboundActorStart {
			namespace_id,
			pool_name,
			checkpoint,
			actor_config,
		}) => (namespace_id, pool_name, checkpoint, actor_config),
	};
	let namespace_id = Id::parse(&namespace_id)?;
	let actor_id = Id::parse(&checkpoint.actor_id)?;
	let generation = checkpoint.generation;

	// Check pool
	let (pool_res, namespace_res) = tokio::try_join!(
		ctx.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(namespace_id, pool_name.clone())],
			bypass_cache: false,
		}),
		ctx.op(namespace::ops::get_global::Input {
			namespace_ids: vec![namespace_id],
		}),
	)?;
	let Some(pool) = pool_res.into_iter().next() else {
		tracing::debug!("pool does not exist, ending outbound handler");
		return Ok(());
	};
	let Some(namespace) = namespace_res.into_iter().next() else {
		tracing::error!("namespace not found, ending outbound handler");
		report_error(
			ctx,
			namespace_id,
			&pool_name,
			RunnerPoolError::InternalError,
		)
		.await;
		return Ok(());
	};

	let payload = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyCommands(vec![
		protocol::CommandWrapper {
			checkpoint,
			inner: protocol::Command::CommandStartActor(protocol::CommandStartActor {
				config: actor_config,
				// Empty because request ids are ephemeral. This is intercepted by guard and
				// populated before it reaches the envoy
				hibernating_requests: Vec::new(),
			}),
		},
	]))
	.serialize_with_embedded_version(PROTOCOL_VERSION)?;

	let RunnerConfigKind::Serverless {
		url,
		headers,
		request_lifespan,
		..
	} = pool.config.kind
	else {
		tracing::warn!(
			?actor_id,
			"config no longer serverless, ignoring outbound allocation"
		);
		return Ok(());
	};

	// Send ack to actor wf before starting an outbound req
	ctx.signal(pegboard::workflows::actor2::Allocated { generation })
		.to_workflow::<pegboard::workflows::actor2::Workflow>()
		.tag("actor_id", &actor_id)
		.send()
		.await?;

	metrics::REQ_ACTIVE
		.with_label_values(&[&namespace_id.to_string(), &pool_name])
		.inc();

	let res = serverless_outbound_req(
		ctx,
		namespace_id,
		&pool_name,
		&namespace.name,
		actor_id,
		generation,
		payload,
		&url,
		headers,
		request_lifespan,
	)
	.await;

	metrics::REQ_ACTIVE
		.with_label_values(&[&namespace_id.to_string(), &pool_name])
		.dec();

	res
}

async fn serverless_outbound_req(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	pool_name: &str,
	namespace_name: &str,
	actor_id: Id,
	generation: u32,
	payload: Vec<u8>,
	url: &str,
	headers: HashMap<String, String>,
	request_lifespan: u32,
) -> Result<()> {
	let current_dc = ctx.config().topology().current_dc()?;
	let mut term_signal = TermSignal::get();

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
			(X_RIVET_POOL_NAME, HeaderValue::try_from(pool_name)?),
			(
				X_RIVET_NAMESPACE_NAME,
				HeaderValue::try_from(namespace_name)?,
			),
		])
		.chain(token)
		.collect();

	let endpoint_url = format!("{}/start", url.trim_end_matches('/'));

	tracing::debug!(%endpoint_url, "sending outbound req");

	let client = rivet_pools::reqwest::client_no_timeout().await?;
	let req = client.post(endpoint_url).body(payload).headers(headers);

	let mut source = sse::EventSource::new(req).context("failed creating event source")?;

	let stream_handler = async {
		while let Some(event) = source.next().await {
			match event {
				Ok(event) => match event {
					sse::Event::Open => {}
					sse::Event::Message(msg) => match msg.event.as_str() {
						"ping" => {}
						event => {
							tracing::warn!(
								event,
								"received unknown serverless sse message event kind"
							);
						}
					},
				},
				Err(sse::Error::StreamEnded) => {
					tracing::debug!("outbound req stopped early");

					report_error(
						ctx,
						namespace_id,
						&pool_name,
						RunnerPoolError::ServerlessStreamEndedEarly,
					)
					.await;

					return Ok(());
				}
				Err(sse::Error::InvalidStatusCode(code, res)) => {
					let body = res
						.text()
						.await
						.unwrap_or_else(|_| "<could not read body>".to_string());
					let body_slice = util::safe_slice(&body, 0, 512).to_string();

					report_error(
						ctx,
						namespace_id,
						&pool_name,
						RunnerPoolError::ServerlessHttpError {
							status_code: code.as_u16(),
							body: body_slice.clone(),
						},
					)
					.await;

					bail!("invalid status code ({code}):\n{body_slice}");
				}
				Err(reqwest_eventsource::Error::InvalidContentType(value, _)) => {
					report_error(
						ctx,
						namespace_id,
						&pool_name,
						RunnerPoolError::ServerlessConnectionError {
							message: format!(
								"expected Content-Type header to be text/event-stream, received {value:?}"
							),
						},
					)
					.await;

					bail!("invalid content type: {value:?}");
				}
				Err(err) => {
					let wrapped_err = anyhow::Error::from(err);

					report_error(
						ctx,
						namespace_id,
						&pool_name,
						RunnerPoolError::ServerlessConnectionError {
							// Print entire error chain
							message: wrapped_err
								.chain()
								.map(|err| err.to_string())
								.collect::<Vec<_>>()
								.join("\n"),
						},
					)
					.await;

					return Err(wrapped_err);
				}
			}
		}

		anyhow::Ok(())
	};

	metrics::REQ_TOTAL
		.with_label_values(&[namespace_id.to_string().as_str(), pool_name])
		.inc();

	let sleep_until_drain = Duration::from_secs(request_lifespan as u64).saturating_sub(
		Duration::from_millis(ctx.config().pegboard().serverless_drain_grace_period()),
	);
	tokio::select! {
		res = stream_handler => {
			// Mark actor as lost for immediate reallocation if SSE errors
			if res.is_err() {
				let res = ctx.signal(pegboard::workflows::actor2::Lost {
						generation,
						reason: pegboard::workflows::actor2::LostReason::EnvoyConnectionLost,
					})
					.to_workflow::<pegboard::workflows::actor2::Workflow>()
					.tag("actor_id", &actor_id)
					.graceful_not_found()
					.send()
					.await?;

				if res.is_none() {
					tracing::warn!(
						?actor_id,
						"actor workflow not found for lost signal"
					);
				}
			}

			return res;
		},
		_ = tokio::time::sleep(sleep_until_drain) => {}
		_ = term_signal.recv() => {}
	}

	tracing::debug!("connection reached lifespan, starting drain");

	// Start actor reallocation
	let res = ctx
		.signal(pegboard::workflows::actor2::GoingAway { generation })
		.to_workflow::<pegboard::workflows::actor2::Workflow>()
		.tag("actor_id", &actor_id)
		.graceful_not_found()
		.send()
		.await?;

	if res.is_none() {
		tracing::warn!(?actor_id, "actor workflow not found for going away signal");
	}

	// Wait for the grace period
	tokio::time::sleep(Duration::from_millis(
		ctx.config().pegboard().serverless_drain_grace_period(),
	))
	.await;

	tracing::debug!("outbound req stopped");

	Ok(())
}

/// Report an error to the error tracker workflow.
async fn report_error(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	pool_name: &str,
	error: RunnerPoolError,
) {
	if let Err(err) = ctx
		.signal(pegboard::workflows::runner_pool_error_tracker::ReportError { error })
		.to_workflow::<pegboard::workflows::runner_pool_error_tracker::Workflow>()
		.tag("namespace_id", namespace_id)
		.tag("runner_name", pool_name)
		.graceful_not_found()
		.send()
		.await
	{
		tracing::warn!(?err, "failed to report serverless error");
	}
}
