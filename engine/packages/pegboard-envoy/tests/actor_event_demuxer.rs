use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use gas::db::debug::DatabaseDebug;
use gas::prelude::*;
use pegboard_envoy::actor_event_demuxer::ActorEventDemuxer;
use rivet_envoy_protocol as protocol;

const ENVOY_KEY: &str = "demuxer-test-envoy";

fn state_event(actor_id: Id, index: i64, state: protocol::ActorState) -> protocol::EventWrapper {
	protocol::EventWrapper {
		checkpoint: protocol::ActorCheckpoint {
			actor_id: actor_id.to_string(),
			generation: 1,
			index,
		},
		inner: protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate { state }),
	}
}

async fn build_ctx(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> Result<StandaloneCtx> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	Ok(StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"actor_event_demuxer_test",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?)
}

/// The event indices the actor workflow has received, in signal order.
async fn delivered_indices(debug_db: &db::DatabaseKv, workflow_id: Id) -> Result<Vec<i64>> {
	let mut signals = debug_db
		.find_signals(&[], Some(workflow_id), Some("pegboard_actor2_events"), None)
		.await?;
	// Signal ids are random, so the debug query does not return signals in send order.
	signals.sort_by_key(|signal| signal.create_ts);
	let mut indices = Vec::new();
	for signal in signals {
		for event in signal.body["events"].as_array().context("events array")? {
			indices.push(
				event["checkpoint"]["index"]
					.as_i64()
					.context("event index")?,
			);
		}
	}
	Ok(indices)
}

#[tokio::test(flavor = "multi_thread")]
async fn events_dispatched_while_storage_is_unreachable_still_reach_the_actor() -> Result<()> {
	// Storage must be reachable over TCP so the test can cut the connection.
	// SAFETY: set before any other thread reads the environment.
	unsafe { std::env::set_var("RIVET_TEST_DATABASE", "postgres") };
	let test_deps = rivet_test_deps::TestDeps::new().await?;
	// Single-node Postgres allows one leader per database. Hand the lease to the
	// proxied pools below, which carry every storage call in this test.
	test_deps.pools().udb()?.shutdown().await;

	// Every storage call goes through a proxy that the test can disable.
	let rivet_config::config::Database::Postgres(postgres) = test_deps.config().database() else {
		anyhow::bail!("test database must be postgres");
	};
	let url = postgres.url.read().clone();
	let port: u16 = url
		.split_whitespace()
		.find_map(|part| part.strip_prefix("port="))
		.context("postgres port")?
		.parse()?;
	let network_faults = rivet_test_deps::ToxiproxyTestServer::start().await?;
	let proxy = network_faults
		.proxy("postgres", SocketAddr::from(([127, 0, 0, 1], port)))
		.await?;
	let mut root = (**test_deps.config()).clone();
	root.database = Some(rivet_config::config::Database::Postgres(
		rivet_config::config::db::Postgres {
			url: rivet_config::secret::Secret::new(url.replace(
				&format!("port={port}"),
				&format!("port={}", proxy.listen_addr().port()),
			)),
			..postgres.clone()
		},
	));
	let proxied_config = rivet_config::Config::from_root(root);
	let proxied_pools = rivet_pools::Pools::new(proxied_config.clone()).await?;
	let ctx = build_ctx(proxied_config.clone(), proxied_pools.clone()).await?;

	// A workflow with no worker keeps every signal it receives, so the test can read them.
	let actor_id = Id::new_v1(test_deps.config().dc_label());
	let workflow_id = ctx
		.workflow(pegboard::workflows::actor2::Input {
			actor_id,
			name: "demuxer-test".to_string(),
			pool_name: "demuxer-test".to_string(),
			key: None,
			namespace_id: Id::new_v1(test_deps.config().dc_label()),
			input: None,
			from_v1: false,
		})
		.tag("actor_id", actor_id)
		.dispatch()
		.await?;

	let mut demuxer = ActorEventDemuxer::new(ctx, ENVOY_KEY.to_string());

	proxy.disable().await?;
	demuxer.ingest(
		actor_id,
		state_event(actor_id, 1, protocol::ActorState::ActorStateRunning),
	);
	// Keep storage unreachable long enough for the dispatch of event 1 to fail.
	// A refused connection fails the transaction at once. The outage stays well
	// under the 10 second leader lease, so the leader keeps running.
	tokio::time::sleep(Duration::from_secs(2)).await;
	proxy.enable().await?;

	demuxer.ingest(
		actor_id,
		state_event(
			actor_id,
			2,
			protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
				code: protocol::StopCode::Ok,
				message: None,
			}),
		),
	);

	let debug_db = db::DatabaseKv::new(proxied_config, proxied_pools).await?;
	// Delivery is asynchronous and has no completion signal visible to the test,
	// so poll the workflow's signals until both events arrive.
	let delivered = tokio::time::timeout(Duration::from_secs(20), async {
		loop {
			let indices = delivered_indices(&debug_db, workflow_id).await?;
			if indices.len() >= 2 {
				return Ok::<_, anyhow::Error>(indices);
			}
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	})
	.await;

	demuxer.shutdown().await;
	let delivered = match delivered {
		Ok(indices) => indices?,
		Err(_) => delivered_indices(&debug_db, workflow_id).await?,
	};
	assert_eq!(
		delivered,
		vec![1, 2],
		"every event must reach the actor, in order"
	);
	Ok(())
}
