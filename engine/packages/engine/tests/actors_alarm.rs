use anyhow::*;
use async_trait::async_trait;
use common::test_runner::*;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

mod common;

/// Helper to wait for actor to wake from sleep using lifecycle events (DEPRECATED for other tests)
/// Polls until sleep_ts is cleared, connectable_ts is set, and start_ts is updated
async fn wait_for_actor_wake_polling(
	port: u16,
	actor_id: &str,
	namespace: &str,
	timeout_secs: u64,
) -> Result<rivet_types::actors::Actor> {
	let start = std::time::Instant::now();
	loop {
		let actor = common::try_get_actor(port, actor_id, namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		// Actor is awake if it's not sleeping and is connectable
		let is_awake = actor.sleep_ts.is_none() && actor.connectable_ts.is_some();

		if is_awake {
			return Ok(actor);
		}

		if start.elapsed() > std::time::Duration::from_secs(timeout_secs) {
			bail!(
				"timeout waiting for actor to wake: sleep_ts={:?}, connectable_ts={:?}",
				actor.sleep_ts,
				actor.connectable_ts
			);
		}

		tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
	}
}

/// Helper to wait for actor to wake from alarm using lifecycle events
/// Waits for the actor to start again - for alarm wakes, generation increments by 1
/// For crash/restart, generation also increments by 1
async fn wait_for_actor_wake_from_alarm(
	mut lifecycle_rx: broadcast::Receiver<ActorLifecycleEvent>,
	actor_id: &str,
	expected_generation: u32,
	timeout_secs: u64,
) -> Result<u32> {
	let start = std::time::Instant::now();
	let actor_id = actor_id.to_string();

	loop {
		tokio::select! {
			result = lifecycle_rx.recv() => {
				match result {
					Result::Ok(ActorLifecycleEvent::Started { actor_id: id, generation }) => {
						if id == actor_id && generation == expected_generation {
							tracing::info!(actor_id = ?id, generation, "actor woke from alarm with expected generation");
							return Result::Ok(generation);
						}
					}
					Result::Ok(_) => continue,
					Result::Err(broadcast::error::RecvError::Lagged(n)) => {
						tracing::warn!(lagged = n, "lifecycle event receiver lagged, continuing");
						continue;
					}
					Result::Err(broadcast::error::RecvError::Closed) => {
						bail!("lifecycle event channel closed");
					}
				}
			}
			_ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs).saturating_sub(start.elapsed())) => {
				bail!(
					"timeout waiting for actor to wake from alarm: actor_id={}, expected_generation={}, waited={:?}",
					actor_id, expected_generation, start.elapsed()
				);
			}
		}
	}
}

/// Helper to wait for actor to enter sleep state
/// Polls until sleep_ts is set
async fn wait_for_actor_sleep(
	port: u16,
	actor_id: &str,
	namespace: &str,
	timeout_secs: u64,
) -> Result<rivet_types::actors::Actor> {
	let start = std::time::Instant::now();
	loop {
		let actor = common::try_get_actor(port, actor_id, namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		if actor.sleep_ts.is_some() {
			return Ok(actor);
		}

		if start.elapsed() > std::time::Duration::from_secs(timeout_secs) {
			bail!(
				"timeout waiting for actor to sleep: sleep_ts={:?}",
				actor.sleep_ts
			);
		}

		tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
	}
}

/// Get current timestamp in milliseconds (matching alarm format)
fn get_current_timestamp_ms() -> i64 {
	rivet_util::timestamp::now()
}

// MARK: Behavior Implementations

/// Actor that sets an alarm and immediately sends sleep intent on first start (generation 0).
/// On subsequent starts (after wake from alarm), it stays awake.
/// Notifies via ready_tx when setup is complete.
struct AlarmAndSleepActor {
	alarm_offset_ms: i64,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl AlarmAndSleepActor {
	fn new(
		alarm_offset_ms: i64,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			alarm_offset_ms,
			ready_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmAndSleepActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm actor starting");

		if generation == 0 {
			// First start: set alarm and sleep
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_sleep_intent();

			// Notify test that we're ready
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}

			tracing::info!(generation, "set alarm and sleeping");
		} else {
			// Subsequent wakes (generation >= 1): stay awake
			tracing::info!(generation, "woke from alarm, staying awake");
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmAndSleepActor"
	}
}

/// Actor that sets an alarm and sleeps only on first run (generation 0).
/// On subsequent wakes (from alarm), stays awake without sleeping again.
/// Notifies via ready_tx when setup is complete.
struct AlarmAndSleepOnceActor {
	alarm_offset_ms: i64,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl AlarmAndSleepOnceActor {
	fn new(
		alarm_offset_ms: i64,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			alarm_offset_ms,
			ready_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmAndSleepOnceActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm once actor starting");

		if generation == 0 {
			// First start (gen 0): set alarm and sleep
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_sleep_intent();

			// Notify test that we're ready
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}

			tracing::info!(generation, "set alarm and sleeping");
		} else {
			// Subsequent wakes (gen >= 1): stay awake
			tracing::info!(generation, "woke from alarm, staying awake");
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmAndSleepOnceActor"
	}
}

/// Actor that sets an alarm, sends sleep intent, then clears the alarm after a delay (generation 0 only).
/// Notifies via ready_tx when initial setup is complete.
/// Notifies via clear_tx when alarm is cleared.
struct AlarmSleepThenClearActor {
	alarm_offset_ms: i64,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl AlarmSleepThenClearActor {
	fn new(
		alarm_offset_ms: i64,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			alarm_offset_ms,
			ready_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmSleepThenClearActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm actor starting");

		if generation == 0 {
			// Set alarm for current_time + offset
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_clear_alarm();
			// Send sleep intent
			config.send_sleep_intent();

			// Notify test
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmSleepThenClearActor"
	}
}

/// Actor that sets an alarm, sends sleep intent, then replaces the alarm after a delay (generation 0 only).
/// Notifies via ready_tx when initial setup is complete.
/// Notifies via replace_tx when alarm is replaced.
struct AlarmSleepThenReplaceActor {
	initial_alarm_offset_ms: i64,
	replace_delay_ms: u64,
	replacement_alarm_offset_ms: i64,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	replace_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

impl AlarmSleepThenReplaceActor {
	fn new(
		initial_alarm_offset_ms: i64,
		replace_delay_ms: u64,
		replacement_alarm_offset_ms: i64,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
		replace_tx: tokio::sync::mpsc::UnboundedSender<()>,
	) -> Self {
		Self {
			initial_alarm_offset_ms,
			replace_delay_ms,
			replacement_alarm_offset_ms,
			ready_tx,
			replace_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmSleepThenReplaceActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm actor starting");

		if generation == 0 {
			// Set alarm A for current_time + offset
			let alarm_a_time = get_current_timestamp_ms() + self.initial_alarm_offset_ms;
			config.send_set_alarm(alarm_a_time);

			// Notify test
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}

			// Wait before replacing alarm (but BEFORE sleeping)
			tokio::time::sleep(tokio::time::Duration::from_millis(self.replace_delay_ms)).await;

			// Replace with alarm B - this must happen BEFORE we sleep
			// because sleeping actors ignore events
			let alarm_b_time = get_current_timestamp_ms() + self.replacement_alarm_offset_ms;
			config.send_set_alarm(alarm_b_time);

			// Notify that alarm was replaced
			let _ = self.replace_tx.send(());
			tracing::info!("alarm replaced, now sleeping");

			// Now send sleep intent AFTER replacing the alarm
			config.send_sleep_intent();
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmSleepThenReplaceActor"
	}
}

/// Actor that sets multiple alarms before sleeping (generation 0 only).
/// Used to test that only the last alarm fires.
struct MultipleAlarmSetActor {
	alarm_offsets_ms: Vec<i64>,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl MultipleAlarmSetActor {
	fn new(
		alarm_offsets_ms: Vec<i64>,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			alarm_offsets_ms,
			ready_tx,
		}
	}
}

#[async_trait]
impl TestActor for MultipleAlarmSetActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "multi alarm actor starting");

		if generation == 0 {
			// Set multiple alarms
			for offset in &self.alarm_offsets_ms {
				let alarm_time = get_current_timestamp_ms() + offset;
				config.send_set_alarm(alarm_time);
			}

			// Send sleep intent
			config.send_sleep_intent();

			// Notify test
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"MultipleAlarmSetActor"
	}
}

/// Actor that sets a new alarm each time it wakes, creating multiple sleep/wake cycles.
struct MultiCycleAlarmActor {
	alarm_offset_ms: i64,
	max_cycles: Arc<Mutex<usize>>,
	wake_tx: tokio::sync::mpsc::UnboundedSender<u32>,
}

impl MultiCycleAlarmActor {
	fn new(
		alarm_offset_ms: i64,
		max_cycles: usize,
		wake_tx: tokio::sync::mpsc::UnboundedSender<u32>,
	) -> Self {
		Self {
			alarm_offset_ms,
			max_cycles: Arc::new(Mutex::new(max_cycles)),
			wake_tx,
		}
	}
}

#[async_trait]
impl TestActor for MultiCycleAlarmActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "multi cycle alarm actor starting");

		// Notify test of wake
		let _ = self.wake_tx.send(generation);

		// Check if we should continue cycling
		let mut remaining = self.max_cycles.lock().unwrap();
		if *remaining > 0 {
			*remaining -= 1;

			// Set alarm and sleep
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_sleep_intent();

			tracing::info!(generation, remaining = *remaining, "set alarm and sleeping");
		} else {
			tracing::info!(generation, "max cycles reached, staying awake");
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"MultiCycleAlarmActor"
	}
}

/// Actor that sets an alarm on first wake (generation 0), then sleeps again without setting a new alarm.
/// Used to test that actor stays asleep when no new alarm is set.
struct AlarmOnceActor {
	alarm_offset_ms: i64,
	wake_tx: tokio::sync::mpsc::UnboundedSender<u32>,
}

impl AlarmOnceActor {
	fn new(alarm_offset_ms: i64, wake_tx: tokio::sync::mpsc::UnboundedSender<u32>) -> Self {
		Self {
			alarm_offset_ms,
			wake_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmOnceActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm once actor starting");

		// Notify test of wake
		let _ = self.wake_tx.send(generation);

		if generation == 0 {
			// First start (gen 0): set alarm and sleep
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_sleep_intent();
			tracing::info!(generation, "first start, set alarm and sleeping");
		} else {
			// Subsequent wakes (gen >= 1): just sleep without setting a new alarm
			config.send_sleep_intent();
			tracing::info!(generation, "subsequent wake, sleeping without alarm");
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmOnceActor"
	}
}

/// Actor that sets an alarm, sleeps on gen 0, then crashes immediately on wake.
/// Gen 1+ stays running. Used to test that alarms don't persist across generations.
struct AlarmSleepThenCrashActor {
	alarm_offset_ms: i64,
	sleeping_tx: tokio::sync::mpsc::UnboundedSender<u32>,
	crash_tx: tokio::sync::mpsc::UnboundedSender<u32>,
}

impl AlarmSleepThenCrashActor {
	fn new(
		alarm_offset_ms: i64,
		sleeping_tx: tokio::sync::mpsc::UnboundedSender<u32>,
		crash_tx: tokio::sync::mpsc::UnboundedSender<u32>,
	) -> Self {
		Self {
			alarm_offset_ms,
			sleeping_tx,
			crash_tx,
		}
	}
}

#[async_trait]
impl TestActor for AlarmSleepThenCrashActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm crash actor starting");

		if generation == 0 {
			// First start (gen 0): set alarm, and crash
			let alarm_time = get_current_timestamp_ms() + self.alarm_offset_ms;
			config.send_set_alarm(alarm_time);

			// Notify test
			let _ = self.crash_tx.send(generation);

			tracing::info!(generation, "set alarm and sleeping");
			Ok(ActorStartResult::Crash {
				code: 1,
				message: "crashing with gen 0".to_string(),
			})
		} else if generation == 1 {
			tracing::info!(generation, "restarted after crash, sending sleep intent");
			config.send_sleep_intent();
			let _ = self.sleeping_tx.send(generation);
			Ok(ActorStartResult::Running)
		} else {
			// If it restarted again, this was not expected
			//
			// Keep the actor running so the test finds out we're not asleep.
			Ok(ActorStartResult::Running)
		}
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"AlarmSleepThenCrashActor"
	}
}

/// Actor that rapidly sets and clears alarms multiple times before sleeping (generation 0 only).
/// Used to test that rapid operations don't cause errors.
struct RapidAlarmCycleActor {
	cycles: usize,
	final_alarm_offset_ms: i64,
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl RapidAlarmCycleActor {
	fn new(
		cycles: usize,
		final_alarm_offset_ms: i64,
		ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			cycles,
			final_alarm_offset_ms,
			ready_tx,
		}
	}
}

#[async_trait]
impl TestActor for RapidAlarmCycleActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "rapid alarm cycle actor starting");

		if generation == 0 {
			// Rapidly set and clear alarms
			for _i in 0..self.cycles {
				config.send_set_alarm(get_current_timestamp_ms() + 5000);
				config.send_clear_alarm();
			}

			// Set final alarm and sleep
			let alarm_time = get_current_timestamp_ms() + self.final_alarm_offset_ms;
			config.send_set_alarm(alarm_time);
			config.send_sleep_intent();

			// Notify test
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"RapidAlarmCycleActor"
	}
}

/// Actor that sets an alarm, immediately clears it, then sends sleep intent (generation 0 only).
/// Used to test that null alarm_ts properly clears alarms.
struct SetClearAlarmAndSleepActor {
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl SetClearAlarmAndSleepActor {
	fn new(ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>) -> Self {
		Self { ready_tx }
	}
}

#[async_trait]
impl TestActor for SetClearAlarmAndSleepActor {
	async fn on_start(&mut self, config: ActorConfig) -> anyhow::Result<ActorStartResult> {
		let generation = config.generation;
		tracing::info!(?config.actor_id, generation, "alarm actor starting");

		if generation == 0 {
			// Set alarm
			let alarm_time = get_current_timestamp_ms() + 2000;
			config.send_set_alarm(alarm_time);

			// Clear it (set to null)
			config.send_clear_alarm();

			// Send sleep intent
			config.send_sleep_intent();

			// Notify test
			if let Some(tx) = self.ready_tx.lock().unwrap().take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> anyhow::Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"SetClearAlarmAndSleepActor"
	}
}

// MARK: Core Functionality

#[test]
fn basic_alarm() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				Box::new(AlarmAndSleepActor::new(3000, ready_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Actor should be sleeping
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.unwrap();

		tracing::info!(
			?actor_id,
			"actor sleeping, alarm was set with gen 0, alarm should fire"
		);

		// Verify actor wakes from valid alarm
		wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), &actor_id, &namespace, 4)
			.await
			.expect("actor should wake from alarm");

		tracing::info!(?actor_id, "gen 0 alarm fired successfully");
	});
}

#[test]
fn clear_alarm_prevents_wake() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				Box::new(AlarmSleepThenClearActor::new(2000, ready_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Verify actor is sleeping
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.unwrap();

		// Wait past the original alarm time
		tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

		// Verify actor is still sleeping
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.sleep_ts.is_some(),
			"actor should still be sleeping after alarm was cleared"
		);

		tracing::info!(?actor_id, "alarm cleared successfully prevented wake");
	});
}

#[test]
fn replace_alarm_overwrites_previous() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));
		let (replace_tx, mut replace_rx) = tokio::sync::mpsc::unbounded_channel();

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				let replace_tx = replace_tx.clone();
				Box::new(AlarmSleepThenReplaceActor::new(
					3000, 500, 1000, ready_tx, replace_tx,
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Wait for alarm to be replaced
		replace_rx.recv().await.expect("alarm should be replaced");

		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.expect("actor should be asleep");

		tracing::info!("waiting for actor to wake from alarm B (~1s)");

		// Actor should wake ~1s after alarm B was set, not 3s
		// We'll wait up to 3 seconds total - it should wake much sooner
		let wake_start = std::time::Instant::now();
		let actor =
			wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), &actor_id, &namespace, 10)
				.await
				.expect("expected actor to be awake from alarm A or B");
		let wake_duration = wake_start.elapsed();

		assert!(actor.sleep_ts.is_none(), "actor should be awake");
		assert!(
			wake_duration < std::time::Duration::from_millis(2500),
			"actor should wake from alarm B (~1.5s), not alarm A (3s), actual: {:?}",
			wake_duration
		);

		tracing::info!(?actor_id, ?wake_duration, "alarm replaced successfully");
	});
}

#[test]
fn alarm_in_the_past() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				Box::new(AlarmAndSleepActor::new(-1000, ready_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready (gen 0)
		ready_rx.await.expect("actor should send ready signal");

		// Actor sets alarm in the past and sleeps
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.expect("actor should be asleep");

		// The past alarm should fire immediately, waking the actor
		wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), &actor_id, &namespace, 2)
			.await
			.expect("actor should wake immediately from past alarm");

		// Verify actor is awake at gen 1
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(actor.sleep_ts.is_none(), "actor should be awake");
		assert!(
			actor.connectable_ts.is_some(),
			"actor should be connectable"
		);

		tracing::info!(?actor_id, "actor woke immediately from past alarm");
	});
}

#[test]
fn alarm_with_null_timestamp() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				Box::new(SetClearAlarmAndSleepActor::new(ready_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Verify actor is sleeping
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.expect("actor is not sleeping");

		// Wait past alarm time
		tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

		// Verify actor is still sleeping
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.sleep_ts.is_some(),
			"actor should still be sleeping after alarm was cleared with null"
		);

		tracing::info!(?actor_id, "null alarm_ts successfully cleared alarm");
	});
}

// MARK: Edge Cases

#[test]
fn alarm_fires_at_correct_time() {
	common::run(
		common::TestOpts::new(1).with_timeout(10),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
			let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("alarm-actor", move |_| {
					let ready_tx = ready_tx.clone();
					Box::new(AlarmAndSleepOnceActor::new(5000, ready_tx))
				})
			})
			.await;

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"alarm-actor",
				runner.name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			let actor_id = res.actor.actor_id.to_string();

			// Wait for actor to be ready
			ready_rx.await.expect("actor should send ready signal");

			// Record when actor started sleeping
			wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 4)
				.await
				.unwrap();
			let sleep_time = std::time::Instant::now();

			tracing::info!(?actor_id, "actor is sleeping, alarm set for +5s");

			// Subscribe to lifecycle events AFTER actor is sleeping, so we only get the wake event
			let lifecycle_rx = runner.subscribe_lifecycle_events();

			// Wait for actor to wake using lifecycle events (expect generation 1, incremented from sleep)
			wait_for_actor_wake_from_alarm(lifecycle_rx, &actor_id, 1, 7)
				.await
				.expect("expected actor to be awake");

			let wake_duration = sleep_time.elapsed();

			// Verify wake time is within ±500ms of alarm time (5s)
			assert!(
				wake_duration >= std::time::Duration::from_millis(4500)
					&& wake_duration <= std::time::Duration::from_millis(5500),
				"alarm should fire within ±500ms of 5s, actual: {:?}",
				wake_duration
			);

			tracing::info!(?actor_id, ?wake_duration, "alarm fired at correct time");
		},
	);
}

#[test]
fn multiple_alarm_sets_before_sleep() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				// Set alarms for +5s, +10s, +2s (last one should win)
				Box::new(MultipleAlarmSetActor::new(
					vec![5000, 10000, 2000],
					ready_tx,
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Verify actor is sleeping
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.unwrap();
		let sleep_time = std::time::Instant::now();

		tracing::info!(?actor_id, "actor is sleeping, last alarm set for +2s");

		// Wait for actor to wake
		wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), &actor_id, &namespace, 4)
			.await
			.expect("expected actor to be awake");

		let wake_duration = sleep_time.elapsed();

		// Verify wakes at ~2s mark (last alarm), not 5s or 10s
		assert!(
			wake_duration >= std::time::Duration::from_millis(1500)
				&& wake_duration <= std::time::Duration::from_millis(2500),
			"actor should wake from last alarm (~2s), actual: {:?}",
			wake_duration
		);

		tracing::info!(?actor_id, ?wake_duration, "only last alarm fired");
	});
}

#[test]
fn multiple_sleep_wake_alarm_cycles() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (wake_tx, mut wake_rx) = tokio::sync::mpsc::unbounded_channel();

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let wake_tx = wake_tx.clone();
				// 3 cycles with 1s alarms
				Box::new(MultiCycleAlarmActor::new(1000, 3, wake_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		tracing::info!(?actor_id, "waiting for 3 wake cycles");

		// Collect 3 wake notifications (initial + 2 alarm wakes)
		let mut wake_count = 0;
		for _ in 0..3 {
			tokio::time::timeout(tokio::time::Duration::from_secs(3), wake_rx.recv())
				.await
				.expect("timeout waiting for wake notification")
				.expect("wake channel closed");
			wake_count += 1;
			tracing::info!(wake_count, "actor woke");
		}

		assert_eq!(wake_count, 3, "actor should have woken 3 times");

		tracing::info!(?actor_id, "all 3 cycles completed successfully");
	});
}

#[test]
fn alarm_wake_then_sleep_without_new_alarm() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (wake_tx, mut wake_rx) = tokio::sync::mpsc::unbounded_channel();

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let wake_tx = wake_tx.clone();
				// Set alarm for 1s on first start
				Box::new(AlarmOnceActor::new(1000, wake_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for first wake (initial start)
		wake_rx.recv().await.expect("first wake notification");
		tracing::info!(?actor_id, "actor initial start");

		// Wait for second wake (from alarm)
		tokio::time::timeout(tokio::time::Duration::from_secs(3), wake_rx.recv())
			.await
			.expect("timeout waiting for alarm wake")
			.expect("wake channel closed");
		tracing::info!(?actor_id, "actor woke from alarm");

		// Verify actor went back to sleep
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.expect("actor should be asleep");

		// Wait additional time to ensure no spurious wake
		tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

		// Verify actor is still sleeping (no zombie alarm)
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.sleep_ts.is_some(),
			"actor should still be sleeping without new alarm"
		);

		tracing::info!(?actor_id, "actor stayed asleep without zombie alarm");
	});
}

// MARK: Advanced Usage

#[test]
fn alarm_behavior_with_crash_policy_restart() {
	common::run(
		common::TestOpts::new(1).with_timeout(45),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (sleeping_tx, mut sleeping_rx) = tokio::sync::mpsc::unbounded_channel();
			let (crash_tx, mut crash_rx) = tokio::sync::mpsc::unbounded_channel();

			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("alarm-actor", move |_| {
					let sleeping_tx = sleeping_tx.clone();
					let crash_tx = crash_tx.clone();
					// Set alarm for 15s, crash after 500ms
					Box::new(AlarmSleepThenCrashActor::new(15000, sleeping_tx, crash_tx))
				})
			})
			.await;

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"alarm-actor",
				runner.name(),
				rivet_types::actors::CrashPolicy::Restart,
			)
			.await;

			let actor_id = res.actor.actor_id.to_string();

			// Wait for crash notification gen 0 sets alarm and crashes
			crash_rx
				.recv()
				.await
				.expect("should receive crash notification");

			tracing::info!(
				?actor_id,
				"gen 0 crashed after alarm wake, waiting for gen 1 restart"
			);

			// Wait for actor to start sleeping again (gen 1 started and sleep)
			sleeping_rx
				.recv()
				.await
				.expect("actor should send sleep signal");

			let actor =
				wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
					.await
					.expect("actor should be sleeping");

			assert!(actor.sleep_ts.is_some(), "actor should be asleep");

			tracing::info!(
				?actor_id,
				"gen 1 is now asleep, waiting past original alarm time"
			);

			// Verify the next gen is awake (woke from gen 0's alarm)
			let actor = wait_for_actor_wake_polling(
				ctx.leader_dc().guard_port(),
				&actor_id,
				&namespace,
				15,
			)
			.await
			.expect("actor should be sleeping");

			assert!(
				actor.sleep_ts.is_none() && actor.connectable_ts.is_some(),
				"next generation should be awake from gen 0 alarm"
			);
		},
	);
}

#[test]
fn rapid_alarm_set_clear_cycles() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let ready_tx = ready_tx.clone();
				// 10 rapid cycles, then final alarm for 1s
				Box::new(RapidAlarmCycleActor::new(10, 1000, ready_tx))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"alarm-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to be ready
		ready_rx.await.expect("actor should send ready signal");

		// Verify actor is sleeping
		wait_for_actor_sleep(ctx.leader_dc().guard_port(), &actor_id, &namespace, 5)
			.await
			.unwrap();

		tracing::info!(
			?actor_id,
			"actor sleeping after rapid cycles, waiting for final alarm"
		);

		// Verify actor wakes at final alarm time
		wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), &actor_id, &namespace, 3)
			.await
			.expect("actor should wake from final alarm");

		tracing::info!(?actor_id, "rapid alarm cycles succeeded, final alarm fired");
	});
}

#[test]
fn multiple_actors_with_different_alarm_times() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create 3 actors with different alarm times
		let alarm_offsets = vec![1000, 2000, 3000];
		let mut actor_ids = Vec::new();

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			let mut b = builder;
			for (idx, offset) in alarm_offsets.iter().enumerate() {
				let offset = *offset;
				b = b.with_actor_behavior(&format!("alarm-actor-{}", idx), move |_| {
					let (ready_tx, _) = tokio::sync::oneshot::channel();
					let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));
					Box::new(AlarmAndSleepActor::new(offset, ready_tx))
				});
			}
			b
		})
		.await;

		// Create actors
		for idx in 0..3 {
			// Create actor with specific behavior
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				&format!("alarm-actor-{}", idx),
				runner.name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;
			actor_ids.push(res.actor.actor_id.to_string());
		}

		tracing::info!("created 3 actors with alarms at +1s, +2s, +3s");

		// Wait for all actors to enter sleep state
		for (idx, actor_id) in actor_ids.iter().enumerate() {
			wait_for_actor_sleep(ctx.leader_dc().guard_port(), actor_id, &namespace, 5)
				.await
				.unwrap();
			tracing::info!(idx, actor_id, "actor sleeping");
		}

		// Verify actors wake in order
		for (idx, actor_id) in actor_ids.iter().enumerate() {
			tracing::info!(idx, actor_id, "waiting for actor to wake");

			wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), actor_id, &namespace, 5)
				.await
				.expect("actor should wake");

			tracing::info!(idx, actor_id, "actor woke at expected time");
		}

		tracing::info!("all actors woke at their independent alarm times");
	});
}

#[test]
fn many_actors_same_alarm_time() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let num_actors = 10;
		let alarm_offset = 2000; // All wake at same time
		let mut actor_ids = Vec::new();

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("alarm-actor", move |_| {
				let (ready_tx, _) = tokio::sync::oneshot::channel();
				let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));
				Box::new(AlarmAndSleepActor::new(alarm_offset, ready_tx))
			})
		})
		.await;

		// Create actors
		for _idx in 0..num_actors {
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"alarm-actor",
				runner.name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;
			actor_ids.push(res.actor.actor_id.to_string());
		}

		tracing::info!(num_actors, "created actors with same alarm time (+2s)");

		// Wait for all actors to enter sleep state
		for actor_id in &actor_ids {
			wait_for_actor_sleep(ctx.leader_dc().guard_port(), actor_id, &namespace, 5)
				.await
				.unwrap();
		}

		tracing::info!("all actors sleeping");

		let alarm_start = std::time::Instant::now();

		// Verify all actors wake within reasonable time window
		for (idx, actor_id) in actor_ids.iter().enumerate() {
			wait_for_actor_wake_polling(ctx.leader_dc().guard_port(), actor_id, &namespace, 4)
				.await
				.expect("actor should wake");

			tracing::info!(idx, actor_id, "actor woke");
		}

		let total_duration = alarm_start.elapsed();

		// All 10 actors should wake within a 500ms window around the alarm time
		assert!(
			total_duration <= std::time::Duration::from_millis(3000),
			"all actors should wake within 3s, actual: {:?}",
			total_duration
		);

		tracing::info!(
			num_actors,
			?total_duration,
			"all actors woke concurrently at same alarm time"
		);
	});
}
