use crate::{
	DurableStateMachine, Effects, Executor, RuntimeEvent,
	keys::kv::{
		LegacyMachineTimerByIdKey, LegacyMachineTimerKey, LegacyMachineTimerPrefix, MachineDataKey,
		MachineStateKey, MachineTimerByIdKey, MachineTimerKey, MachineTimerPrefix,
	},
};
use anyhow::{Context, Result, bail};
use bytes::Bytes;
use depot_worker::command::KvHandle;
use depot_worker::command::{CommitError, Transaction};
use rivet_data::generated::depot_durable_state_machine_v1 as wire;
use rivet_data::generated::shared_depot_alarms_v1::{AlarmRevision, AlarmSet};
use rivet_engine_context::EngineContext;
use rivet_env::Clocks;
use rivet_keys::keys::KeyAddress;
use rivet_keys::keys::OrderedI64;
use rivet_keys::keys::SubspaceRange;
use rivet_keys::keys::codec::FormalKey;
use rivet_state_machine::{Effect, Event, EventBody, RunnerMetrics, RunnerPhase, RuntimeEffect};
use std::collections::HashMap;
use std::ops::{Bound, Range};
use std::{marker::PhantomData, sync::Arc};

pub struct Runner<M: DurableStateMachine>(PhantomData<M>);

enum BackendEvent<M: DurableStateMachine> {
	Event(M::Event),
	Alarm(AlarmRevision),
}

enum Attempt<T> {
	Committed(T),
	Replan,
}

impl<M: DurableStateMachine> Runner<M> {
	pub async fn process_event(
		ctx: &EngineContext,
		db: KvHandle,
		method: Method<M>,
		executor: Arc<dyn Executor<M>>,
		event: M::Event,
	) -> Result<M::State> {
		Self::process(ctx, db, method, executor, BackendEvent::Event(event)).await
	}

	pub async fn process_alarm(
		ctx: &EngineContext,
		db: KvHandle,
		executor: Arc<dyn Executor<M>>,
		revision: AlarmRevision,
	) -> Result<M::State> {
		Self::process(
			ctx,
			db,
			Method::Get,
			executor,
			BackendEvent::Alarm(revision),
		)
		.await
	}

	async fn process(
		ctx: &EngineContext,
		db: KvHandle,
		method: Method<M>,
		executor: Arc<dyn Executor<M>>,
		event: BackendEvent<M>,
	) -> Result<M::State> {
		loop {
			let method = match &method {
				Method::Get => Method::Get,
				Method::GetOrCreate { state } => Method::GetOrCreate {
					state: state.clone(),
				},
			};
			let event = match &event {
				BackendEvent::Event(event) => BackendEvent::Event(event.clone()),
				BackendEvent::Alarm(revision) => BackendEvent::Alarm(revision.clone()),
			};
			match Self::process_attempt(ctx, db.clone(), method, executor.clone(), event).await? {
				// Only an unadmitted conflict is safe to replan. A timed-out commit may still
				// land and must retain its unknown outcome. KvHandle keeps the command's
				// original deadline and authority across these attempts.
				Attempt::Replan => {}
				Attempt::Committed(state) => return Ok(state),
			}
		}
	}

	async fn process_attempt(
		ctx: &EngineContext,
		db: KvHandle,
		method: Method<M>,
		executor: Arc<dyn Executor<M>>,
		event: BackendEvent<M>,
	) -> Result<Attempt<M::State>> {
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let lock_started = std::time::Instant::now();
		let guard = db.lock_partition(MachineStateKey::new().range()).await?;
		crate::metrics::LOCK_WAIT_SECONDS
			.with_label_values(&[M::DEBUG_NAME])
			.observe(lock_started.elapsed().as_secs_f64());
		let mut metrics = RunnerMetrics::new(M::DEBUG_NAME);

		// Plan pass: the machine, legacy timers and due timers are read from a snapshot,
		// then each transition runs in memory with the executor's `prepare` before it. The
		// lock excludes other handlers, but an earlier commit whose outcome was unknown
		// may still land. Validate this snapshot again in the write transaction. No pin is
		// retained across `prepare`, so its I/O does not retain a filesystem snapshot.
		let (mut machine, legacy_timers, due_timers) = {
			let mut snapshot = db.snapshot().await?;
			let machine = Machine::<M>::read(&mut snapshot, method).await?;
			let legacy_timers = read_legacy_timers::<M>(&mut snapshot).await?;
			let due_timers = match &event {
				BackendEvent::Event(_) => Vec::new(),
				BackendEvent::Alarm(_) => {
					let now = ctx.env().clock().observe()?;
					let max_events = ctx.config().depot().durable_max_events_per_commit() as usize;
					let range = MachineTimerPrefix::new().range();
					let entries = snapshot
						.scan(scan_bounds(&range), Some(max_events), false)
						.await?;
					let mut due = Vec::new();
					for (key, payload) in entries {
						let key = MachineTimerKey::unpack(&key)?;
						if key.deadline_unix_ms.0 > now.unix_ms {
							break;
						}
						due.push(DueTimer {
							key,
							payload: Bytes::from(payload),
						});
					}

					// Due legacy timers fire with the rest, in deadline order. They move to the
					// id-keyed rows in this batch's commit before any transition's writes.
					let due_legacy = legacy_timers
						.iter()
						.filter(|legacy| legacy.due.key.deadline_unix_ms.0 <= now.unix_ms)
						.map(|legacy| legacy.due.clone());
					due.extend(due_legacy);
					due.sort_by_key(|timer| timer.key.deadline_unix_ms.0);
					due.truncate(max_events);
					due
				}
			};
			(machine, legacy_timers, due_timers)
		};
		// A delivery for a revision the machine no longer has armed raced a transition that
		// re-armed or cleared the alarm in the same commit as the Depot alarm row, so the current
		// revision, if any, has its own delivery. The stale one is acknowledged as done without
		// firing anything; refusing it would only log an internal error and retry it.
		if let BackendEvent::Alarm(revision) = &event
			&& !machine
				.data
				.alarm
				.as_ref()
				.is_some_and(|alarm| alarm.arm_id == revision.arm_id)
		{
			tracing::debug!(
				machine = M::DEBUG_NAME,
				"durable alarm delivery is for a revision that is no longer armed; acknowledged as stale"
			);
			crate::metrics::STALE_ALARM_DELIVERIES_TOTAL
				.with_label_values(&[M::DEBUG_NAME])
				.inc();
			drop(guard);
			return Ok(Attempt::Committed(machine.state));
		}

		let armed_head = machine
			.data
			.alarm
			.as_ref()
			.map(|alarm| alarm.deadline_unix_ms);
		// A volume made by a group fork inherits its parent's machine data and Depot alarm row,
		// but its lazy fork indexed no alarm, so nothing fires until this volume arms it again.
		// That happens in the first commit of this incarnation: the one that resets the
		// inherited state for it. Its deadline can equal the inherited one, so the comparison
		// below alone would skip it.
		let rearm_inherited_alarm = machine.stored_state.is_some()
			&& db
				.volume_record()
				.pending_fork_reset(M::fork_reset(&machine.state))
				.is_some();
		// Selected once per batch: the version every live storage worker reads. The state,
		// the writes and the timers are encoded at it, or above it behind a tablet version
		// mark when it cannot express them; see the write pass.
		let selected = db.write_version(M::FAMILY)?;
		let mut plan: Vec<Transition<M>> = Vec::new();

		if !machine.has_started() {
			let now = ctx.env().clock().observe()?;
			let entropy = rivet_state_machine::Entropy(
				ctx.env().rng("durable.state-machine.entropy").fork_seed(),
			);
			tracing::debug!(target: "rivet_state_machine", sm = M::DEBUG_NAME,
				?now, ?entropy, "durable state machine startup");
			let mut effects = Effects::new(now);
			#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
			let started = std::time::Instant::now();
			M::start(&mut machine.state, now, entropy, &mut effects);
			metrics.observe_transition("start", started.elapsed().as_secs_f64());

			machine.debug_validate(ctx).await;
			machine.data.started_ms = Some(now.unix_ms);
			machine.data_changed = true;
			plan.push(Transition::new(None, &mut effects));
		}

		let alarm_revision = match event {
			BackendEvent::Event(event) => {
				let body = RuntimeEvent::Input(event);
				let preparation = executor.prepare(&machine.state, &body).await?;
				let now = ctx.env().clock().observe()?;
				let mut effects =
					Self::apply(ctx, &mut machine, &mut metrics, body, preparation, now).await;
				plan.push(Transition::new(None, &mut effects));

				None
			}
			BackendEvent::Alarm(revision) => {
				// Timers an earlier transition of this batch armed or cancelled, by identity:
				// the deadline it now has, or none. A due timer whose row that transition
				// replaced or removed is skipped, as it would be if the row were read back.
				let mut overlay: HashMap<Bytes, Option<i64>> = HashMap::new();
				for due in due_timers {
					match overlay.get(&due.key.timer_id) {
						Some(Some(deadline_unix_ms))
							if *deadline_unix_ms == due.key.deadline_unix_ms.0 => {}
						Some(_) => continue,
						None => {}
					}

					let timer = M::deserialize_timer(&due.payload)?;
					let body = RuntimeEvent::Timer(timer);
					let preparation = executor.prepare(&machine.state, &body).await?;
					let now = ctx.env().clock().observe()?;
					let mut effects =
						Self::apply(ctx, &mut machine, &mut metrics, body, preparation, now).await;
					let transition = Transition::new(Some(due), &mut effects);
					for effect in &transition.effects {
						match effect {
							RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
								let timer_id = Bytes::from(M::timer_id(timer));
								let deadline = deadline_unix_ms(transition.clocks, *at_boot_ms);
								overlay.insert(timer_id, Some(deadline));
							}
							RuntimeEffect::CancelTimer(timer) => {
								let timer_id = Bytes::from(M::timer_id(timer));
								overlay.insert(timer_id, None);
							}
							RuntimeEffect::Execute(_) | RuntimeEffect::Exit => {}
						}
					}
					plan.push(transition);
				}

				Some(revision)
			}
		};

		// A batch that leaves the stored state byte-identical and asks for no write, timer or
		// durable effect has nothing to make durable, so its commit is skipped and only its
		// nondurable effects run. The state it read is already durable: a snapshot's view stops
		// at the quorum-committed position. The two views that serve entries before their
		// quorum, a speculative creating capture and a volume opened empty, hold no stored
		// state, so `stored_state` is `None` there and the batch commits. An alarm always
		// commits its acknowledgement, and legacy timers always commit their move.
		//
		// The comparison is by bytes at this batch's version, so the first batch on a state
		// stored at another version rewrites it once: one extra durable write on that actor's
		// first wake after the upgrade.
		let (state_bytes, state_version) = state_encoding::<M>(&machine.state, selected)?;
		let unchanged = alarm_revision.is_none()
			&& legacy_timers.is_empty()
			&& !rearm_inherited_alarm
			&& !machine.data_changed
			&& machine.stored_state.as_deref() == Some(state_bytes.as_slice())
			&& plan.iter().all(Transition::is_read_only);
		if unchanged {
			crate::metrics::SKIPPED_COMMITS_TOTAL
				.with_label_values(&[M::DEBUG_NAME])
				.inc();
			let nondurable = plan
				.into_iter()
				.flat_map(|transition| transition.nondurable);
			for effect in nondurable {
				let effect_ctx = metrics.dispatch(effect.name(), (), ctx.env().clone());
				executor.execute(effect, effect_ctx);
			}
			drop(guard);
			return Ok(Attempt::Committed(machine.state));
		}

		// A batch storing the state, a write or an armed timer above the selected version,
		// because the selected version cannot express it, first makes the tablet's mark of
		// the family durable at that version, so no worker that cannot read it is granted
		// the tablet. Writes follow the state's version; a cancel writes nothing it encodes.
		let mut stored_version = state_version;
		for transition in &plan {
			for effect in &transition.effects {
				if let RuntimeEffect::ArmTimer { timer, .. } = effect {
					let (_, timer_version) = timer_encoding::<M>(timer, selected)?;
					stored_version = stored_version.max(timer_version);
				}
			}
		}
		if stored_version > selected {
			db.ensure_version_mark(M::FAMILY, stored_version).await?;
		}

		// Write pass: the planned transitions become one transaction, in order. A fired timer's
		// row goes before its transition's own timer writes, so a rearm of the same timer lands
		// after the removal.
		let mut tx = db.begin().await?;
		if !machine.validate_plan(&mut tx)? {
			return Ok(Attempt::Replan);
		}
		let mut timers = TimerWrites::new(armed_head);
		// Legacy timers move first, at their own deadlines, so this batch's arms, cancels and
		// fired removals find them in the id-keyed rows.
		for legacy in &legacy_timers {
			timers.move_legacy(&mut tx, legacy).await?;
		}
		let mut executions = Vec::new();
		let mut nondurable = Vec::new();
		for transition in plan {
			if let Some(fired) = &transition.fired {
				timers.remove(&mut tx, &fired.key)?;
			}
			for write in transition.writes {
				executor.write(write, &mut tx, state_version).await?;
			}
			// Timer deadlines are relative to the transition's own clock snapshot.
			let clocks = transition.clocks;
			for effect in transition.effects {
				match effect {
					RuntimeEffect::Execute(effect) => executions.push(effect),
					RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
						let timer_id = Bytes::from(M::timer_id(&timer));
						// A rearm of the timer that just fired keeps at least the version its fired
						// payload was stored at. That payload is already in this tablet, so the
						// tablet's version mark covers its version.
						let (payload, payload_version) = timer_encoding::<M>(&timer, selected)?;
						let payload = match &transition.fired {
							Some(fired) if fired.key.timer_id == timer_id => {
								M::serialize_rearmed_timer(&timer, payload_version, &fired.payload)?
							}
							_ => payload,
						};
						timers
							.arm(
								&mut tx,
								timer_id,
								Bytes::from(payload),
								deadline_unix_ms(clocks, at_boot_ms),
							)
							.await?;
					}
					RuntimeEffect::CancelTimer(timer) => {
						let timer_id = Bytes::from(M::timer_id(&timer));
						timers.cancel(&mut tx, timer_id).await?;
					}
					RuntimeEffect::Exit => bail!("durable state machines cannot exit"),
				}
			}
			nondurable.extend(transition.nondurable);
		}

		// Point the depot alarm at the earliest remaining timer in the same commit
		let next_deadline = timers.head(&mut tx).await?;
		let armed_deadline = machine
			.data
			.alarm
			.as_ref()
			.map(|alarm| alarm.deadline_unix_ms);

		match alarm_revision {
			Some(revision) => {
				let next = next_deadline.map(|deadline_unix_ms| wire::Alarm {
					arm_id: *ctx.env().rng("durable.alarm.arm").uuid_v4().as_bytes(),
					deadline_unix_ms,
				});

				let next_set = next.as_ref().map(|alarm| AlarmSet {
					arm_id: alarm.arm_id,
					deadline_unix_ms: alarm.deadline_unix_ms,
				});
				tx.ack_alarm(revision, next_set).await?;

				machine.data.alarm = next;
				machine.data_changed = true;
			}
			// The first command of a lazily forked incarnation arms its inherited alarm here,
			// never a read or the fork itself. This is intentional: it keeps a fork O(1), a
			// fork cannot wake every inherited alarm at once and overload the child's pools,
			// and reading an actor does not start it. Two later options are not built: the
			// parent fires its children's alarms, or a colocated import places volumes by root
			// group and copies alarm entries on `GROUP_FORKED`.
			None if rearm_inherited_alarm || next_deadline != armed_deadline => {
				let next = next_deadline.map(|deadline_unix_ms| wire::Alarm {
					arm_id: *ctx.env().rng("durable.alarm.arm").uuid_v4().as_bytes(),
					deadline_unix_ms,
				});

				match &next {
					Some(alarm) => tx.set_alarm(alarm.deadline_unix_ms, alarm.arm_id)?,
					None => tx.clear_alarm()?,
				}

				machine.data.alarm = next;
				machine.data_changed = true;
			}
			None => {}
		}

		machine.save(&mut tx, &state_bytes)?;

		// Nondurable effects run before the commit, so they cannot wait on it.
		for effect in nondurable {
			let effect_ctx = metrics.dispatch(effect.name(), (), ctx.env().clone());
			executor.execute(effect, effect_ctx);
		}

		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let commit_started = std::time::Instant::now();
		match db.commit(tx).await {
			Ok(_) => {}
			Err(CommitError::Conflict) => return Ok(Attempt::Replan),
			Err(error) => return Err(error.into()),
		}
		crate::metrics::COMMIT_SECONDS
			.with_label_values(&[M::DEBUG_NAME])
			.observe(commit_started.elapsed().as_secs_f64());

		drop(guard);

		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		for effect in executions {
			let effect_ctx = metrics.dispatch(effect.name(), (), ctx.env().clone());
			executor.execute(effect, effect_ctx);
		}
		metrics.observe_phase(RunnerPhase::Dispatch, started.elapsed().as_secs_f64());

		Ok(Attempt::Committed(machine.state))
	}

	/// Runs one transition in memory and returns its effects.
	async fn apply(
		ctx: &EngineContext,
		machine: &mut Machine<M>,
		metrics: &mut RunnerMetrics,
		body: RuntimeEvent<M::Event, M::Timer>,
		preparation: M::Preparation,
		now: Clocks,
	) -> Effects<M::Effect, M::Timer, M::WriteEffect> {
		let event_name = body.name();
		let event = Event {
			now,
			body,
			entropy: rivet_state_machine::Entropy(
				ctx.env().rng("durable.state-machine.entropy").fork_seed(),
			),
		};
		tracing::debug!(target: "rivet_state_machine", sm = M::DEBUG_NAME,
			event = event_name, ?now, entropy = ?event.entropy, "durable state machine delivery");

		let mut effects = Effects::new(now);
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		M::on_event(&mut machine.state, event, preparation, &mut effects);
		metrics.observe_transition(event_name, started.elapsed().as_secs_f64());

		machine.debug_validate(ctx).await;
		effects
	}
}

/// One planned transition: the timer it fired for, if any, and the effects it emitted, with
/// the clock snapshot its timer deadlines are relative to.
struct Transition<M: DurableStateMachine> {
	fired: Option<DueTimer>,
	clocks: Clocks,
	effects: Vec<RuntimeEffect<M::Effect, M::Timer>>,
	nondurable: Vec<M::Effect>,
	writes: Vec<M::WriteEffect>,
}

impl<M: DurableStateMachine> Transition<M> {
	/// Whether the transition asked for nothing that a commit would carry: no fired timer
	/// row, no write, no timer change and no durable effect.
	fn is_read_only(&self) -> bool {
		self.fired.is_none() && self.writes.is_empty() && self.effects.is_empty()
	}

	fn new(
		fired: Option<DueTimer>,
		effects: &mut Effects<M::Effect, M::Timer, M::WriteEffect>,
	) -> Self {
		Self {
			fired,
			clocks: effects.clocks(),
			effects: effects.drain(),
			nondurable: effects.drain_nondurable(),
			writes: effects.drain_writes(),
		}
	}
}

/// The timer payload's encoding at the lowest version of the machine's family it can be stored
/// at: `selected`, or the lowest newer one that expresses it when `selected` cannot. The row
/// that holds it is named by `timer_id`, which no version change moves. Only a version's
/// [`Inexpressible`](rivet_data::versioned::Inexpressible) refusal moves on to the next
/// version: any other encoding error fails the batch, since skipping it would hide a converter
/// bug behind a version mark.
fn timer_encoding<M: DurableStateMachine>(
	timer: &M::Timer,
	selected: u16,
) -> Result<(Vec<u8>, u16)> {
	let max_write = M::FAMILY
		.info()
		.map(|info| info.max_write)
		.with_context(|| format!("this binary does not know family {}", M::FAMILY))?;
	let mut refusal = None;
	for version in selected..=max_write {
		match M::serialize_timer(timer, version) {
			Ok(bytes) => return Ok((bytes, version)),
			Err(error) if error.is::<rivet_data::versioned::Inexpressible>() => {
				refusal = Some(error)
			}
			Err(error) => {
				return Err(error.context(format!(
					"failed to encode a timer of family {} at version {version}",
					M::FAMILY
				)));
			}
		}
	}
	Err(refusal.unwrap_or_else(|| {
		anyhow::anyhow!(
			"no version of family {} from {selected} encodes the timer",
			M::FAMILY
		)
	}))
}

/// The state's encoding at the lowest version of the machine's family it can be stored at:
/// `selected`, the version the storage read floor allows, or the lowest newer one that
/// expresses it when `selected` cannot, such as actor state under a floor of version 1. Only a
/// version's [`Inexpressible`](rivet_data::versioned::Inexpressible) refusal moves on to the
/// next version: any other encoding error fails the batch, since skipping it would hide a
/// converter bug behind a version mark and the bounded cold flush the mark costs.
fn state_encoding<M: DurableStateMachine>(
	state: &M::State,
	selected: u16,
) -> Result<(Vec<u8>, u16)> {
	let max_write = M::FAMILY
		.info()
		.map(|info| info.max_write)
		.with_context(|| format!("this binary does not know family {}", M::FAMILY))?;
	let mut refusal = None;
	for version in selected..=max_write {
		match M::serialize_state(state, version) {
			Ok(bytes) => return Ok((bytes, version)),
			Err(error) if error.is::<rivet_data::versioned::Inexpressible>() => {
				refusal = Some(error)
			}
			Err(error) => {
				return Err(error.context(format!(
					"failed to encode the state of family {} at version {version}",
					M::FAMILY
				)));
			}
		}
	}
	Err(refusal.unwrap_or_else(|| {
		anyhow::anyhow!(
			"no version of family {} from {selected} encodes the state",
			M::FAMILY
		)
	}))
}

/// Converts a boot clock deadline to unix time, keeping the delay from the transition's clock
/// snapshot. Boot deadlines mean nothing after a restart, and the depot alarm is in unix time.
fn deadline_unix_ms(clocks: Clocks, at_boot_ms: u64) -> i64 {
	let at_boot_ms = i64::try_from(at_boot_ms).unwrap_or(i64::MAX);
	let boot_ms = i64::try_from(clocks.boot_ms).unwrap_or(i64::MAX);
	let delay_ms = at_boot_ms.saturating_sub(boot_ms);
	clocks.unix_ms.saturating_add(delay_ms)
}

struct TimerWrites {
	/// The earliest deadline at the start of the transaction.
	durable_armed_head: Option<i64>,
	/// The earliest deadline this transaction armed.
	new_armed_head: Option<i64>,
	head_removed: bool,
}

impl TimerWrites {
	fn new(durable_armed_head: Option<i64>) -> Self {
		Self {
			durable_armed_head,
			new_armed_head: None,
			head_removed: false,
		}
	}

	/// Arms the timer `timer_id` at `deadline_unix_ms` with `payload`, replacing the armed timer
	/// with the same id.
	async fn arm(
		&mut self,
		tx: &mut Transaction,
		timer_id: Bytes,
		payload: Bytes,
		deadline_unix_ms: i64,
	) -> Result<()> {
		self.cancel(tx, timer_id.clone()).await?;
		self.put(tx, timer_id, payload, deadline_unix_ms)?;

		self.new_armed_head = Some(
			self.new_armed_head
				.map_or(deadline_unix_ms, |earliest| earliest.min(deadline_unix_ms)),
		);

		Ok(())
	}

	/// Moves a legacy timer to the id-keyed rows at its own deadline. The deadline is already
	/// counted in the durable head, so the head does not move. An armed timer with the same id
	/// is replaced, so the move is idempotent.
	async fn move_legacy(&mut self, tx: &mut Transaction, legacy: &LegacyTimer) -> Result<()> {
		tx.delete(&legacy.key.pack())?;
		tx.delete(&LegacyMachineTimerByIdKey::new(legacy.key.timer.clone()).pack())?;

		let timer_id = legacy.due.key.timer_id.clone();
		self.cancel(tx, timer_id.clone()).await?;
		self.put(
			tx,
			timer_id,
			legacy.due.payload.clone(),
			legacy.due.key.deadline_unix_ms.0,
		)
	}

	fn put(
		&mut self,
		tx: &mut Transaction,
		timer_id: Bytes,
		payload: Bytes,
		deadline_unix_ms: i64,
	) -> Result<()> {
		let key = MachineTimerKey::new(OrderedI64(deadline_unix_ms), timer_id.clone());
		tx.put(&key.pack(), &key.serialize(payload)?)?;

		let entry = MachineTimerByIdKey::new(timer_id);
		tx.put(
			&entry.pack(),
			&entry.serialize(wire::TimerEntry { deadline_unix_ms })?,
		)?;

		Ok(())
	}

	async fn cancel(&mut self, tx: &mut Transaction, timer_id: Bytes) -> Result<()> {
		let entry_key = MachineTimerByIdKey::new(timer_id.clone());

		let Some(entry) = tx.get(&entry_key.pack()).await? else {
			return Ok(());
		};

		let entry = entry_key.deserialize(entry.into())?;

		self.remove(
			tx,
			&MachineTimerKey::new(OrderedI64(entry.deadline_unix_ms), timer_id),
		)
	}

	fn remove(&mut self, tx: &mut Transaction, key: &MachineTimerKey) -> Result<()> {
		tx.delete(&key.pack())?;
		tx.delete(&MachineTimerByIdKey::new(key.timer_id.clone()).pack())?;

		let deadline_unix_ms = key.deadline_unix_ms.0;
		if self
			.durable_armed_head
			.is_none_or(|head| deadline_unix_ms <= head)
		{
			self.head_removed = true;
		}
		Ok(())
	}

	async fn head(&self, tx: &mut Transaction) -> Result<Option<i64>> {
		if self.head_removed {
			let range = MachineTimerPrefix::new().range();
			let first = tx.scan(scan_bounds(&range), Some(1), false).await?;
			return first
				.into_iter()
				.next()
				.map(|(key, _)| Ok(MachineTimerKey::unpack(&key)?.deadline_unix_ms.0))
				.transpose();
		}

		Ok(match (self.durable_armed_head, self.new_armed_head) {
			(Some(durable), Some(new)) => Some(durable.min(new)),
			(durable, new) => durable.or(new),
		})
	}
}

/// A timer due to fire: its row and the payload stored in it.
#[derive(Clone)]
struct DueTimer {
	key: MachineTimerKey,
	payload: Bytes,
}

/// A timer stored before timers had stable ids: its legacy row, and the id-keyed row it moves
/// to. Its legacy row named it by its serialized bytes, which become its payload.
struct LegacyTimer {
	key: LegacyMachineTimerKey,
	due: DueTimer,
}

/// Reads every legacy timer of the machine. A machine holds a few timers at most, and the
/// first commit after the read moves them all, so later reads find none.
async fn read_legacy_timers<M: DurableStateMachine>(
	tx: &mut Transaction,
) -> Result<Vec<LegacyTimer>> {
	let range = LegacyMachineTimerPrefix::new().range();
	let entries = tx.scan(scan_bounds(&range), None, false).await?;

	let mut legacy_timers = Vec::with_capacity(entries.len());
	for (key, _) in entries {
		let key = LegacyMachineTimerKey::unpack(&key)?;
		let timer = M::deserialize_timer(&key.timer)?;
		let timer_id = Bytes::from(M::timer_id(&timer));
		let due = DueTimer {
			key: MachineTimerKey::new(OrderedI64(key.deadline_unix_ms.0), timer_id),
			payload: key.timer.clone(),
		};
		legacy_timers.push(LegacyTimer { key, due });
	}

	Ok(legacy_timers)
}

struct Machine<M: DurableStateMachine> {
	state: M::State,
	/// The state's bytes as read, or `None` for a machine `GetOrCreate` supplied.
	stored_state: Option<Vec<u8>>,
	/// The filesystem snapshot from which the state, metadata and due timers were planned.
	read_version: u64,
	data: wire::Data,
	data_changed: bool,
}

impl<M: DurableStateMachine> Machine<M> {
	/// Reads the machine, or builds the one `GetOrCreate` supplies when none is stored. Nothing
	/// is written: the state is saved by the transition's transaction either way.
	async fn read(tx: &mut Transaction, method: Method<M>) -> Result<Self> {
		let read_version = tx.read_version();
		let state_key_packed = MachineStateKey::new().pack();
		let data_key = MachineDataKey::new();
		let data_key_packed = data_key.pack();

		let state = tx.get(&state_key_packed).await?;
		let data = tx.get(&data_key_packed).await?;

		let machine = if let Some(state) = state {
			Some(Machine {
				state: M::deserialize_state(&state)?,
				stored_state: Some(state.to_vec()),
				read_version,
				data: data
					.map(|v| data_key.deserialize(v.into()))
					.transpose()?
					.unwrap_or(wire::Data {
						started_ms: None,
						alarm: None,
					}),
				data_changed: false,
			})
		} else {
			None
		};

		match (method, machine) {
			(_, Some(machine)) => Ok(machine),
			(Method::Get, None) => Err(crate::errors::MachineNotFound.build()),
			(Method::GetOrCreate { state }, None) => Ok(Machine {
				state,
				stored_state: None,
				read_version,
				data: wire::Data {
					started_ms: None,
					alarm: None,
				},
				data_changed: false,
			}),
		}
	}

	fn has_started(&self) -> bool {
		self.data.started_ms.is_some()
	}

	/// Reject any commit between planning and opening this transaction, even a timer change
	/// that leaves state and metadata byte-identical. Every runner commit writes the state
	/// key, so its read conflict also catches a commit that lands after this transaction opens,
	/// including while commit waits for an uncertain write. No snapshot pin crosses prepare.
	fn validate_plan(&self, tx: &mut Transaction) -> Result<bool> {
		if tx.read_version() != self.read_version {
			return Ok(false);
		}
		let key = MachineStateKey::new().pack();
		tx.add_read_conflict_range((Bound::Included(&key[..]), Bound::Included(&key[..])))?;
		Ok(true)
	}

	fn save(&self, tx: &mut Transaction, state_bytes: &[u8]) -> Result<()> {
		let state_key_packed = MachineStateKey::new().pack();
		tx.put(&state_key_packed, state_bytes)?;

		if self.data_changed {
			let data_key = MachineDataKey::new();
			let data_key_packed = data_key.pack();
			tx.put(&data_key_packed, &data_key.serialize(self.data.clone())?)?;
		}

		Ok(())
	}

	fn debug_validate(&self, ctx: &EngineContext) -> impl Future<Output = ()> + Send + 'static {
		#[cfg(debug_assertions)]
		let invalid = M::debug_validate(&self.state).err();
		#[cfg(not(debug_assertions))]
		let invalid: Option<anyhow::Error> = None;
		let env = ctx.env().clone();

		async move {
			if let Some(error) = invalid {
				tracing::error!(machine = M::DEBUG_NAME, %error, "invalid state machine state");
				// EX_SOFTWARE (70): a state machine invariant failed.
				match env.exit(70).await {}
			}
		}
	}
}

pub enum Method<M: DurableStateMachine> {
	Get,
	GetOrCreate { state: M::State },
}

fn scan_bounds(range: &Range<Vec<u8>>) -> (Bound<&[u8]>, Bound<&[u8]>) {
	(
		Bound::Included(range.start.as_slice()),
		Bound::Excluded(range.end.as_slice()),
	)
}
