use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use pegboard::keys;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_runner_protocol as protocol;
use rivet_types::runner_configs::RunnerConfigKind;
use tokio::{sync::oneshot, task::JoinHandle, time::Duration};
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

const X_RIVET_ENDPOINT: HeaderName = HeaderName::from_static("x-rivet-endpoint");
const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
const X_RIVET_TOTAL_SLOTS: HeaderName = HeaderName::from_static("x-rivet-total-slots");
const X_RIVET_RUNNER_NAME: HeaderName = HeaderName::from_static("x-rivet-runner-name");
const X_RIVET_NAMESPACE_NAME: HeaderName = HeaderName::from_static("x-rivet-namespace-name");

const DRAIN_GRACE_PERIOD: Duration = Duration::from_secs(5);

struct OutboundConnection {
	handle: JoinHandle<()>,
	shutdown_tx: oneshot::Sender<()>,
	draining: Arc<AtomicBool>,
}

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"pegboard-serverless",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let mut sub = ctx
		.subscribe::<rivet_types::msgs::pegboard::BumpServerlessAutoscaler>(())
		.await?;
	let mut outbound_connections = HashMap::new();

	loop {
		tick(&ctx, &mut outbound_connections).await?;

		sub.next().await?;
	}
}

async fn tick(
	ctx: &StandaloneCtx,
	outbound_connections: &mut HashMap<(Id, String), Vec<OutboundConnection>>,
) -> Result<()> {
	let serverless_data = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let serverless_desired_subspace = keys::subspace().subspace(
				&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::entire_subspace(),
			);

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&serverless_desired_subspace).into()
				},
				// NOTE: This is a snapshot to prevent conflict with updates to this subspace
				Snapshot,
			)
			.map(|res| match res {
				Ok(entry) => {
					let (key, desired_slots) =
						tx.read_entry::<rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey>(&entry)?;

					Ok((key.namespace_id, key.runner_name, desired_slots))
				}
				Err(err) => Err(err.into()),
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("tick_tx"))
		.await?;

	let runner_configs = ctx
		.op(namespace::ops::runner_config::get::Input {
			runners: serverless_data
				.iter()
				.map(|(ns_id, runner_name, _)| (*ns_id, runner_name.clone()))
				.collect(),
			bypass_cache: true,
		})
		.await?;

	// Process each runner config with error handling
	for (ns_id, runner_name, desired_slots) in &serverless_data {
		let runner_config = runner_configs
			.iter()
			.find(|rc| rc.namespace_id == *ns_id && &rc.name == runner_name);

		let Some(runner_config) = runner_config else {
			tracing::debug!(
				?ns_id,
				?runner_name,
				"runner config not found, likely deleted"
			);
			continue;
		};

		if let Err(err) = tick_runner_config(
			ctx,
			*ns_id,
			runner_name.clone(),
			*desired_slots,
			runner_config,
			outbound_connections,
		)
		.await
		{
			tracing::error!(
				?ns_id,
				?runner_name,
				?err,
				"failed to process runner config, continuing with others"
			);
			// Continue processing other runner configs even if this one failed
			continue;
		}
	}

	// Remove entries that aren't returned from udb
	outbound_connections.retain(|(ns_id, runner_name), _| {
		serverless_data
			.iter()
			.any(|(ns_id2, runner_name2, _)| ns_id == ns_id2 && runner_name == runner_name2)
	});

	tracing::debug!(
		connection_counts=?outbound_connections.iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>(),
	);

	Ok(())
}

async fn tick_runner_config(
	ctx: &StandaloneCtx,
	ns_id: Id,
	runner_name: String,
	desired_slots: i64,
	runner_config: &namespace::ops::runner_config::get::RunnerConfig,
	outbound_connections: &mut HashMap<(Id, String), Vec<OutboundConnection>>,
) -> Result<()> {
	let namespace = ctx
		.op(namespace::ops::get_global::Input {
			namespace_ids: vec![ns_id.clone()],
		})
		.await
		.context("runner namespace not found")?;
	let namespace = namespace.first().context("runner namespace not found")?;
	let namespace_name = &namespace.name;

	let RunnerConfigKind::Serverless {
		url,
		headers,
		request_lifespan,
		slots_per_runner,
		min_runners,
		max_runners,
		runners_margin,
	} = &runner_config.config.kind
	else {
		tracing::debug!("not serverless config");
		return Ok(());
	};

	let curr = outbound_connections
		.entry((ns_id, runner_name.clone()))
		.or_insert_with(Vec::new);

	// Remove finished and draining connections from list
	curr.retain(|conn| !conn.handle.is_finished() && !conn.draining.load(Ordering::SeqCst));

	// Log warning and reset to 0 if negative
	let adjusted_desired_slots = if desired_slots < 0 {
		tracing::error!(
			?ns_id,
			?runner_name,
			?desired_slots,
			"negative desired slots, scaling to 0"
		);
		0
	} else {
		desired_slots
	};

	let desired_count =
		(rivet_util::math::div_ceil_i64(adjusted_desired_slots, *slots_per_runner as i64)
			.max(*min_runners as i64)
			+ *runners_margin as i64)
			.min(*max_runners as i64)
			.try_into()?;

	// Calculate diff
	let drain_count = curr.len().saturating_sub(desired_count);
	let start_count = desired_count.saturating_sub(curr.len());

	tracing::debug!(%namespace_name, %runner_name, %desired_count, %drain_count, %start_count, "scaling");

	if drain_count != 0 {
		// TODO: Implement smart logic of draining runners with the lowest allocated actors
		let draining_connections = curr.split_off(desired_count);

		for conn in draining_connections {
			if conn.shutdown_tx.send(()).is_err() {
				tracing::debug!(
					"serverless connection shutdown channel dropped, likely already stopped"
				);
			}
		}
	}

	let starting_connections = std::iter::repeat_with(|| {
		spawn_connection(
			ctx.clone(),
			url.clone(),
			headers.clone(),
			Duration::from_secs(*request_lifespan as u64),
			*slots_per_runner,
			runner_name.clone(),
			namespace_name.clone(),
		)
	})
	.take(start_count);
	curr.extend(starting_connections);

	Ok(())
}

fn spawn_connection(
	ctx: StandaloneCtx,
	url: String,
	headers: HashMap<String, String>,
	request_lifespan: Duration,
	slots_per_runner: u32,
	runner_name: String,
	namespace_name: String,
) -> OutboundConnection {
	let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
	let draining = Arc::new(AtomicBool::new(false));

	let draining2 = draining.clone();
	let handle = tokio::spawn(async move {
		if let Err(err) = outbound_handler(
			&ctx,
			url,
			headers,
			request_lifespan,
			slots_per_runner,
			runner_name,
			namespace_name,
			shutdown_rx,
			draining2,
		)
		.await
		{
			tracing::warn!(?err, "outbound req failed");

			// TODO: Add backoff
			tokio::time::sleep(Duration::from_secs(1)).await;

			// On error, bump the autoscaler loop again
			let _ = ctx
				.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
				.send()
				.await;
		}
	});

	OutboundConnection {
		handle,
		shutdown_tx,
		draining,
	}
}

async fn outbound_handler(
	ctx: &StandaloneCtx,
	url: String,
	headers: HashMap<String, String>,
	request_lifespan: Duration,
	slots_per_runner: u32,
	runner_name: String,
	namespace_name: String,
	shutdown_rx: oneshot::Receiver<()>,
	draining: Arc<AtomicBool>,
) -> Result<()> {
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
			(X_RIVET_RUNNER_NAME, HeaderValue::try_from(runner_name)?),
			(
				X_RIVET_NAMESPACE_NAME,
				HeaderValue::try_from(namespace_name.clone())?,
			),
			// Deprecated
			(
				HeaderName::from_static("x-rivet-namespace-id"),
				HeaderValue::try_from(namespace_name)?,
			),
		])
		.chain(token)
		.collect();

	let endpoint_url = format!("{}/start", url.trim_end_matches('/'));
	tracing::debug!(%endpoint_url, "sending outbound req");
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
					tracing::debug!(?runner_id, "outbound req stopped early");

					return Ok(());
				}
				Err(err) => return Err(err.into()),
			}
		}

		anyhow::Ok(())
	};

	let sleep_until_drop = request_lifespan.saturating_sub(DRAIN_GRACE_PERIOD);
	tokio::select! {
		res = stream_handler => return res.map_err(Into::into),
		_ = tokio::time::sleep(sleep_until_drop) => {}
		_ = shutdown_rx => {}
	}

	draining.store(true, Ordering::SeqCst);

	ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
		.send()
		.await?;

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

async fn drain_runner(ctx: &StandaloneCtx, runner_id: Id) -> Result<()> {
	let res = ctx
		.signal(pegboard::workflows::runner::Forward {
			inner: protocol::ToServer::ToServerStopping,
		})
		.to_workflow::<pegboard::workflows::runner::Workflow>()
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
async fn publish_to_client_stop(ctx: &StandaloneCtx, runner_id: Id) -> Result<()> {
	let receiver_subject =
		pegboard::pubsub_subjects::RunnerReceiverSubject::new(runner_id).to_string();

	let message_serialized = rivet_runner_protocol::versioned::ToClient::latest(
		rivet_runner_protocol::ToClient::ToClientClose,
	)
	.serialize_with_embedded_version(rivet_runner_protocol::PROTOCOL_VERSION)?;

	ctx.ups()?
		.publish(&receiver_subject, &message_serialized, PublishOpts::one())
		.await?;

	Ok(())
}
