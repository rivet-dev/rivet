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
			.get_async(&instance_key(actor_id, 49))
			.await
			.is_some(),
		"gen-49 stop should be parked for gen 49",
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
			.get_async(&instance_key(actor_id, 49))
			.await
			.is_none(),
		"stale gen-49 stop should be cleared during gen 50 startup",
	);

	// gen 50 survives: the stop for a previous generation did not kill it.
	let is_active = is_actor_active(
		dispatcher
			.actor_instances
			.get_async(&instance_key(actor_id, 50))
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
				.get_async(&instance_key(actor_id, 50))
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

	// gen 49's `Lost` stop arrives while gen 50 is starting and gen 49 has no record: it parks.
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
			.get_async(&instance_key(actor_id, 49))
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
			.get_async(&instance_key(actor_id, 49))
			.await
			.is_none(),
		"stale gen-49 stop should be cleared during gen 50 startup",
	);
	let is_active = is_actor_active(
		dispatcher
			.actor_instances
			.get_async(&instance_key(actor_id, 50))
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
				.get_async(&instance_key(actor_id, 50))
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
				.get_async(&instance_key(actor_id, 50))
				.await
				.as_ref()
				.map(|entry| entry.get()),
		),
		"a stop for the current generation must stop it",
	);
}

fn counter_dispatcher() -> Arc<RegistryDispatcher> {
	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), |_start| {
			Box::pin(async { Ok(()) })
		})),
	);
	Arc::new(RegistryDispatcher::new(factories, false))
}

async fn start_counter(
	dispatcher: &Arc<RegistryDispatcher>,
	actor_id: &str,
	generation: u32,
) -> crate::actor::context::ActorContext {
	let ctx = crate::actor::context::ActorContext::new(actor_id, "counter", Vec::new(), "local");
	dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation,
			actor_name: "counter".to_owned(),
			input: None,
			ctx: ctx.clone(),
		})
		.await
		.expect("generation should start");
	ctx
}

async fn instance_generation(
	dispatcher: &RegistryDispatcher,
	actor_id: &str,
	generation: u32,
) -> Option<u32> {
	dispatcher
		.actor_instances
		.get_async(&instance_key(actor_id, generation))
		.await
		.map(|entry| entry.get().instance().generation)
}

/// A Lost stop for generation N can reach the registry after N+1 has registered, because N+1
/// only waits for N's task to finish, not for N's stop. The stop must still reach N instead of
/// being dropped as stale, and it must leave N+1 routed.
#[tokio::test(start_paused = true)]
async fn stop_for_older_generation_after_newer_registered_stops_the_older_generation() {
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-overlap";
	let old_ctx = start_counter(&dispatcher, actor_id, 1).await;
	let new_ctx = start_counter(&dispatcher, actor_id, 2).await;
	assert_eq!(instance_generation(&dispatcher, actor_id, 1).await, Some(1));
	assert_eq!(dispatcher.current_generation(actor_id).await, Some(2));

	dispatcher
		.stop_actor(
			actor_id,
			1,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stopping the older generation should succeed");

	assert!(
		old_ctx.is_lost(),
		"the older generation must receive the Lost stop"
	);
	assert_eq!(
		instance_generation(&dispatcher, actor_id, 1).await,
		None,
		"the older generation's record is removed after its teardown",
	);
	assert!(
		!new_ctx.is_lost(),
		"the newer generation must not be touched"
	);
	assert_eq!(instance_generation(&dispatcher, actor_id, 2).await, Some(2));
	assert_eq!(dispatcher.current_generation(actor_id).await, Some(2));
	let routed = dispatcher
		.active_actor(actor_id)
		.await
		.expect("dispatch should route to the newer generation");
	assert_eq!(routed.generation, 2);
}

/// A stop for a registered generation must stop it immediately even while the next
/// generation is starting. Only a stop for the starting generation parks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parked_stops_are_scoped_to_the_starting_generation() {
	use std::time::Duration;

	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-scoped-park";
	let old_ctx = start_counter(&dispatcher, actor_id, 1).await;

	test_hooks::arm_startup_gate(actor_id);
	let start_dispatcher = dispatcher.clone();
	let new_ctx =
		crate::actor::context::ActorContext::new(actor_id, "counter", Vec::new(), "local");
	let start_ctx = new_ctx.clone();
	let start_task = tokio::spawn(async move {
		start_dispatcher
			.start_actor(StartActorRequest {
				actor_id: actor_id.to_owned(),
				generation: 2,
				actor_name: "counter".to_owned(),
				input: None,
				ctx: start_ctx,
			})
			.await
	});
	tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			if dispatcher
				.starting_instances
				.get_async(&instance_key(actor_id, 2))
				.await
				.is_some()
			{
				break;
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("gen 2 should register as starting");

	dispatcher
		.stop_actor(
			actor_id,
			1,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stopping gen 1 should succeed");
	assert!(old_ctx.is_lost(), "gen 1 must be stopped, not parked");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&instance_key(actor_id, 1))
			.await
			.is_none(),
		"the gen-1 stop must not park behind gen 2's startup",
	);
	assert_eq!(instance_generation(&dispatcher, actor_id, 1).await, None);

	dispatcher
		.stop_actor(
			actor_id,
			2,
			StopActorReason::SleepIntent,
			ActorStopHandle::detached(),
		)
		.await
		.expect("a stop for the starting generation parks");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&instance_key(actor_id, 2))
			.await
			.is_some(),
		"the gen-2 stop parks for gen 2's startup",
	);

	test_hooks::release_startup_gate(actor_id);
	start_task
		.await
		.expect("start task joins")
		.expect("gen 2 should start");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&instance_key(actor_id, 2))
			.await
			.is_none(),
		"gen 2's startup consumes its parked stop",
	);
	assert!(!new_ctx.is_lost());
}

/// A stop for a generation that already finished while a newer generation is registered
/// completes immediately instead of parking forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_for_finished_older_generation_completes_immediately() {
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-finished";
	start_counter(&dispatcher, actor_id, 2).await;

	dispatcher
		.stop_actor(
			actor_id,
			1,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("a stale stop returns Ok");
	assert!(
		dispatcher
			.pending_stops
			.get_async(&instance_key(actor_id, 1))
			.await
			.is_none(),
		"a stop for a finished older generation must not park",
	);
	assert_eq!(instance_generation(&dispatcher, actor_id, 2).await, Some(2));
}

/// A generation starting while an older generation of the same actor still runs on this runner
/// marks the older one lost and waits for its task to finish, so the two never overlap.
#[tokio::test(start_paused = true)]
async fn newer_generation_waits_for_older_generation_to_finish() {
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-gated-by-older";
	let old_ctx = start_counter(&dispatcher, actor_id, 1).await;
	assert!(!old_ctx.is_generation_finished());

	start_counter(&dispatcher, actor_id, 2).await;

	assert!(
		old_ctx.is_lost(),
		"the older generation must be marked lost"
	);
	assert!(
		old_ctx.is_generation_finished(),
		"the newer generation must not start before the older task finished",
	);
	assert_eq!(instance_generation(&dispatcher, actor_id, 2).await, Some(2));
	assert_eq!(dispatcher.current_generation(actor_id).await, Some(2));

	// The engine's Lost stop for the older generation still removes its record.
	dispatcher
		.stop_actor(
			actor_id,
			1,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stopping the older generation should succeed");
	assert_eq!(instance_generation(&dispatcher, actor_id, 1).await, None);
	assert_eq!(instance_generation(&dispatcher, actor_id, 2).await, Some(2));
}

/// If an older generation never finishes, the newer one fails to start instead of overlapping
/// it. A generation held in startup models a stuck older task.
#[tokio::test(start_paused = true)]
async fn newer_generation_fails_closed_when_older_generation_never_finishes() {
	use std::time::Duration;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-stuck-older";

	test_hooks::arm_startup_gate(actor_id);
	let old_ctx =
		crate::actor::context::ActorContext::new(actor_id, "counter", Vec::new(), "local");
	let start_dispatcher = dispatcher.clone();
	let start_old_ctx = old_ctx.clone();
	let old_start = tokio::spawn(async move {
		start_dispatcher
			.start_actor(StartActorRequest {
				actor_id: actor_id.to_owned(),
				generation: 1,
				actor_name: "counter".to_owned(),
				input: None,
				ctx: start_old_ctx,
			})
			.await
	});
	tokio::time::timeout(Duration::from_secs(1), async {
		while dispatcher
			.starting_instances
			.get_async(&instance_key(actor_id, 1))
			.await
			.is_none()
		{
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("gen 1 should register as starting");

	let new_ctx =
		crate::actor::context::ActorContext::new(actor_id, "counter", Vec::new(), "local");
	let error = dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 2,
			actor_name: "counter".to_owned(),
			input: None,
			ctx: new_ctx.clone(),
		})
		.await
		.expect_err("gen 2 must not start while gen 1 is still running");
	assert!(
		format!("{error:#}").contains("older generation of this actor is still running"),
		"unexpected error: {error:#}"
	);
	assert!(
		old_ctx.is_lost(),
		"the stuck older generation is marked lost"
	);
	assert!(new_ctx.is_generation_finished());
	assert!(
		dispatcher
			.starting_instances
			.get_async(&instance_key(actor_id, 2))
			.await
			.is_none(),
		"the refused generation must not stay registered as starting",
	);

	// Released, the lost older generation refuses to start.
	test_hooks::release_startup_gate(actor_id);
	old_start
		.await
		.expect("gen 1 start joins")
		.expect_err("a generation lost before starting must not start");
	assert!(old_ctx.is_generation_finished());
}

/// A generation lost while its runtime preamble hangs drops its start reply. The registry must
/// keep it until its task finishes aborting, so envoy-client does not report it stopped early.
#[tokio::test(start_paused = true)]
async fn lost_startup_is_released_only_after_its_task_finishes() {
	use std::time::Duration;

	let (entered_tx, entered_rx) = oneshot::channel::<()>();
	let entered_tx = Arc::new(std::sync::Mutex::new(Some(entered_tx)));
	let mut factories = HashMap::new();
	factories.insert(
		"stuck".to_owned(),
		Arc::new(ActorFactory::new_with_manual_startup_ready(
			ActorConfig::default(),
			move |mut start| {
				let entered_tx = entered_tx.clone();
				Box::pin(async move {
					let _startup_ready = start.startup_ready.take();
					if let Some(entered_tx) = entered_tx.lock().expect("entered lock").take() {
						let _ = entered_tx.send(());
					}
					std::future::pending::<()>().await;
					Ok(())
				})
			},
		)),
	);
	let dispatcher = Arc::new(RegistryDispatcher::new(factories, false));
	let actor_id = "actor-lost-startup";
	let ctx = crate::actor::context::ActorContext::new(actor_id, "stuck", Vec::new(), "local");
	let start_dispatcher = dispatcher.clone();
	let start_ctx = ctx.clone();
	let start = tokio::spawn(async move {
		start_dispatcher
			.start_actor(StartActorRequest {
				actor_id: actor_id.to_owned(),
				generation: 1,
				actor_name: "stuck".to_owned(),
				input: None,
				ctx: start_ctx,
			})
			.await
	});
	entered_rx.await.expect("runtime preamble should start");

	ctx.mark_lost();
	tokio::time::timeout(Duration::from_secs(10), start)
		.await
		.expect("a lost startup must finish within the lost bound")
		.expect("start task joins")
		.expect_err("a generation lost during startup must not start");
	assert!(
		ctx.is_generation_finished(),
		"startup must not be released before the generation's task finished"
	);
	assert!(
		dispatcher
			.starting_instances
			.get_async(&instance_key(actor_id, 1))
			.await
			.is_none()
	);
}

/// Start callbacks run in independent tasks, so an older generation's start can arrive after a
/// newer generation registered. It must refuse to run instead of overlapping the newer one.
#[tokio::test(start_paused = true)]
async fn superseded_generation_refuses_to_start() {
	let dispatcher = counter_dispatcher();
	let actor_id = "actor-superseded";
	let new_ctx = start_counter(&dispatcher, actor_id, 2).await;

	let old_ctx =
		crate::actor::context::ActorContext::new(actor_id, "counter", Vec::new(), "local");
	let error = dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 1,
			actor_name: "counter".to_owned(),
			input: None,
			ctx: old_ctx.clone(),
		})
		.await
		.expect_err("a superseded generation must not start");
	assert!(
		format!("{error:#}").contains("newer generation of this actor is already running"),
		"unexpected error: {error:#}"
	);
	assert!(old_ctx.is_generation_finished());
	assert!(!new_ctx.is_lost(), "the newer generation must be untouched");
	assert_eq!(instance_generation(&dispatcher, actor_id, 1).await, None);
	assert_eq!(dispatcher.current_generation(actor_id).await, Some(2));
}

/// A lost generation's storage cleanup can outlive its task. Its record must stay visible until
/// that cleanup releases the generation, so a newer generation keeps waiting on it.
#[tokio::test(start_paused = true)]
async fn stopped_generation_stays_registered_until_its_storage_is_released() {
	use rivet_envoy_client::config::ActorStopHandle;
	use rivet_envoy_client::protocol::StopActorReason;

	let dispatcher = counter_dispatcher();
	let actor_id = "actor-storage-fence";
	let ctx = start_counter(&dispatcher, actor_id, 1).await;
	// Stands in for a storage cleanup that is still closing SQLite.
	let storage_hold = ctx.hold_generation();

	dispatcher
		.stop_actor(
			actor_id,
			1,
			StopActorReason::Lost,
			ActorStopHandle::detached(),
		)
		.await
		.expect("stopping the generation should succeed");
	assert_eq!(
		instance_generation(&dispatcher, actor_id, 1).await,
		Some(1),
		"the record must stay while storage is still held",
	);

	drop(storage_hold);
	ctx.wait_for_generation_finished().await;
	for _ in 0..10 {
		tokio::task::yield_now().await;
	}
	assert_eq!(instance_generation(&dispatcher, actor_id, 1).await, None);
}
