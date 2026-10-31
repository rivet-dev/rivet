//! Owns one recovery-only client, bounded transport work, and cancellation cleanup.

use super::{Payloads, VolumeClientTransport};
use crate::volume_client_machine::{
	VolumeClientMachine, effect::VolumeClientEffect as Effect, event::VolumeClientEvent as Input,
	snapshot::VolumeClientSnapshot, state::VolumeClientState, types::*,
};
use depot_types::VolumeLogKey;
use parking_lot::Mutex;
use rivet_data::generated::shared_depot_types_v1::TabletReplicaSet;
use rivet_env::{Env, TaskScope};
use rivet_state_machine_ephemeral::{EffectContext, Executor, Runner};
use rivet_topology::worker::WorkerId;
use std::collections::BTreeSet;
use std::sync::{
	Arc, Weak,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, oneshot, watch};

/// Checks the fixed admitted grant and current lease using the adapter's fresh clock.
pub trait RecoveryAuthority: Send + Sync + 'static {
	fn may_run(&self) -> bool;
}

pub struct RecoveryDependencies {
	pub env: Arc<dyn Env>,
	pub transport: Arc<dyn VolumeClientTransport>,
	pub authority: Arc<dyn RecoveryAuthority>,
}

/// Bounds the entire attempt, including a peer that never reaches its claim entry.
#[derive(Clone, Debug)]
pub struct RecoveryConfig {
	pub client: VolumeClientConfig,
	pub timeout_ms: u64,
	/// Destinations that must match and learn the claim entry in addition to its publication
	/// quorums. Empty preserves ordinary best-effort matching of every destination.
	pub required_members: BTreeSet<WorkerId>,
}

impl Default for RecoveryConfig {
	fn default() -> Self {
		Self {
			client: VolumeClientConfig::default(),
			timeout_ms: 60_000,
			required_members: BTreeSet::new(),
		}
	}
}

/// Supervisors await shutdown before releasing authority. Drop transfers cleanup and is not proof of completion.
pub struct Recovery {
	pub snapshot: watch::Receiver<VolumeClientSnapshot>,
	inbox: Option<mpsc::Sender<Input>>,
	result: Option<oneshot::Receiver<Result<RecoveryReport, WriteError>>>,
	stopped: Option<oneshot::Receiver<()>>,
	env: Arc<dyn Env>,
	tasks: Arc<TaskScope>,
	closing: Arc<AtomicBool>,
	authority: Arc<dyn RecoveryAuthority>,
	deadline_boot_ms: u64,
}

impl Recovery {
	/// Starts the existing client core in recovery purpose; constructing dependencies starts no tasks.
	pub fn start(
		key: VolumeLogKey,
		replica_set: TabletReplicaSet,
		config: RecoveryConfig,
		deps: RecoveryDependencies,
	) -> anyhow::Result<Self> {
		anyhow::ensure!(config.timeout_ms > 0, "recovery timeout must be positive");
		anyhow::ensure!(
			deps.authority.may_run(),
			"recovery authority is not current"
		);

		// Preserve one attempt deadline, even if the owner starts waiting later.
		let deadline_boot_ms = deps
			.env
			.clock()
			.observe()?
			.boot_ms
			.checked_add(config.timeout_ms)
			.ok_or_else(|| anyhow::anyhow!("recovery deadline overflow"))?;
		let state = VolumeClientState::recover_with_required_members(
			config.client.clone(),
			key,
			replica_set,
			config.required_members,
		)?;

		// The owner retains task cleanup while the executor holds only a weak scope.
		let tasks = Arc::new(deps.env.task_scope());
		let closing = Arc::new(AtomicBool::new(false));
		let (send, result) = oneshot::channel();
		let executor = Arc::new(RecoveryExecutor {
			payloads: Arc::new(Mutex::new(Payloads::default())),
			tasks: Arc::downgrade(&tasks),
			closing: closing.clone(),
			result: Mutex::new(Some(send)),
			transport: deps.transport,
			authority: deps.authority.clone(),
			config: config.client,
		});

		let handle = Runner::<VolumeClientMachine>::spawn(deps.env.clone(), state, executor);
		Ok(Self {
			snapshot: handle.snapshot,
			inbox: Some(handle.inbox),
			result: Some(result),
			stopped: Some(handle.stopped),
			env: deps.env,
			tasks,
			closing,
			authority: deps.authority,
			deadline_boot_ms,
		})
	}

	/// Waits once within the original attempt deadline. The owner still calls shutdown after any outcome.
	#[tracing::instrument(skip_all)]
	pub async fn wait(&mut self) -> Result<RecoveryReport, WriteError> {
		let Some(result) = self.result.as_mut() else {
			return Err(WriteError::RecoveryFailed);
		};
		let outcome =
			rivet_env::timeout_at(self.env.clock().as_ref(), self.deadline_boot_ms, result).await;
		self.result.take();
		if !self.authority.may_run() {
			return Err(WriteError::Fenced { token: None });
		}

		match outcome {
			Ok(Ok(result)) => result,
			Ok(Err(_)) => Err(WriteError::RecoveryFailed),
			Err(_) => Err(WriteError::QuorumUnavailable { token: None }),
		}
	}

	/// Stops admission, cancels and joins transport futures, then joins the pure runner.
	#[tracing::instrument(skip_all)]
	pub async fn shutdown(mut self) -> anyhow::Result<()> {
		self.closing.store(true, Ordering::Release);
		self.inbox.take();
		self.tasks.shutdown().await;

		// Every transport task holds an inbox sender until it ends. Joining the runner
		// also covers any canceled task registered concurrently with scope shutdown.
		let result = if let Some(stopped) = self.stopped.as_mut() {
			stopped.await.map_err(anyhow::Error::from)
		} else {
			Ok(())
		};
		self.stopped.take();

		result
	}
}

impl Drop for Recovery {
	fn drop(&mut self) {
		let Some(stopped) = self.stopped.take() else {
			return;
		};
		self.closing.store(true, Ordering::Release);
		self.inbox.take();
		let tasks = self.tasks.clone();
		self.env.spawn(Box::pin(async move {
			tasks.shutdown().await;
			let _ = stopped.await;
		}));
	}
}

struct RecoveryExecutor {
	payloads: Arc<Mutex<Payloads>>,
	tasks: Weak<TaskScope>,
	closing: Arc<AtomicBool>,
	result: Mutex<Option<oneshot::Sender<Result<RecoveryReport, WriteError>>>>,
	transport: Arc<dyn VolumeClientTransport>,
	authority: Arc<dyn RecoveryAuthority>,
	config: VolumeClientConfig,
}

impl Executor<VolumeClientMachine> for RecoveryExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<VolumeClientMachine>) {
		if self.closing.load(Ordering::Acquire) {
			return;
		}
		match effect {
			Effect::RecoveryResponse { result } => {
				let result = if self.authority.may_run() {
					result
				} else {
					Err(WriteError::Fenced { token: None })
				};
				if let Some(send) = self.result.lock().take() {
					let _ = send.send(result);
				}
			}
			Effect::ReleasePayload { payload } => self.payloads.lock().release(payload),
			Effect::OpenResponse { .. }
			| Effect::CloseResponse { .. }
			| Effect::WriteResponse { .. }
			| Effect::WaitForCommitResponse { .. }
			| Effect::ConfirmResponse { .. } => {
				tracing::error!("application reply emitted by recovery-only client");
				if let Some(tasks) = self.tasks.upgrade() {
					let inbox = ctx.inbox().clone();
					ctx.spawn_in(&tasks, async move {
						let _ = inbox.send(Input::Fence).await;
					});
				}
			}
			request => self.request(request, ctx),
		}
	}
}

impl RecoveryExecutor {
	/// Shares request execution with application clients while retaining this recovery attempt's authority.
	fn request(&self, request: Effect, ctx: EffectContext<VolumeClientMachine>) {
		let authority = self.authority.clone();
		let closing = self.closing.clone();
		super::requests::RequestExecutor {
			payloads: self.payloads.clone(),
			tasks: self.tasks.clone(),
			transport: self.transport.clone(),
			may_dispatch: Arc::new(move || !closing.load(Ordering::Acquire) && authority.may_run()),
			config: self.config.clone(),
			on_revoked: Some(Input::Fence),
		}
		.request(request, ctx);
	}
}
