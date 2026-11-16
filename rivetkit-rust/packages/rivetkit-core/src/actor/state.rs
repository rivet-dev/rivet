use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};
use std::time::Duration;

use crate::time::Instant as StdInstant;
use crate::time::sleep;

use anyhow::{Context, Result};
use rivetkit_actor_persist::{generated::v4 as persist_v4, versioned as persist_versioned};
#[cfg(not(feature = "wasm-runtime"))]
use tokio::runtime::Handle;
use tokio::sync::{Mutex as AsyncMutex, Notify, OwnedMutexGuard, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::actor::connection::{PersistedConnection, decode_persisted_connection};
use crate::actor::context::ActorContext;
use crate::actor::internal_storage;
use crate::actor::lifecycle_hooks::Reply;
use crate::actor::messages::{StateDelta, WorkflowKvWrite};
use crate::actor::persist::decode_latest_with_embedded_version;
use crate::actor::task::LifecycleEvent;
use crate::actor::task_types::StateMutationReason;
use crate::error::{ActorLifecycle, ActorRuntime};
use crate::runtime::RuntimeSpawner;
use crate::sqlite::{BindParam, ExecuteResult, SqliteBatchStatement, SqliteTransaction};
use crate::types::SaveStateOpts;

pub type PersistedScheduleEvent = persist_v4::ScheduleEvent;
pub type PersistedActor = persist_v4::Actor;

pub(crate) fn decode_persisted_actor(payload: &[u8]) -> Result<PersistedActor> {
	let actor = decode_latest_with_embedded_version::<persist_versioned::Actor>(
		payload,
		"persisted actor",
	)?;
	Ok(actor)
}

pub(crate) fn decode_last_pushed_alarm(payload: &[u8]) -> Result<Option<i64>> {
	decode_latest_with_embedded_version::<persist_versioned::LastPushedAlarm>(
		payload,
		"last pushed alarm",
	)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestSaveOpts {
	pub immediate: bool,
	pub max_wait_ms: Option<u32>,
}

pub(super) struct PendingSave {
	scheduled_at: StdInstant,
	handle: JoinHandle<()>,
}

pub struct OnStateChangeGuard {
	ctx: Option<ActorContext>,
}
/// A SQLite transaction that owns the actor state-save exclusion until it is
/// committed or rolled back.
#[derive(Clone)]
pub struct ActorStateTransaction {
	owner: Arc<AsyncMutex<ActorStateTransactionOwner>>,
}

/// Which state transaction, if any, holds the actor's `save_guard`.
///
/// The actor task reads this to defer saves and workflow flushes instead of
/// waiting on a lock that user code owns. Every transition except `Closed`
/// happens while the guard is held, so a later transaction cannot be
/// overwritten by an earlier one finishing.
pub(super) enum StateTransactionSlot {
	/// No state transaction holds `save_guard`.
	Idle,
	/// `begin_state_transaction` holds `save_guard` while it opens the SQLite
	/// transaction. It moves to `Open` on success or back to `Idle` on failure
	/// or cancellation.
	Beginning,
	/// An open transaction holds `save_guard` until it is committed, rolled back,
	/// or dropped. Shutdown uses the weak owner to roll it back.
	Open(Weak<AsyncMutex<ActorStateTransactionOwner>>),
	/// Shutdown closed state transactions. This state is terminal: new
	/// transactions are rejected, and a transaction that was beginning rolls
	/// itself back instead of opening.
	Closed,
}

/// Holds the actor's `save_guard` on behalf of one state transaction.
///
/// Dropping it returns the slot to `Idle` unless shutdown closed it, releases
/// the guard, and wakes the actor task to resume saves and workflow flushes it
/// deferred. This also runs when `begin_state_transaction` is cancelled, so a
/// dropped begin never leaves the slot stuck in `Beginning`.
struct StateSaveExclusion {
	ctx: ActorContext,
	save_guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for StateSaveExclusion {
	fn drop(&mut self) {
		{
			let mut slot = self.ctx.0.state_transaction.lock();
			match *slot {
				StateTransactionSlot::Closed => {}
				StateTransactionSlot::Idle
				| StateTransactionSlot::Beginning
				| StateTransactionSlot::Open(_) => *slot = StateTransactionSlot::Idle,
			}
		}

		// Release the guard before the wakeup so the actor task can take it.
		self.save_guard.take();
		self.ctx.notify_state_transaction_finished();
	}
}

pub(super) struct ActorStateTransactionOwner {
	ctx: ActorContext,
	transaction: SqliteTransaction,
	/// Released after the `Drop` body runs, so connection states are restored
	/// and the epoch is advanced before a blocked save can take the guard.
	save_exclusion: Option<StateSaveExclusion>,
	write_guard: Option<InFlightWrite>,
	finalized: bool,
	committed: bool,
	save_request_revision: u64,
	statement_count: usize,
	bind_payload_bytes: usize,
	rollback_connection_states: Vec<(String, Vec<u8>)>,
}

impl ActorStateTransactionOwner {
	fn restore_connection_states(&self) {
		for (conn_id, state) in &self.rollback_connection_states {
			if let Some(conn) = self.ctx.connection(conn_id) {
				conn.set_state(state.clone());
			}
		}
	}

	fn advance_epoch(&self) {
		self.ctx
			.0
			.state_transaction_epoch
			.fetch_add(1, Ordering::SeqCst);
	}
}

impl Drop for ActorStateTransactionOwner {
	fn drop(&mut self) {
		if !self.finalized {
			self.restore_connection_states();
			self.advance_epoch();
			let transaction = self.transaction.clone();
			#[cfg(not(feature = "wasm-runtime"))]
			if let Ok(handle) = Handle::try_current() {
				handle.spawn(async move {
					if let Err(error) = transaction.rollback().await {
						tracing::debug!(?error, "dropped actor state transaction rollback failed");
					}
				});
			}
			#[cfg(feature = "wasm-runtime")]
			wasm_bindgen_futures::spawn_local(async move {
				if let Err(error) = transaction.rollback().await {
					tracing::debug!(?error, "dropped actor state transaction rollback failed");
				}
			});
		}
		if !self.committed {
			self.ctx.schedule_save(None);
		}
	}
}

impl ActorStateTransaction {
	pub async fn execute(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		let mut owner = self.owner.lock().await;
		if owner.finalized {
			return Err(anyhow::anyhow!(
				"actor state transaction is already finalized"
			));
		}
		let sql = sql.into();
		if is_transaction_terminator(&sql) {
			return Err(anyhow::anyhow!(
				"cannot execute transaction terminator inside an actor state transaction; use the transaction commit or rollback method"
			));
		}
		let payload_bytes = match params.as_deref() {
			Some(params) => params
				.iter()
				.map(internal_storage::bind_param_payload_len)
				.fold(0usize, usize::saturating_add),
			None => sql.len(),
		};
		let result = owner.transaction.execute(sql, params).await?;
		owner.statement_count = owner.statement_count.saturating_add(1);
		owner.bind_payload_bytes = owner.bind_payload_bytes.saturating_add(payload_bytes);
		Ok(result)
	}

	pub async fn commit(&self, deltas: Vec<StateDelta>) -> Result<()> {
		let mut owner = self.owner.lock().await;
		if owner.finalized {
			return Err(anyhow::anyhow!(
				"actor state transaction is already finalized"
			));
		}
		// The actor's teardown, including a failed start's, closed state transactions while this
		// commit waited for the transaction, so it rolls back instead: a teardown that stopped
		// waiting for its rollback must never see the transaction commit after it. The caller
		// sees the structured `actor.stopping`, as a transaction begun after the teardown does.
		if owner.ctx.state_transactions_closed() {
			if let Err(error) = roll_back_owner(&mut owner).await {
				tracing::debug!(?error, "closed state transaction rollback failed");
			}
			return Err(ActorLifecycle::Stopping.build());
		}

		let save_request_revision = owner.save_request_revision;
		let transaction = owner.transaction.clone();
		let statement_count = owner.statement_count;
		let bind_payload_bytes = owner.bind_payload_bytes;
		let result = owner
			.ctx
			.commit_state_transaction(
				&transaction,
				deltas,
				save_request_revision,
				statement_count,
				bind_payload_bytes,
			)
			.await;
		owner.finalized = true;
		owner.committed = result.is_ok();
		if result.is_err() {
			owner.restore_connection_states();
		}
		owner.advance_epoch();
		owner.write_guard.take();
		owner.save_exclusion.take();
		if result.is_err() {
			owner.ctx.schedule_save(None);
		}
		result
	}

	pub async fn rollback(&self) -> Result<()> {
		let mut owner = self.owner.lock().await;
		if owner.finalized {
			return Err(anyhow::anyhow!(
				"actor state transaction is already finalized"
			));
		}
		roll_back_owner(&mut owner).await
	}
}

/// Rolls back the unfinalized transaction `owner` holds, restores the connection states it
/// changed, and releases the save exclusion.
async fn roll_back_owner(owner: &mut ActorStateTransactionOwner) -> Result<()> {
	let result = owner.transaction.rollback().await;
	owner.finalized = true;
	owner.restore_connection_states();
	owner.advance_epoch();
	owner.write_guard.take();
	owner.save_exclusion.take();
	owner.ctx.schedule_save(None);
	result
}

/// What [`ActorContext::close_state_transactions_within`] found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StateTransactionClose {
	/// No state transaction was open, so nothing was rolled back.
	NoneOpen,
	/// The open state transaction was rolled back.
	RolledBack,
	/// The open state transaction's rollback did not finish within the limit, as when user code
	/// holds the transaction mid-statement. The transaction still never commits: its commit
	/// rolls back once it runs, since state transactions are closed.
	RollbackTimedOut,
}

/// Rolls back `transaction`, which closing state transactions found open. A transaction that
/// finalized first is left as it is.
async fn roll_back_closed(transaction: ActorStateTransaction) {
	if let Err(error) = transaction.rollback().await {
		tracing::debug!(
			?error,
			"state transaction finalized before its teardown rollback"
		);
	}
}

fn is_transaction_terminator(sql: &str) -> bool {
	let mut offset = 0;
	let Some(first) = next_sql_keyword(sql, &mut offset) else {
		return false;
	};
	if first.eq_ignore_ascii_case("COMMIT") || first.eq_ignore_ascii_case("END") {
		return true;
	}
	if !first.eq_ignore_ascii_case("ROLLBACK") {
		return false;
	}

	let mut next = next_sql_keyword(sql, &mut offset);
	if next.is_some_and(|keyword| keyword.eq_ignore_ascii_case("TRANSACTION")) {
		next = next_sql_keyword(sql, &mut offset);
	}
	!next.is_some_and(|keyword| keyword.eq_ignore_ascii_case("TO"))
}

fn next_sql_keyword<'a>(sql: &'a str, offset: &mut usize) -> Option<&'a str> {
	let bytes = sql.as_bytes();
	loop {
		while bytes.get(*offset).is_some_and(u8::is_ascii_whitespace) {
			*offset += 1;
		}
		if bytes.get(*offset..*offset + 2) == Some(b"--") {
			*offset += 2;
			while bytes.get(*offset).is_some_and(|byte| *byte != b'\n') {
				*offset += 1;
			}
			continue;
		}
		if bytes.get(*offset..*offset + 2) == Some(b"/*") {
			*offset += 2;
			while bytes.get(*offset..*offset + 2) != Some(b"*/") {
				bytes.get(*offset)?;
				*offset += 1;
			}
			*offset += 2;
			continue;
		}
		break;
	}

	let start = *offset;
	while bytes.get(*offset).is_some_and(u8::is_ascii_alphabetic) {
		*offset += 1;
	}
	(start != *offset).then(|| &sql[start..*offset])
}

impl OnStateChangeGuard {
	fn new(ctx: ActorContext) -> Self {
		ctx.on_state_change_started();
		Self { ctx: Some(ctx) }
	}
}

impl Drop for OnStateChangeGuard {
	fn drop(&mut self) {
		if let Some(ctx) = self.ctx.take() {
			ctx.on_state_change_finished();
		}
	}
}

impl ActorContext {
	pub async fn begin_state_transaction(
		&self,
		timeout: Option<Duration>,
	) -> Result<ActorStateTransaction> {
		self.clear_pending_save();
		let save_guard = Arc::clone(&self.0.save_guard).lock_owned().await;
		{
			// Holding the guard means no other transaction occupies the slot.
			let mut slot = self.0.state_transaction.lock();
			match *slot {
				StateTransactionSlot::Closed => {
					return Err(ActorLifecycle::Stopping.build());
				}
				StateTransactionSlot::Idle
				| StateTransactionSlot::Beginning
				| StateTransactionSlot::Open(_) => *slot = StateTransactionSlot::Beginning,
			}
		}
		let save_exclusion = StateSaveExclusion {
			ctx: self.clone(),
			save_guard: Some(save_guard),
		};
		self.0
			.state_transaction_epoch
			.fetch_add(1, Ordering::SeqCst);
		self.wait_for_in_flight_writes().await;
		let rollback_connection_states = self
			.iter_connections()
			.filter(|conn| conn.is_hibernatable())
			.map(|conn| (conn.id().to_owned(), conn.state()))
			.collect();
		let sql = self.invocation_sql();
		let transaction = match sql.begin_transaction(timeout).await {
			Ok(transaction) => transaction,
			Err(error) => {
				self.0
					.state_transaction_epoch
					.fetch_add(1, Ordering::SeqCst);
				drop(save_exclusion);
				self.schedule_save(None);
				return Err(error);
			}
		};
		let write_guard = self.begin_write();
		let save_request_revision = self.save_request_revision();
		let owner = Arc::new(AsyncMutex::new(ActorStateTransactionOwner {
			ctx: self.clone(),
			transaction,
			save_exclusion: Some(save_exclusion),
			write_guard: Some(write_guard),
			finalized: false,
			committed: false,
			save_request_revision,
			statement_count: 0,
			bind_payload_bytes: 0,
			rollback_connection_states,
		}));

		// Shutdown may have closed state transactions while this one was
		// beginning. It cannot see a transaction that is not open yet, so the
		// transaction rolls itself back instead of opening.
		let opened = {
			let mut slot = self.0.state_transaction.lock();
			match *slot {
				StateTransactionSlot::Closed => false,
				StateTransactionSlot::Idle
				| StateTransactionSlot::Beginning
				| StateTransactionSlot::Open(_) => {
					*slot = StateTransactionSlot::Open(Arc::downgrade(&owner));
					true
				}
			}
		};
		let transaction = ActorStateTransaction { owner };
		if !opened {
			if let Err(error) = transaction.rollback().await {
				tracing::debug!(?error, "state transaction rollback at shutdown failed");
			}
			return Err(ActorLifecycle::Stopping.build());
		}
		Ok(transaction)
	}

	/// True while a state transaction holds the state-save exclusion.
	pub(crate) fn state_transaction_active(&self) -> bool {
		match *self.0.state_transaction.lock() {
			StateTransactionSlot::Idle | StateTransactionSlot::Closed => false,
			StateTransactionSlot::Beginning | StateTransactionSlot::Open(_) => true,
		}
	}

	/// Closes state transactions for the rest of this actor instance so the
	/// final save does not wait on user code that never commits.
	///
	/// An open transaction is rolled back here, a transaction that is still
	/// beginning rolls itself back instead of opening, and later begins are
	/// rejected. A transaction already committing finishes first, and its
	/// rollback is then a no-op; a commit that starts later rolls back instead.
	pub(crate) async fn close_state_transactions(&self) {
		let Some(transaction) = self.close_state_transaction_slot() else {
			return;
		};

		tracing::warn!(
			actor_id = %self.actor_id(),
			"rolling back state transaction left open at shutdown; the action did not finish within the grace period"
		);
		roll_back_closed(transaction).await;
	}

	/// Closes state transactions like [`Self::close_state_transactions`], for a teardown that
	/// cannot wait on user code: the rollback waits on the transaction's owner, which a hook
	/// stuck mid-statement holds, so it stops waiting after `limit`. Logs nothing, so the caller
	/// logs the event once with what this found.
	pub(crate) async fn close_state_transactions_within(
		&self,
		limit: Duration,
	) -> StateTransactionClose {
		let Some(transaction) = self.close_state_transaction_slot() else {
			return StateTransactionClose::NoneOpen;
		};
		match crate::time::timeout(limit, roll_back_closed(transaction)).await {
			Ok(()) => StateTransactionClose::RolledBack,
			Err(_) => StateTransactionClose::RollbackTimedOut,
		}
	}

	/// Whether the actor's teardown closed state transactions for good.
	fn state_transactions_closed(&self) -> bool {
		match *self.0.state_transaction.lock() {
			StateTransactionSlot::Closed => true,
			StateTransactionSlot::Idle
			| StateTransactionSlot::Beginning
			| StateTransactionSlot::Open(_) => false,
		}
	}

	/// Marks state transactions closed and answers the transaction that was open, if any.
	fn close_state_transaction_slot(&self) -> Option<ActorStateTransaction> {
		let previous = std::mem::replace(
			&mut *self.0.state_transaction.lock(),
			StateTransactionSlot::Closed,
		);
		let owner = match previous {
			StateTransactionSlot::Open(owner) => owner.upgrade(),
			StateTransactionSlot::Idle
			| StateTransactionSlot::Beginning
			| StateTransactionSlot::Closed => None,
		}?;
		Some(ActorStateTransaction { owner })
	}

	/// Wakes the actor task after a state transaction releases `save_guard`.
	fn notify_state_transaction_finished(&self) {
		if let Some(sender) = self.lifecycle_event_sender() {
			let _ = sender.send(LifecycleEvent::StateTransactionFinished);
		}
	}

	pub fn state(&self) -> Vec<u8> {
		self.0.current_state.read().clone()
	}

	pub(crate) async fn persist_state(&self, opts: SaveStateOpts) -> Result<()> {
		self.persist_state_carrying(opts, &mut Vec::new()).await
	}

	/// Like `persist_state`, but the batch that writes the state also runs the statements in
	/// `carried`, which it empties. `carried` keeps its statements when the state was clean.
	pub(crate) async fn persist_state_carrying(
		&self,
		opts: SaveStateOpts,
		carried: &mut Vec<SqliteBatchStatement>,
	) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		let result = if opts.immediate {
			self.clear_pending_save();
			self.persist_if_dirty_carrying(carried).await
		} else {
			let delay = self.compute_save_delay(None);
			if !delay.is_zero() {
				sleep(delay).await;
			}
			self.persist_if_dirty_carrying(carried).await
		};
		result?;
		self.record_state_updated();
		Ok(())
	}

	/// Foreign-runtime bootstrap hook for installing the actor state snapshot
	/// before the actor starts handling lifecycle/dispatch work.
	pub fn set_state_initial(&self, state: Vec<u8>) {
		self.set_initial_state(state);
	}

	/// Fire-and-forget save request helper.
	///
	/// If the lifecycle event inbox is unavailable, this only logs a warning and
	/// returns. That `warn!` is the sole failure signal for this path; callers do
	/// not receive a `Result`. Call
	/// [`Self::request_save_and_wait`] when the caller must observe
	/// save-request delivery failures.
	pub fn request_save(&self, opts: RequestSaveOpts) {
		#[cfg(target_arch = "wasm32")]
		{
			self.request_save_best_effort(opts);
		}

		#[cfg(not(target_arch = "wasm32"))]
		if let Err(error) = self.request_save_with_revision(opts) {
			tracing::warn!(?error, "failed to request actor state save");
		}
	}

	#[cfg(target_arch = "wasm32")]
	fn request_save_best_effort(&self, opts: RequestSaveOpts) {
		let immediate = opts.immediate;
		let _save_request_revision =
			self.0.save_request_revision.fetch_add(1, Ordering::SeqCst) + 1;
		self.notify_request_save_hooks(opts);
		let already_requested = self.0.save_requested.swap(true, Ordering::SeqCst);
		let immediate_already_requested = if immediate {
			self.0.save_requested_immediate.swap(true, Ordering::SeqCst)
		} else {
			self.0.save_requested_immediate.load(Ordering::SeqCst)
		};

		if let Some(max_wait_ms) = opts.max_wait_ms {
			let deadline = StdInstant::now() + Duration::from_millis(u64::from(max_wait_ms));
			let mut requested_deadline = self.0.save_requested_within_deadline.lock();
			*requested_deadline = Some(match *requested_deadline {
				Some(existing) => existing.min(deadline),
				None => deadline,
			});
		}

		let Some(sender) = self.lifecycle_event_sender() else {
			return;
		};

		if opts.max_wait_ms.is_none()
			&& already_requested
			&& (!immediate || immediate_already_requested)
		{
			return;
		}

		let _ = sender.send(LifecycleEvent::SaveRequested { immediate });
	}

	/// Requests a save and resolves once the state current at the request has been written.
	/// Fails with `actor/stopping` when the actor shuts down without writing it.
	pub async fn request_save_and_wait(&self, opts: RequestSaveOpts) -> Result<()> {
		let save_request_revision = self.request_save_with_revision(opts)?;
		self.wait_for_save_request(save_request_revision).await
	}

	pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()> {
		let save_request_revision = self.save_request_revision();
		self.save_state_with_revision(deltas, save_request_revision)
			.await
	}

	/// Requests one logical workflow-engine flush. The actor lifecycle owns
	/// serializing actor state and committing both sides in one SQLite transaction.
	pub async fn save_state_and_workflow_batch(
		&self,
		workflow_writes: Vec<WorkflowKvWrite>,
	) -> Result<()> {
		let Some(sender) = self.lifecycle_event_sender() else {
			return Err(ActorRuntime::NotConfigured {
				component: "lifecycle events".to_owned(),
			}
			.build());
		};
		let (reply_tx, reply_rx) = oneshot::channel();
		sender
			.send(LifecycleEvent::WorkflowFlushRequested {
				writes: workflow_writes,
				reply: Reply::from(reply_tx),
			})
			.map_err(|_| {
				ActorRuntime::NotConfigured {
					component: "lifecycle events".to_owned(),
				}
				.build()
			})?;
		reply_rx
			.await
			.context("receive workflow flush lifecycle reply")?
	}

	pub(crate) fn request_save_with_revision(&self, opts: RequestSaveOpts) -> Result<u64> {
		let immediate = opts.immediate;
		let save_request_revision = self.0.save_request_revision.fetch_add(1, Ordering::SeqCst) + 1;
		self.notify_request_save_hooks(opts);
		let already_requested = self.0.save_requested.swap(true, Ordering::SeqCst);
		let immediate_already_requested = if immediate {
			self.0.save_requested_immediate.swap(true, Ordering::SeqCst)
		} else {
			self.0.save_requested_immediate.load(Ordering::SeqCst)
		};

		if let Some(max_wait_ms) = opts.max_wait_ms {
			let deadline = StdInstant::now() + Duration::from_millis(u64::from(max_wait_ms));
			let mut requested_deadline = self.0.save_requested_within_deadline.lock();
			*requested_deadline = Some(match *requested_deadline {
				Some(existing) => existing.min(deadline),
				None => deadline,
			});
		}

		let Some(sender) = self.lifecycle_event_sender() else {
			return Err(ActorRuntime::NotConfigured {
				component: "lifecycle events".to_owned(),
			}
			.build());
		};

		if opts.max_wait_ms.is_none()
			&& already_requested
			&& (!immediate || immediate_already_requested)
		{
			return Ok(save_request_revision);
		}

		sender
			.send(LifecycleEvent::SaveRequested { immediate })
			.map(|()| save_request_revision)
			.map_err(|_| {
				ActorRuntime::NotConfigured {
					component: "lifecycle events".to_owned(),
				}
				.build()
			})
	}

	pub(crate) async fn wait_for_save_request(&self, save_request_revision: u64) -> Result<()> {
		wait_for_notified_condition(&self.0.save_completion, || {
			self.0.save_completed_revision.load(Ordering::SeqCst) >= save_request_revision
				|| self.0.save_failed_revision.load(Ordering::SeqCst) >= save_request_revision
		})
		.await;
		if self.0.save_completed_revision.load(Ordering::SeqCst) >= save_request_revision {
			return Ok(());
		}
		Err(ActorLifecycle::Stopping.build())
	}

	/// Keeps a failed save requested but drops its immediate priority and restarts the save
	/// throttle from the failure, so a failure that repeats is retried once per save interval
	/// instead of in a tight loop. The throttle otherwise counts from the last successful
	/// save, which a failing actor never makes.
	pub(crate) fn throttle_failed_save_request(&self) {
		self.0
			.save_requested_immediate
			.store(false, Ordering::SeqCst);
		*self.0.last_save_at.lock() = Some(StdInstant::now());
	}

	/// Fails every save request that has not been written, including any made later.
	/// Shutdown calls this once no later save can run, so waiters never resolve as if a
	/// write had happened and a request made afterwards fails instead of waiting forever.
	pub(crate) fn fail_unpersisted_save_requests(&self) {
		self.0
			.save_failed_revision
			.store(u64::MAX, Ordering::SeqCst);
		self.0.save_completion.notify_waiters();
	}

	pub(crate) fn save_requested(&self) -> bool {
		self.0.save_requested.load(Ordering::SeqCst)
	}

	pub(crate) fn save_requested_immediate(&self) -> bool {
		self.0.save_requested_immediate.load(Ordering::SeqCst)
	}

	pub(crate) fn save_deadline(&self, immediate: bool) -> StdInstant {
		self.compute_save_deadline(immediate)
	}

	pub(crate) fn compute_save_deadline(&self, immediate: bool) -> StdInstant {
		if immediate || self.save_requested_immediate() {
			return StdInstant::now();
		}

		let throttled_deadline = StdInstant::now() + self.compute_save_delay(None);
		let requested_deadline = *self.0.save_requested_within_deadline.lock();

		match requested_deadline {
			Some(requested_deadline) => throttled_deadline.min(requested_deadline),
			None => throttled_deadline,
		}
	}

	pub(crate) fn save_request_revision(&self) -> u64 {
		self.0.save_request_revision.load(Ordering::SeqCst)
	}

	pub(crate) async fn apply_state_deltas(
		&self,
		deltas: Vec<StateDelta>,
		save_request_revision: u64,
	) -> Result<()> {
		self.apply_state_deltas_inner(deltas, None, save_request_revision, None, &mut Vec::new())
			.await
			.map(|_| ())
	}

	/// Applies `deltas` and writes them in one batch. A batch that writes state or connections
	/// without workflow writes also runs, and empties, the statements in `carried`; a call that
	/// writes nothing leaves them for the caller.
	pub(super) async fn apply_state_deltas_inner(
		&self,
		deltas: Vec<StateDelta>,
		workflow_writes: Option<Vec<WorkflowKvWrite>>,
		save_request_revision: u64,
		expected_state_transaction_epoch: Option<u64>,
		carried: &mut Vec<SqliteBatchStatement>,
	) -> Result<bool> {
		let delta_count = deltas.len();
		let delta_bytes: usize = deltas.iter().map(StateDelta::payload_len).sum();
		let workflow_write_count = workflow_writes.as_ref().map_or(0, Vec::len);
		let current_revision = self.0.state_revision.load(Ordering::SeqCst);
		tracing::debug!(
			delta_count,
			delta_bytes,
			state_revision = current_revision,
			save_request_revision,
			"applying actor state deltas"
		);
		self.clear_pending_save();

		if deltas.is_empty() && workflow_write_count == 0 {
			self.mark_save_request_completed(save_request_revision);
			self.finish_save_request(save_request_revision);
			tracing::debug!(
				delta_count,
				state_revision = current_revision,
				save_request_revision,
				"actor state deltas applied without kv write"
			);
			return Ok(true);
		}

		// The actor task passes an expected epoch and must not wait on a guard
		// that a user state transaction may hold for as long as its callback
		// runs. Contention discards the serialized state like an epoch change,
		// and the task defers the retry until the transaction finishes.
		let save_guard = match expected_state_transaction_epoch {
			Some(_) => match self.0.save_guard.try_lock() {
				Ok(save_guard) => save_guard,
				Err(_) => {
					tracing::debug!(
						save_request_revision,
						"state save guard is held, deferring actor task save"
					);
					return Ok(false);
				}
			},
			None => self.0.save_guard.lock().await,
		};
		let prepared = {
			let _save_guard = save_guard;
			if expected_state_transaction_epoch.is_some_and(|expected| {
				self.0.state_transaction_epoch.load(Ordering::SeqCst) != expected
			}) {
				None
			} else {
				let revision = self.0.state_revision.load(Ordering::SeqCst);
				let mut persisted = self.persisted();
				let mut next_state = None;
				let mut actor_to_persist = None;
				let mut connections_to_persist: Vec<PersistedConnection> = Vec::new();
				let mut connections_to_delete = Vec::new();

				for delta in deltas {
					match delta {
						StateDelta::ActorState(bytes) => {
							next_state = Some(bytes.clone());
							persisted.state = bytes.into();
						}
						StateDelta::ConnHibernation { conn: _, bytes } => {
							connections_to_persist.push(
								decode_persisted_connection(&bytes)
									.context("decode hibernatable connection state delta")?,
							);
						}
						StateDelta::ConnHibernationRemoved(conn) => {
							connections_to_delete.push(conn);
						}
					}
				}

				if next_state.is_some() {
					actor_to_persist = Some(persisted.clone());
				}

				Some((
					next_state,
					actor_to_persist,
					connections_to_persist,
					connections_to_delete,
					revision,
					self.begin_write(),
				))
			}
		};
		let Some((
			next_state,
			actor_to_persist,
			connections_to_persist,
			connections_to_delete,
			revision,
			_write_guard,
		)) = prepared
		else {
			tracing::debug!(
				save_request_revision,
				"discarding state serialized across a state transaction boundary"
			);
			return Ok(false);
		};

		if let Some(workflow_writes) = workflow_writes.as_deref() {
			internal_storage::persist_actor_core_connections_and_workflow(
				self.sql(),
				actor_to_persist.as_ref(),
				&connections_to_persist,
				&connections_to_delete,
				workflow_writes,
			)
			.await
			.context("atomically persist actor state, connection deltas, and workflow kv")?;
		} else if actor_to_persist.is_some()
			|| !connections_to_persist.is_empty()
			|| !connections_to_delete.is_empty()
		{
			internal_storage::persist_actor_core_and_connections_with(
				self.sql(),
				actor_to_persist.as_ref(),
				&connections_to_persist,
				&connections_to_delete,
				std::mem::take(carried),
			)
			.await
			.context("persist actor state and connection deltas to sqlite")?;
		}

		if let Some(state) = next_state {
			self.0.persisted.write().state = state.clone().into();
			*self.0.current_state.write() = state;
		}
		for connection in &connections_to_persist {
			if let Some(handle) = self.connection(&connection.id) {
				handle.set_state_initial(connection.state.clone().to_vec());
			}
		}

		*self.0.last_save_at.lock() = Some(StdInstant::now());

		if self.0.state_revision.load(Ordering::SeqCst) == revision {
			self.0.state_dirty.store(false, Ordering::SeqCst);
		}

		self.mark_save_request_completed(save_request_revision);
		self.finish_save_request(save_request_revision);
		tracing::debug!(
			delta_count,
			delta_bytes,
			workflow_write_count,
			state_revision = self.0.state_revision.load(Ordering::SeqCst),
			save_request_revision,
			"actor state deltas applied"
		);
		Ok(true)
	}
	async fn commit_state_transaction(
		&self,
		transaction: &SqliteTransaction,
		deltas: Vec<StateDelta>,
		save_request_revision: u64,
		statement_count: usize,
		bind_payload_bytes: usize,
	) -> Result<()> {
		let (deltas, pending_hibernation_changes) = match self.prepare_state_deltas(deltas) {
			Ok(prepared) => prepared,
			Err(error) => {
				let _ = transaction.rollback().await;
				return Err(error);
			}
		};
		let commit_result = async {
			let revision = self.0.state_revision.load(Ordering::SeqCst);
			let mut persisted = self.persisted();
			let mut next_state = None;
			let mut actor_to_persist = None;
			let mut connections_to_persist: Vec<PersistedConnection> = Vec::new();
			let mut connections_to_delete = Vec::new();

			for delta in deltas {
				match delta {
					StateDelta::ActorState(bytes) => {
						next_state = Some(bytes.clone());
						persisted.state = bytes.into();
					}
					StateDelta::ConnHibernation { conn: _, bytes } => {
						connections_to_persist.push(
							decode_persisted_connection(&bytes)
								.context("decode hibernatable connection state delta")?,
						);
					}
					StateDelta::ConnHibernationRemoved(conn) => {
						connections_to_delete.push(conn);
					}
				}
			}

			if next_state.is_some() {
				actor_to_persist = Some(persisted);
			}
			let statements = internal_storage::build_actor_core_and_connection_statements(
				actor_to_persist.as_ref(),
				&connections_to_persist,
				&connections_to_delete,
			)?;
			internal_storage::validate_atomic_state_transaction_budget(
				statement_count.saturating_add(statements.len()),
				bind_payload_bytes
					.saturating_add(internal_storage::statement_bind_payload_len(&statements)),
			)?;
			for statement in statements {
				transaction
					.execute(statement.sql, statement.params)
					.await
					.context("persist actor state inside sqlite transaction")?;
			}
			transaction
				.commit()
				.await
				.context("commit sqlite transaction with actor state")?;

			if let Some(state) = next_state {
				self.0.persisted.write().state = state.clone().into();
				*self.0.current_state.write() = state;
			}
			for connection in &connections_to_persist {
				if let Some(handle) = self.connection(&connection.id) {
					handle.set_state_initial(connection.state.clone().to_vec());
				}
			}
			*self.0.last_save_at.lock() = Some(StdInstant::now());
			if self.0.state_revision.load(Ordering::SeqCst) == revision {
				self.0.state_dirty.store(false, Ordering::SeqCst);
			}
			self.mark_save_request_completed(save_request_revision);
			self.finish_save_request(save_request_revision);
			self.record_state_updated();
			Ok(())
		}
		.await;

		if let Err(error) = commit_result {
			self.restore_pending_hibernation_changes(pending_hibernation_changes);
			let _ = transaction.rollback().await;
			return Err(error);
		}
		Ok(())
	}

	pub(crate) async fn wait_for_pending_writes(&self) {
		loop {
			if let Some(handle) = self.take_tracked_persist() {
				let _ = handle.await;
				continue;
			}

			let save_guard = self.0.save_guard.lock().await;
			if self.has_tracked_persist() {
				drop(save_guard);
				continue;
			}

			if self.0.in_flight_state_writes.load(Ordering::SeqCst) == 0 {
				return;
			}
			drop(save_guard);

			self.wait_for_in_flight_writes().await;
		}
	}

	pub(crate) async fn wait_for_pending_state_writes(&self) {
		self.wait_for_pending_writes().await;
	}

	pub fn begin_on_state_change(&self) -> OnStateChangeGuard {
		OnStateChangeGuard::new(self.clone())
	}

	pub fn on_state_change_started(&self) {
		self.0
			.on_state_change_in_flight
			.fetch_add(1, Ordering::SeqCst);
		self.0.sleep.work.keep_awake.increment();
		self.reset_sleep_timer();
	}

	pub fn on_state_change_finished(&self) {
		let previous = self.0.on_state_change_in_flight.try_update(
			Ordering::SeqCst,
			Ordering::SeqCst,
			|count| count.checked_sub(1),
		);

		match previous {
			Ok(1) => {
				self.0.sleep.work.keep_awake.decrement();
				self.0.on_state_change_idle.notify_waiters();
				self.reset_sleep_timer();
			}
			Ok(_) => {
				self.0.sleep.work.keep_awake.decrement();
				self.reset_sleep_timer();
			}
			Err(_) => {
				tracing::warn!(
					actor_id = %self.actor_id(),
					"on_state_change finished without a matching start"
				);
			}
		}
	}

	pub fn persisted(&self) -> PersistedActor {
		self.0.persisted.read().clone()
	}

	pub fn load_persisted(&self, persisted: PersistedActor) {
		let state = persisted.state.clone();
		*self.0.persisted.write() = persisted;
		*self.0.current_state.write() = state.to_vec();
		self.0.state_dirty.store(false, Ordering::SeqCst);
		self.finish_save_request(self.save_request_revision());
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::InternalReplace);
	}

	pub(crate) fn load_last_pushed_alarm(&self, alarm_ts: Option<i64>) {
		*self.0.last_pushed_alarm.write() = alarm_ts;
	}

	pub(crate) fn last_pushed_alarm(&self) -> Option<i64> {
		*self.0.last_pushed_alarm.read()
	}

	/// Persists an acknowledged alarm push. The in-memory value was already set when the
	/// push was sent, and a newer push may have replaced it since, so this leaves it alone.
	pub(crate) async fn persist_last_pushed_alarm(&self, alarm_ts: Option<i64>) -> Result<()> {
		internal_storage::persist_last_pushed_alarm(self.sql(), alarm_ts)
			.await
			.context("persist last pushed alarm to sqlite")
	}

	pub(crate) fn load_run_wake_at(&self, wake_at: Option<i64>) {
		*self.0.run_wake_at.write() = wake_at;
	}

	pub(crate) fn run_wake_at(&self) -> Option<i64> {
		*self.0.run_wake_at.read()
	}

	pub(crate) async fn persist_run_wake_at(&self, wake_at: Option<i64>) -> Result<()> {
		internal_storage::persist_run_wake_at(self.sql(), wake_at)
			.await
			.context("persist run wake deadline to sqlite")?;
		self.load_run_wake_at(wake_at);
		Ok(())
	}

	pub(crate) fn set_initial_state(&self, state: Vec<u8>) {
		*self.0.current_state.write() = state.clone();
		self.0.persisted.write().state = state.into();
		self.0.state_dirty.store(true, Ordering::SeqCst);
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
	}

	pub fn scheduled_events(&self) -> Vec<PersistedScheduleEvent> {
		self.0.persisted.read().scheduled_events.clone()
	}

	pub fn set_scheduled_events(&self, scheduled_events: Vec<PersistedScheduleEvent>) {
		self.0.persisted.write().scheduled_events = scheduled_events;
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::ScheduledEventsUpdate);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn set_input(&self, input: Option<Vec<u8>>) {
		self.0.persisted.write().input = input.map(Into::into);
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::InputSet);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn input(&self) -> Option<Vec<u8>> {
		self.0.persisted.read().input.clone().map(Into::into)
	}

	pub fn set_has_initialized(&self, has_initialized: bool) {
		{
			let mut persisted = self.0.persisted.write();
			if persisted.has_initialized == has_initialized {
				return;
			}
			persisted.has_initialized = has_initialized;
		}
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::HasInitialized);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn has_initialized(&self) -> bool {
		self.0.persisted.read().has_initialized
	}

	pub fn flush_on_shutdown(&self) {
		self.persist_now_tracked("shutdown_flush");
	}

	pub fn on_request_save(&self, hook: Box<dyn Fn(RequestSaveOpts) + Send + Sync>) {
		self.0.request_save_hooks.write().push(Arc::from(hook));
	}

	fn is_dirty(&self) -> bool {
		self.0.state_dirty.load(Ordering::SeqCst)
	}

	fn mark_dirty(&self) {
		self.0.state_dirty.store(true, Ordering::SeqCst);
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
	}

	fn lifecycle_event_sender(&self) -> Option<mpsc::UnboundedSender<LifecycleEvent>> {
		self.0.lifecycle_events.read().clone()
	}

	fn compute_save_delay(&self, max_wait: Option<Duration>) -> Duration {
		let elapsed = self
			.0
			.last_save_at
			.lock()
			.map(|instant| instant.elapsed())
			.unwrap_or_default();

		throttled_save_delay(self.0.state_save_interval, elapsed, max_wait)
	}

	fn schedule_save(&self, max_wait: Option<Duration>) {
		if !self.is_dirty() {
			return;
		}

		let delay = self.compute_save_delay(max_wait);
		let scheduled_at = StdInstant::now() + delay;

		let mut pending_save = self.0.pending_save.lock();

		if let Some(existing) = pending_save.as_ref() {
			if existing.scheduled_at <= scheduled_at {
				return;
			}

			existing.handle.abort();
		}

		let state = self.clone();
		// Intentionally detached but abortable: pending delayed saves are
		// retained in `pending_save`, replaced by newer saves, and awaited at
		// shutdown through the state save guard.
		let task = async move {
			if !delay.is_zero() {
				sleep(delay).await;
			}

			state.take_pending_save();

			if let Err(error) = state.persist_if_dirty().await {
				tracing::error!(?error, "failed to persist actor state");
			}
		}
		.in_current_span();

		#[cfg(not(feature = "wasm-runtime"))]
		let handle = {
			let Ok(tokio_handle) = Handle::try_current() else {
				return;
			};
			tokio_handle.spawn(task)
		};

		#[cfg(feature = "wasm-runtime")]
		let handle = RuntimeSpawner::spawn(task);

		*pending_save = Some(PendingSave {
			scheduled_at,
			handle,
		});
	}

	fn clear_pending_save(&self) {
		if let Some(pending_save) = self.take_pending_save() {
			pending_save.handle.abort();
		}
	}

	pub(crate) fn persist_now_tracked(&self, description: &'static str) {
		self.clear_pending_save();

		let state = self.clone();
		let mut tracked_persist = self.0.tracked_persist.lock();
		let previous = tracked_persist.take();
		let task = async move {
			if let Some(previous) = previous {
				let _ = previous.await;
			}

			if let Err(error) = state.persist_state(SaveStateOpts { immediate: true }).await {
				tracing::error!(?error, description, "failed to persist actor state");
			}
		}
		.in_current_span();

		#[cfg(not(feature = "wasm-runtime"))]
		let handle = {
			let Ok(tokio_handle) = Handle::try_current() else {
				tracing::warn!(
					description,
					"skipping tracked actor state persistence without runtime"
				);
				return;
			};
			tokio_handle.spawn(task)
		};

		#[cfg(feature = "wasm-runtime")]
		let handle = RuntimeSpawner::spawn(task);

		*tracked_persist = Some(handle);
	}

	fn take_pending_save(&self) -> Option<PendingSave> {
		self.0.pending_save.lock().take()
	}

	fn take_tracked_persist(&self) -> Option<JoinHandle<()>> {
		self.0.tracked_persist.lock().take()
	}

	fn has_tracked_persist(&self) -> bool {
		self.0.tracked_persist.lock().is_some()
	}

	async fn persist_if_dirty(&self) -> Result<()> {
		self.persist_if_dirty_carrying(&mut Vec::new()).await
	}

	/// Writes the persisted actor when it is dirty, running and emptying the statements in
	/// `carried` in the same batch. A clean actor writes nothing and leaves them.
	async fn persist_if_dirty_carrying(
		&self,
		carried: &mut Vec<SqliteBatchStatement>,
	) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		let (revision, actor_to_persist, _write_guard) = {
			let _save_guard = self.0.save_guard.lock().await;
			if !self.is_dirty() {
				return Ok(());
			}

			let revision = self.0.state_revision.load(Ordering::SeqCst);
			let persisted = self.persisted();
			(revision, persisted, self.begin_write())
		};

		internal_storage::persist_actor_snapshot_with(
			self.sql(),
			&actor_to_persist,
			std::mem::take(carried),
		)
		.await
		.context("persist actor state to sqlite")?;

		*self.0.last_save_at.lock() = Some(StdInstant::now());

		if self.0.state_revision.load(Ordering::SeqCst) == revision {
			self.0.state_dirty.store(false, Ordering::SeqCst);
		}

		Ok(())
	}

	/// Runs the startup commit in the background, so the actor's first requests run while it is
	/// in flight; their responses wait for it in `wait_startup_commit`. A failed commit stops
	/// the actor with an error, as a failed startup does. The write counts as in flight from
	/// this call, so a shutdown that follows waits for it.
	pub(crate) fn run_startup_commit(
		&self,
		commit: impl std::future::Future<Output = Result<()>> + Send + 'static,
	) {
		self.0
			.startup_commit
			.send_modify(|startup| startup.pending += 1);
		let write = self.begin_write();
		let ctx = self.clone();
		RuntimeSpawner::spawn(
			async move {
				let result = commit.await;
				drop(write);
				let failed = result.is_err();
				let mut returned = false;
				ctx.0.startup_commit.send_modify(|startup| {
					startup.pending = startup.pending.saturating_sub(1);
					startup.failed |= failed;
					returned = startup.returned;
				});
				if let Err(error) = result {
					tracing::error!(
						?error,
						actor_id = %ctx.actor_id(),
						"actor startup state commit failed; stopping the actor"
					);
					// Before startup returns, its own check fails the start instead.
					if !returned {
						return;
					}
					if let Err(stop_error) = ctx.stop_with_error(format!("{error:#}")) {
						tracing::error!(
							?stop_error,
							actor_id = %ctx.actor_id(),
							"failed to report the startup commit failure to the engine"
						);
					}
				}
			}
			.in_current_span(),
		);
	}

	/// Begins a startup's tracking of its background commits. A restart within this context
	/// starts over: an earlier generation's failure does not fail it.
	pub(crate) fn begin_startup_commits(&self) {
		self.0.startup_commit.send_modify(|startup| {
			startup.returned = false;
			startup.failed = false;
		});
	}

	/// Marks startup as returned, and reports whether a startup commit already failed, in
	/// which case the start must fail. A commit that fails later stops the actor itself.
	pub(crate) fn finish_startup_commits(&self) -> Result<()> {
		let mut failed = false;
		self.0.startup_commit.send_modify(|startup| {
			startup.returned = true;
			failed = startup.failed;
		});
		if failed {
			return Err(ActorLifecycle::Stopping.build())
				.context("actor startup state commit failed");
		}
		Ok(())
	}

	/// Waits until every startup commit of this generation is durable. An error means one
	/// failed and the actor is stopping, so a request served in the meantime must not report
	/// success.
	pub(crate) async fn wait_startup_commit(&self) -> Result<()> {
		let mut startup = self.0.startup_commit.subscribe();
		let failed = startup
			.wait_for(|startup| startup.pending == 0)
			.await
			.map(|startup| startup.failed);
		match failed {
			Ok(false) => Ok(()),
			Ok(true) | Err(_) => {
				Err(ActorLifecycle::Stopping.build()).context("actor startup state commit failed")
			}
		}
	}

	fn begin_write(&self) -> InFlightWrite {
		self.0.in_flight_state_writes.fetch_add(1, Ordering::SeqCst);
		InFlightWrite { ctx: self.clone() }
	}

	async fn wait_for_in_flight_writes(&self) {
		wait_for_notified_condition(&self.0.state_write_completion, || {
			self.0.in_flight_state_writes.load(Ordering::SeqCst) == 0
		})
		.await;
	}

	fn finish_save_request(&self, save_request_revision: u64) {
		if self.0.save_request_revision.load(Ordering::SeqCst) == save_request_revision {
			self.0.save_requested.store(false, Ordering::SeqCst);
			self.0
				.save_requested_immediate
				.store(false, Ordering::SeqCst);
			*self.0.save_requested_within_deadline.lock() = None;
		}
	}

	fn mark_save_request_completed(&self, save_request_revision: u64) {
		self.0
			.save_completed_revision
			.fetch_max(save_request_revision, Ordering::SeqCst);
		self.0.save_completion.notify_waiters();
	}

	fn notify_request_save_hooks(&self, opts: RequestSaveOpts) {
		let hooks = self.0.request_save_hooks.read().clone();
		for hook in hooks {
			hook(opts);
		}
	}
}

struct InFlightWrite {
	ctx: ActorContext,
}

impl Drop for InFlightWrite {
	fn drop(&mut self) {
		if self
			.ctx
			.0
			.in_flight_state_writes
			.fetch_sub(1, Ordering::SeqCst)
			== 1
		{
			self.ctx.0.state_write_completion.notify_waiters();
		}
	}
}

/// Waits until `ready` returns true, re-checking after each wakeup of `notify`.
///
/// Producers must update the state `ready` reads before calling
/// `notify.notify_waiters()`. The notification is enabled before every check,
/// so a producer that runs between the check and the await still wakes this
/// waiter even though `notify_waiters` stores no permit. Every concurrent
/// waiter is woken, not just one.
async fn wait_for_notified_condition(notify: &Notify, mut ready: impl FnMut() -> bool) {
	loop {
		let notified = notify.notified();
		tokio::pin!(notified);
		notified.as_mut().enable();

		if ready() {
			return;
		}

		notified.await;
	}
}

fn throttled_save_delay(
	save_interval: Duration,
	time_since_last_save: Duration,
	max_wait: Option<Duration>,
) -> Duration {
	let save_delay = save_interval.saturating_sub(time_since_last_save);
	if let Some(max_wait) = max_wait {
		save_delay.min(max_wait)
	} else {
		save_delay
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
