use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use namespace::types::RunnerKind;
use pegboard::keys;
use reqwest_eventsource as sse;
use rivet_runner_protocol::protocol;
use tokio::{sync::oneshot, task::JoinHandle, time::Duration};
use udb_util::{SNAPSHOT, TxnExt};
use universaldb::{self as udb, options::StreamingMode};

const OUTBOUND_REQUEST_LIFESPAN: Duration = Duration::from_secs(14 * 60 + 30);

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
		"pegboard-outbound",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let mut sub = ctx
		.subscribe::<pegboard::messages::BumpOutboundAutoscaler>(())
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
	let outbound_data = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());
			let outbound_desired_subspace =
				txs.subspace(&keys::ns::OutboundDesiredSlotsKey::subspace());

			txs.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&outbound_desired_subspace).into()
				},
				// NOTE: This is a snapshot to prevent conflict with updates to this subspace
				SNAPSHOT,
			)
			.map(|res| match res {
				Ok(entry) => {
					let (key, desired_slots) =
						txs.read_entry::<keys::ns::OutboundDesiredSlotsKey>(&entry)?;

					Ok((key.namespace_id, key.runner_name_selector, desired_slots))
				}
				Err(err) => Err(err.into()),
			})
			.try_collect::<Vec<_>>()
			.await

			// outbound/{ns_id}/{runner_name_selector}/desired_slots
		})
		.await?;

	let mut namespace_ids = outbound_data
		.iter()
		.map(|(ns_id, _, _)| *ns_id)
		.collect::<Vec<_>>();
	namespace_ids.dedup();

	let namespaces = ctx
		.op(namespace::ops::get_global::Input { namespace_ids })
		.await?;

	for (ns_id, runner_name_selector, desired_slots) in &outbound_data {
		let namespace = namespaces
			.iter()
			.find(|ns| ns.namespace_id == *ns_id)
			.context("ns not found")?;

		let RunnerKind::Outbound {
			url,
			slots_per_runner,
			min_runners,
			max_runners,
			runners_margin,
		} = &namespace.runner_kind
		else {
			tracing::warn!(
				?ns_id,
				"this namespace should not be in the outbound subspace (wrong runner kind)"
			);
			continue;
		};

		let curr = outbound_connections
			.entry((*ns_id, runner_name_selector.clone()))
			.or_insert_with(Vec::new);

		// Remove finished and draining connections from list
		curr.retain(|conn| !conn.handle.is_finished() && !conn.draining.load(Ordering::SeqCst));

		let desired_count = (desired_slots
			.div_ceil(*slots_per_runner)
			.max(*min_runners)
			.min(*max_runners)
			+ runners_margin)
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
						"outbound connection shutdown channel dropped, likely already stopped"
					);
				}
			}
		}

		let starting_connections =
			std::iter::repeat_with(|| spawn_connection(ctx.clone(), url.clone())).take(start_count);
		curr.extend(starting_connections);
	}

	// Remove entries that aren't returned from udb
	outbound_connections.retain(|(ns_id, runner_name_selector), _| {
		outbound_data
			.iter()
			.any(|(ns_id2, runner_name_selector2, _)| {
				ns_id == ns_id2 && runner_name_selector == runner_name_selector2
			})
	});

	Ok(())
}

fn spawn_connection(ctx: StandaloneCtx, url: String) -> OutboundConnection {
	let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
	let draining = Arc::new(AtomicBool::new(false));

	let draining2 = draining.clone();
	let handle = tokio::spawn(async move {
		if let Err(err) = outbound_handler(&ctx, url, shutdown_rx, draining2).await {
			tracing::error!(?err, "outbound req failed");

			// TODO: Add backoff
			tokio::time::sleep(Duration::from_secs(1)).await;

			// On error, bump the autoscaler loop again
			let _ = ctx
				.msg(pegboard::messages::BumpOutboundAutoscaler {})
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
	shutdown_rx: oneshot::Receiver<()>,
	draining: Arc<AtomicBool>,
) -> Result<()> {
	let client = rivet_pools::reqwest::client_no_timeout().await?;
	let mut es = sse::EventSource::new(client.get(url))?;
	let mut runner_id = None;

	let stream_handler = async {
		while let Some(event) = es.next().await {
			match event {
				Ok(sse::Event::Open) => {}
				Ok(sse::Event::Message(msg)) => {
					tracing::debug!(%msg.data, "received outbound req message");

					if runner_id.is_none() {
						runner_id = Some(Id::parse(&msg.data)?);
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
		_ = tokio::time::sleep(OUTBOUND_REQUEST_LIFESPAN) => {}
		_ = shutdown_rx => {}
	}

	draining.store(true, Ordering::SeqCst);

	ctx.msg(pegboard::messages::BumpOutboundAutoscaler {})
		.send()
		.await?;

	if let Some(runner_id) = runner_id {
		stop_runner(ctx, runner_id).await?;
	}

	// Continue waiting on req while draining
	while let Some(event) = es.next().await {
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
		.signal(protocol::ToServer::Stopping)
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
