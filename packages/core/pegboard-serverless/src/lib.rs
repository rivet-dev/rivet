use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use pegboard::keys;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest_eventsource as sse;
use rivet_runner_protocol as protocol;
use rivet_types::namespaces::RunnerConfig;
use tokio::{sync::oneshot, task::JoinHandle, time::Duration};
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;
use vbare::OwnedVersionedData;

const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");

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
		.await?;

	let runner_configs = ctx
		.op(namespace::ops::runner_config::get_global::Input {
			runners: serverless_data
				.iter()
				.map(|(ns_id, runner_name, _)| (*ns_id, runner_name.clone()))
				.collect(),
		})
		.await?;

	tracing::debug!(?serverless_data, ?runner_configs);

	for (ns_id, runner_name, desired_slots) in &serverless_data {
		let runner_config = runner_configs
			.iter()
			.find(|rc| rc.namespace_id == *ns_id)
			.context("runner config not found")?;

		let RunnerConfig::Serverless {
			url,
			headers,
			request_lifespan,
			slots_per_runner,
			min_runners,
			max_runners,
			runners_margin,
		} = &runner_config.config
		else {
			tracing::warn!(
				?ns_id,
				"this runner config should not be in the serverless subspace (wrong config kind)"
			);
			continue;
		};

		let curr = outbound_connections
			.entry((*ns_id, runner_name.clone()))
			.or_insert_with(Vec::new);

		// Remove finished and draining connections from list
		curr.retain(|conn| !conn.handle.is_finished() && !conn.draining.load(Ordering::SeqCst));

		let desired_count = (desired_slots.div_ceil(*slots_per_runner).max(*min_runners)
			+ *runners_margin)
			.min(*max_runners)
			.try_into()?;

		// Calculate diff
		let drain_count = curr.len().saturating_sub(desired_count);
		let start_count = desired_count.saturating_sub(curr.len());

		if drain_count != 0 {
			// TODO: Implement smart logic of draining runners with the lowest allocated actors
			let draining_connections = curr.split_off(desired_count);

			for conn in draining_connections {
				if conn.shutdown_tx.send(()).is_err() {
					tracing::warn!(
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
			)
		})
		.take(start_count);
		curr.extend(starting_connections);
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

fn spawn_connection(
	ctx: StandaloneCtx,
	url: String,
	headers: HashMap<String, String>,
	request_lifespan: Duration,
) -> OutboundConnection {
	let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
	let draining = Arc::new(AtomicBool::new(false));

	let draining2 = draining.clone();
	let handle = tokio::spawn(async move {
		if let Err(err) =
			outbound_handler(&ctx, url, headers, request_lifespan, shutdown_rx, draining2).await
		{
			tracing::error!(?err, "outbound req failed");

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
	shutdown_rx: oneshot::Receiver<()>,
	draining: Arc<AtomicBool>,
) -> Result<()> {
	let client = rivet_pools::reqwest::client_no_timeout().await?;
	let headers = headers
		.into_iter()
		.flat_map(|(k, v)| {
			// NOTE: This will filter out invalid headers without warning
			Some((
				k.parse::<HeaderName>().ok()?,
				v.parse::<HeaderValue>().ok()?,
			))
		})
		.collect();

	let mut req = client.get(url).headers(headers);

	// Add admin token if configured
	if let Some(auth) = &ctx.config().auth {
		req = req.header(X_RIVET_TOKEN, auth.admin_token.read());
	}

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
				Err(err) => return Err(err.into()),
			}
		}

		anyhow::Ok(())
	};

	tokio::select! {
		res = stream_handler => return res.map_err(Into::into),
		_ = tokio::time::sleep(request_lifespan) => {}
		_ = shutdown_rx => {}
	}

	draining.store(true, Ordering::SeqCst);

	ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
		.send()
		.await?;

	if let Some(runner_id) = runner_id {
		stop_runner(ctx, runner_id).await?;
	}

	// Continue waiting on req while draining
	while let Some(event) = source.next().await {
		match event {
			Ok(sse::Event::Open) => {}
			Ok(sse::Event::Message(msg)) => {
				tracing::debug!(%msg.data, "received outbound req message");

				// If runner_id is none at this point it means we did not send the stopping signal yet, so
				// send it now
				if runner_id.is_none() {
					stop_runner(ctx, Id::parse(&msg.data)?).await?;
				}
			}
			Err(sse::Error::StreamEnded) => break,
			Err(err) => return Err(err.into()),
		}
	}

	tracing::info!("outbound req stopped");

	Ok(())
}

async fn stop_runner(ctx: &StandaloneCtx, runner_id: Id) -> Result<()> {
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
