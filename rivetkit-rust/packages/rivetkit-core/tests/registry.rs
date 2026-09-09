use std::sync::Arc;

use super::*;
use crate::actor::config::ActorConfig;

#[test]
fn actor_metadata_does_not_request_legacy_kv_preload_probes() {
	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);

	let metadata = build_actor_metadata_map_from_factories(&factories);
	assert!(
		metadata["counter"].get("preload").is_none(),
		"kv-to-sqlite import is live-scan-only and must not request legacy KV preload probes"
	);
}

fn is_actor_active(state: Option<&ActorInstanceState>) -> bool {
	matches!(state, Some(ActorInstanceState::Active(_)))
}

/// Regression test for the production incident where a `CommandStopActor` addressed
/// to a previous generation was applied to a freshly-started newer generation.
///
/// A gen-49 `Lost` stop is parked before gen 50 starts. Because stops are now
/// generation-scoped, gen 50's startup must recognize the parked stop as stale and
/// leave gen 50 running instead of stopping itself.
#[tokio::test]
async fn stop_for_previous_generation_does_not_kill_freshly_started_generation() {
	use crate::actor::context::ActorContext;
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);
	let dispatcher = Arc::new(RegistryDispatcher::new(factories, false));

	let actor_id = "actor-preparked";

	// gen 49's `Lost` stop arrives while there is no active instance and is parked.
	dispatcher
		.stop_actor(
			actor_id,
			49,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("parking a stop with no active instance returns Ok");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&actor_id.to_owned())
			.await
			.is_some(),
		"gen-49 stop should be parked under the actor id",
	);

	// gen 50 is started to take over the actor on the same runner.
	let ctx = ActorContext::new(actor_id, "counter", Vec::new(), "local");
	dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 50,
			actor_name: "counter".to_owned(),
			input: None,
			ctx,
		})
		.await
		.expect("gen 50 should start successfully");

	// The stale gen-49 stop is discarded during startup rather than applied to gen 50.
	assert!(
		dispatcher
			.pending_stops
			.get_async(&actor_id.to_owned())
			.await
			.is_none(),
		"stale gen-49 stop should be cleared during gen 50 startup",
	);

	// gen 50 survives: the stop for a previous generation did not kill it.
	let is_active = is_actor_active(
		dispatcher
			.actor_instances
			.get_async(&actor_id.to_owned())
			.await
			.as_ref()
			.map(|entry| entry.get()),
	);
	assert!(
		is_active,
		"gen 50 must stay Active; a stop for gen 49 must not kill gen 50",
	);
}

/// Regression test for the exact logged ordering: a gen-49 stop arrives *while gen
/// 50 is still starting*. It parks, and gen 50's startup must treat it as stale and
/// keep running.
///
/// Uses a test-only startup gate (`start_actor` seam) to hold gen 50 in the
/// "starting" window so the stop is delivered mid-startup.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parked_previous_generation_stop_does_not_kill_starting_generation() {
	use std::time::Duration;

	use crate::actor::context::ActorContext;
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);
	let dispatcher = Arc::new(RegistryDispatcher::new(factories, false));

	let actor_id = "actor-gated";

	// Hold gen 50 in the starting window.
	test_hooks::arm_startup_gate(actor_id);
	let start_dispatcher = dispatcher.clone();
	let ctx = ActorContext::new(actor_id, "counter", Vec::new(), "local");
	let start_task = tokio::spawn(async move {
		start_dispatcher
			.start_actor(StartActorRequest {
				actor_id: actor_id.to_owned(),
				generation: 50,
				actor_name: "counter".to_owned(),
				input: None,
				ctx,
			})
			.await
	});

	// Wait until gen 50 has registered as starting (paused at the gate).
	tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			if dispatcher
				.starting_instances
				.get_async(&actor_id.to_owned())
				.await
				.is_some()
			{
				break;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("gen 50 should register as starting");

	// gen 49's `Lost` stop arrives while gen 50 is starting: it parks under the actor id.
	dispatcher
		.stop_actor(
			actor_id,
			49,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stop parks while gen 50 is starting");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&actor_id.to_owned())
			.await
			.is_some(),
		"gen-49 stop should be parked while gen 50 is starting",
	);

	// Release gen 50's startup; it must recognize the parked gen-49 stop as stale.
	test_hooks::release_startup_gate(actor_id);
	start_task
		.await
		.expect("start task joins")
		.expect("gen 50 should start successfully");

	assert!(
		dispatcher
			.pending_stops
			.get_async(&actor_id.to_owned())
			.await
			.is_none(),
		"stale gen-49 stop should be cleared during gen 50 startup",
	);
	let is_active = is_actor_active(
		dispatcher
			.actor_instances
			.get_async(&actor_id.to_owned())
			.await
			.as_ref()
			.map(|entry| entry.get()),
	);
	assert!(
		is_active,
		"gen 50 must stay Active; a gen-49 stop parked during its startup must not kill it",
	);
}

/// Guards against over-correction: a stop for the *current* generation must still
/// stop it. Prevents the generation-scoping fix from turning legitimate stops into
/// no-ops.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_for_current_generation_stops_it() {
	use crate::actor::context::ActorContext;
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);
	let dispatcher = Arc::new(RegistryDispatcher::new(factories, false));

	let actor_id = "actor-current";

	let ctx = ActorContext::new(actor_id, "counter", Vec::new(), "local");
	dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 50,
			actor_name: "counter".to_owned(),
			input: None,
			ctx,
		})
		.await
		.expect("gen 50 should start successfully");
	assert!(
		is_actor_active(
			dispatcher
				.actor_instances
				.get_async(&actor_id.to_owned())
				.await
				.as_ref()
				.map(|entry| entry.get()),
		),
		"gen 50 should be Active after starting",
	);

	// A stop for the matching generation (50) must stop the running instance.
	dispatcher
		.stop_actor(
			actor_id,
			50,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stopping the current generation should succeed");

	assert!(
		!is_actor_active(
			dispatcher
				.actor_instances
				.get_async(&actor_id.to_owned())
				.await
				.as_ref()
				.map(|entry| entry.get()),
		),
		"a stop for the current generation must stop it",
	);
}
