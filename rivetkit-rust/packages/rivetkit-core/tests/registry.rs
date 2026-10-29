use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::messages::{ActorEvent, Response};

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

/// Builds a dispatcher whose actor answers every raw request with a buffered
/// response. For each request it parks a task on the request cancellation
/// token, the way foreign runtimes wait to deliver `abortSignal`, and hands
/// that task's handle to the test.
fn request_waiter_dispatcher(
	waiters: tokio::sync::mpsc::UnboundedSender<tokio::task::JoinHandle<()>>,
	hang_until_cancelled: bool,
) -> Arc<RegistryDispatcher> {
	let mut factories = HashMap::new();
	factories.insert(
		"counter".to_owned(),
		Arc::new(ActorFactory::new(ActorConfig::default(), move |start| {
			let waiters = waiters.clone();
			Box::pin(async move {
				let mut start = start;
				if let Some(ready) = start.startup_ready.take() {
					let _ = ready.send(Ok(()));
				}
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					let ActorEvent::HttpRequest { request, reply, .. } = event else {
						continue;
					};
					let token = request.cancellation_token();
					let waiter = tokio::spawn({
						let token = token.clone();
						async move { token.cancelled().await }
					});
					let _ = waiters.send(waiter);
					if hang_until_cancelled {
						tokio::spawn(async move {
							token.cancelled().await;
							reply.send(Ok(Response::from(
								::http::Response::builder()
									.status(::http::StatusCode::SERVICE_UNAVAILABLE)
									.body(Vec::new())
									.expect("build response"),
							)
							.into()));
						});
					} else {
						reply.send(Ok(Response::from(
							::http::Response::builder()
								.status(::http::StatusCode::OK)
								.body(b"pong".to_vec())
								.expect("build response"),
						)
						.into()));
					}
				}
				Ok(())
			})
		})),
	);
	Arc::new(RegistryDispatcher::new(factories, false))
}

fn raw_ping_request() -> rivet_envoy_client::config::HttpRequest {
	rivet_envoy_client::config::HttpRequest {
		method: "GET".to_owned(),
		path: "/request/ping".to_owned(),
		headers: HashMap::new(),
		body: None,
		body_stream: None,
	}
}

/// Regression test for a per-request task leak. A buffered response used to
/// drop the request token without cancelling it, so every task waiting on it
/// (one per `onRequest` in the TypeScript runtime) stayed alive until the
/// actor stopped and the live task count grew by one per request.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn buffered_raw_requests_release_request_cancellation_waiters() {
	const REQUESTS: usize = 500;

	let (waiters_tx, mut waiters_rx) = tokio::sync::mpsc::unbounded_channel();
	let dispatcher = request_waiter_dispatcher(waiters_tx, false);
	let actor_id = "actor-request-leak";
	dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 1,
			actor_name: "counter".to_owned(),
			input: None,
			ctx: ActorContext::new(actor_id, "counter", Vec::new(), "local"),
		})
		.await
		.expect("actor should start");

	let metrics = tokio::runtime::Handle::current().metrics();
	let baseline_tasks = metrics.num_alive_tasks();

	for _ in 0..REQUESTS {
		let response = dispatcher
			.handle_fetch(actor_id, raw_ping_request())
			.await
			.expect("raw request should succeed");
		assert_eq!(response.status, ::http::StatusCode::OK.as_u16());
		assert_eq!(response.body.as_deref(), Some(&b"pong"[..]));
	}

	for _ in 0..REQUESTS {
		let waiter = waiters_rx.recv().await.expect("one waiter per request");
		tokio::time::timeout(Duration::from_secs(5), waiter)
			.await
			.expect("request token must be cancelled once the response is returned")
			.expect("waiter task should not panic");
	}

	let live_tasks = metrics.num_alive_tasks();
	assert!(
		live_tasks <= baseline_tasks + 2,
		"live tasks should return to baseline after {REQUESTS} requests: baseline={baseline_tasks} live={live_tasks}",
	);
}

/// Request tokens are children of the actor abort signal, so stopping the
/// actor cancels requests that are still in flight.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actor_abort_signal_cancels_in_flight_raw_requests() {
	let (waiters_tx, mut waiters_rx) = tokio::sync::mpsc::unbounded_channel();
	let dispatcher = request_waiter_dispatcher(waiters_tx, true);
	let actor_id = "actor-request-abort";
	let ctx = ActorContext::new(actor_id, "counter", Vec::new(), "local");
	dispatcher
		.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation: 1,
			actor_name: "counter".to_owned(),
			input: None,
			ctx: ctx.clone(),
		})
		.await
		.expect("actor should start");

	let fetch = tokio::spawn({
		let dispatcher = dispatcher.clone();
		async move { dispatcher.handle_fetch(actor_id, raw_ping_request()).await }
	});
	let waiter = waiters_rx
		.recv()
		.await
		.expect("request should reach the actor");
	assert!(
		!waiter.is_finished(),
		"request must stay in flight until aborted"
	);

	ctx.cancel_actor_abort_signal();

	tokio::time::timeout(Duration::from_secs(5), waiter)
		.await
		.expect("actor abort must cancel the in-flight request token")
		.expect("waiter task should not panic");
	let response = tokio::time::timeout(Duration::from_secs(5), fetch)
		.await
		.expect("aborted request should complete")
		.expect("fetch task should not panic")
		.expect("fetch should return a response");
	assert_eq!(
		response.status,
		::http::StatusCode::SERVICE_UNAVAILABLE.as_u16()
	);
}
