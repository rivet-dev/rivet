use crate::actor::*;
use anyhow::Result;
use async_trait::async_trait;
use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

/// Simple echo actor that responds successfully and does nothing special
pub struct EchoActor;

impl EchoActor {
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for EchoActor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for EchoActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "echo actor started");
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		tracing::info!("echo actor stopped");
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"EchoActor"
	}
}

/// Actor that crashes immediately on start with specified exit code
pub struct CrashOnStartActor {
	pub exit_code: i32,
	pub message: String,
	notify_tx: Option<std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>>,
}

impl CrashOnStartActor {
	pub fn new(exit_code: i32) -> Self {
		Self {
			exit_code,
			message: format!("crash on start with code {}", exit_code),
			notify_tx: None,
		}
	}

	pub fn new_with_notify(
		exit_code: i32,
		notify_tx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			exit_code,
			message: format!("crash on start with code {}", exit_code),
			notify_tx: Some(notify_tx),
		}
	}
}

#[async_trait]
impl TestActor for CrashOnStartActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::warn!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			exit_code = self.exit_code,
			"crash on start actor crashing"
		);

		// Notify before crashing
		if let Some(notify_tx) = &self.notify_tx {
			let mut guard = notify_tx.lock().expect("failed to lock notify_tx");
			if let Some(tx) = guard.take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Crash {
			code: self.exit_code,
			message: self.message.clone(),
		})
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"CrashOnStartActor"
	}
}

/// Actor that delays before sending running state
pub struct DelayedStartActor {
	pub delay: Duration,
}

impl DelayedStartActor {
	pub fn new(delay: Duration) -> Self {
		Self { delay }
	}
}

#[async_trait]
impl TestActor for DelayedStartActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			delay_ms = self.delay.as_millis(),
			"delayed start actor will delay before running"
		);
		Ok(ActorStartResult::Delay(self.delay))
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"DelayedStartActor"
	}
}

/// Actor that never sends running state (simulates timeout)
pub struct TimeoutActor;

impl TimeoutActor {
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for TimeoutActor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for TimeoutActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::warn!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			"timeout actor will never send running state"
		);
		Ok(ActorStartResult::Timeout)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"TimeoutActor"
	}
}

/// Actor that sends sleep intent immediately after starting
pub struct SleepImmediatelyActor {
	notify_tx: Option<std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>>,
}

impl SleepImmediatelyActor {
	pub fn new() -> Self {
		Self { notify_tx: None }
	}

	pub fn new_with_notify(
		notify_tx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			notify_tx: Some(notify_tx),
		}
	}
}

impl Default for SleepImmediatelyActor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for SleepImmediatelyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			"sleep immediately actor started, sending sleep intent"
		);

		// Send sleep intent immediately
		config.send_sleep_intent();

		// Notify that we're sending sleep intent
		if let Some(notify_tx) = &self.notify_tx {
			let mut guard = notify_tx.lock().expect("failed to lock notify_tx");
			if let Some(tx) = guard.take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		tracing::info!("sleep immediately actor stopped");
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"SleepImmediatelyActor"
	}
}

/// Actor that sends stop intent immediately after starting
pub struct StopImmediatelyActor;

impl StopImmediatelyActor {
	pub fn new() -> Self {
		Self
	}
}

impl Default for StopImmediatelyActor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for StopImmediatelyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			"stop immediately actor started, sending stop intent"
		);

		// Send stop intent immediately
		config.send_stop_intent();

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		tracing::info!("stop immediately actor stopped gracefully");
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"StopImmediatelyActor"
	}
}

/// Actor that always crashes and increments a counter.
/// Used to test crash policy restart behavior.
pub struct CountingCrashActor {
	crash_count: Arc<std::sync::atomic::AtomicU32>,
}

impl CountingCrashActor {
	pub fn new(crash_count: Arc<std::sync::atomic::AtomicU32>) -> Self {
		Self { crash_count }
	}
}

#[async_trait]
impl TestActor for CountingCrashActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		let count = self
			.crash_count
			.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		tracing::warn!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			crash_count = count + 1,
			"counting crash actor crashing"
		);
		Ok(ActorStartResult::Crash {
			code: 1,
			message: format!("crash #{}", count + 1),
		})
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"CountingCrashActor"
	}
}

/// Actor that crashes N times then succeeds
/// Used to test crash policy restart with retry reset on success
pub struct CrashNTimesThenSucceedActor {
	crash_count: Arc<Mutex<usize>>,
	max_crashes: usize,
}

impl CrashNTimesThenSucceedActor {
	pub fn new(max_crashes: usize, crash_count: Arc<Mutex<usize>>) -> Self {
		Self {
			crash_count,
			max_crashes,
		}
	}
}

#[async_trait]
impl TestActor for CrashNTimesThenSucceedActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		let mut count = self.crash_count.lock().unwrap();
		let current = *count;

		if current < self.max_crashes {
			*count += 1;
			tracing::warn!(
				actor_id = ?config.actor_id,
				generation = config.generation,
				crash_count = current + 1,
				max_crashes = self.max_crashes,
				"crashing (will succeed after more crashes)"
			);
			Ok(ActorStartResult::Crash {
				code: 1,
				message: format!("crash {} of {}", current + 1, self.max_crashes),
			})
		} else {
			tracing::info!(
				actor_id = ?config.actor_id,
				generation = config.generation,
				crash_count = current,
				"succeeded after crashes"
			);
			Ok(ActorStartResult::Running)
		}
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"CrashNTimesThenSucceedActor"
	}
}

/// Actor that notifies via a oneshot channel when it starts running
/// This allows tests to wait for the actor to actually start instead of sleeping
pub struct NotifyOnStartActor {
	notify_tx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl NotifyOnStartActor {
	pub fn new(
		notify_tx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
	) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for NotifyOnStartActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			"notify on start actor started, sending notification"
		);

		// Send notification that actor has started
		let mut guard = self.notify_tx.lock().expect("failed to lock notify_tx");
		if let Some(tx) = guard.take() {
			let _ = tx.send(());
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		tracing::info!("notify on start actor stopped");
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"NotifyOnStartActor"
	}
}

/// Actor that verifies it received the expected input data
/// Crashes if input doesn't match or is missing, succeeds if it matches
pub struct VerifyInputActor {
	expected_input: Vec<u8>,
}

impl VerifyInputActor {
	pub fn new(expected_input: Vec<u8>) -> Self {
		Self { expected_input }
	}
}

#[async_trait]
impl TestActor for VerifyInputActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(
			actor_id = ?config.actor_id,
			generation = config.generation,
			expected_input_size = self.expected_input.len(),
			received_input_size = config.input.as_ref().map(|i| i.len()),
			"verify input actor started, checking input"
		);

		// Check if input is present
		let Some(received_input) = &config.input else {
			tracing::error!("no input data received");
			return Ok(ActorStartResult::Crash {
				code: 1,
				message: "no input data received".to_string(),
			});
		};

		// Check if input matches expected
		if received_input != &self.expected_input {
			tracing::error!(
				expected_len = self.expected_input.len(),
				received_len = received_input.len(),
				"input data mismatch"
			);
			return Ok(ActorStartResult::Crash {
				code: 1,
				message: format!(
					"input mismatch: expected {} bytes, got {} bytes",
					self.expected_input.len(),
					received_input.len()
				),
			});
		}

		tracing::info!("input data verified successfully");
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		tracing::info!("verify input actor stopped");
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"VerifyInputActor"
	}
}

/// Generic actor that accepts closures for on_start and on_stop
/// This allows tests to define actor behavior inline without creating separate structs
pub struct CustomActor {
	on_start_fn: Box<
		dyn Fn(
				ActorConfig,
			) -> std::pin::Pin<
				Box<dyn std::future::Future<Output = Result<ActorStartResult>> + Send>,
			> + Send
			+ Sync,
	>,
	on_stop_fn: Box<
		dyn Fn() -> std::pin::Pin<
				Box<dyn std::future::Future<Output = Result<ActorStopResult>> + Send>,
			> + Send
			+ Sync,
	>,
}

/// Builder for CustomActor with default implementations
pub struct CustomActorBuilder {
	on_start_fn: Option<
		Box<
			dyn Fn(
					ActorConfig,
				) -> std::pin::Pin<
					Box<dyn std::future::Future<Output = Result<ActorStartResult>> + Send>,
				> + Send
				+ Sync,
		>,
	>,
	on_stop_fn: Option<
		Box<
			dyn Fn() -> std::pin::Pin<
					Box<dyn std::future::Future<Output = Result<ActorStopResult>> + Send>,
				> + Send
				+ Sync,
		>,
	>,
}

impl CustomActorBuilder {
	pub fn new() -> Self {
		Self {
			on_start_fn: None,
			on_stop_fn: None,
		}
	}

	pub fn on_start<F>(mut self, f: F) -> Self
	where
		F: Fn(
				ActorConfig,
			) -> std::pin::Pin<
				Box<dyn std::future::Future<Output = Result<ActorStartResult>> + Send>,
			> + Send
			+ Sync
			+ 'static,
	{
		self.on_start_fn = Some(Box::new(f));
		self
	}

	pub fn on_stop<F>(mut self, f: F) -> Self
	where
		F: Fn() -> std::pin::Pin<
				Box<dyn std::future::Future<Output = Result<ActorStopResult>> + Send>,
			> + Send
			+ Sync
			+ 'static,
	{
		self.on_stop_fn = Some(Box::new(f));
		self
	}

	pub fn build(self) -> CustomActor {
		CustomActor {
			on_start_fn: self.on_start_fn.unwrap_or_else(|| {
				Box::new(|_config| {
					Box::pin(async { Ok(ActorStartResult::Running) })
						as std::pin::Pin<
							Box<dyn std::future::Future<Output = Result<ActorStartResult>> + Send>,
						>
				})
			}),
			on_stop_fn: self.on_stop_fn.unwrap_or_else(|| {
				Box::new(|| {
					Box::pin(async { Ok(ActorStopResult::Success) })
						as std::pin::Pin<
							Box<dyn std::future::Future<Output = Result<ActorStopResult>> + Send>,
						>
				})
			}),
		}
	}
}

impl Default for CustomActorBuilder {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for CustomActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		(self.on_start_fn)(config).await
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		(self.on_stop_fn)().await
	}

	fn name(&self) -> &str {
		"CustomActor"
	}
}
