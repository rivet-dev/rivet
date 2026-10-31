use crate::{
	DurableStateMachine, Effects, Executor, RuntimeEvent,
	keys::kv::{
		MachineDataKey, MachineStateKey, MachineTimerByIdKey, MachineTimerKey, MachineTimerPrefix,
	},
};
use anyhow::{Result, bail, ensure};
use bytes::Bytes;
use depot_worker::command::KvHandle;
use depot_worker::command::Transaction;
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
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let lock_started = std::time::Instant::now();
		let guard = db.lock_partition(MachineStateKey::new().range()).await?;
		crate::metrics::LOCK_WAIT_SECONDS
			.with_label_values(&[M::DEBUG_NAME])
			.observe(lock_started.elapsed().as_secs_f64());
		let mut metrics = RunnerMetrics::new(M::DEBUG_NAME);

		// Plan pass: the machine and, for an alarm, its due timers are read from a snapshot,
		// then each transition runs in memory with the executor's `prepare` before it. The
		// lock keeps the state from changing meanwhile; no transaction is open, so `prepare`
		// can do I/O without holding one.
		let (mut machine, due_timers) = {
			let mut snapshot = db.snapshot().await?;
			let machine = Machine::<M>::read(&mut snapshot, method).await?;
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
					for (key, _) in entries {
						let key = MachineTimerKey::unpack(&key)?;
						if key.deadline_unix_ms.0 > now.unix_ms {
							break;
						}
						due.push(key);
					}
					due
				}
			};
			(machine, due_timers)
		};
		let armed_head = machine
			.data
			.alarm
			.as_ref()
			.map(|alarm| alarm.deadline_unix_ms);
		let mut plan: Vec<Transition<M>> = Vec::new();

		if !machine.has_started() {
			let now = ctx.env().clock().observe()?;
			let mut effects = Effects::new(now);
			#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
			let started = std::time::Instant::now();
			M::start(
				&mut machine.state,
				now,
				rivet_state_machine::Entropy([0; 32]),
				&mut effects,
			);
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
				let armed = machine
					.data
					.alarm
					.as_ref()
					.is_some_and(|alarm| alarm.arm_id == revision.arm_id);
				ensure!(
					armed,
					"durable alarm delivery is for a revision that is no longer armed"
				);

				let now = ctx.env().clock().observe()?;
				// Timers an earlier transition of this batch armed or cancelled, by identity:
				// the deadline it now has, or none. A due timer whose row that transition
				// replaced or removed is skipped, as it would be if the row were read back.
				let mut overlay: HashMap<Bytes, Option<i64>> = HashMap::new();
				for key in due_timers {
					match overlay.get(&key.timer) {
						Some(Some(deadline_unix_ms))
							if *deadline_unix_ms == key.deadline_unix_ms.0 => {}
						Some(_) => continue,
						None => {}
					}

					let timer = M::deserialize_timer(&key.timer)?;
					let body = RuntimeEvent::Timer(timer);
					let preparation = executor.prepare(&machine.state, &body).await?;
					let mut effects =
						Self::apply(ctx, &mut machine, &mut metrics, body, preparation, now).await;
					let transition = Transition::new(Some(key), &mut effects);
					for effect in &transition.effects {
						match effect {
							RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
								let timer = Bytes::from(M::serialize_timer(timer)?);
								let deadline = deadline_unix_ms(transition.clocks, *at_boot_ms);
								overlay.insert(timer, Some(deadline));
							}
							RuntimeEffect::CancelTimer(timer) => {
								let timer = Bytes::from(M::serialize_timer(timer)?);
								overlay.insert(timer, None);
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
		// commits its acknowledgement.
		let state_bytes = M::serialize_state(&machine.state)?;
		let unchanged = alarm_revision.is_none()
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
			return Ok(machine.state);
		}

		// Write pass: the planned transitions become one transaction, in order. A fired timer's
		// row goes before its transition's own timer writes, so a rearm of the same timer lands
		// after the removal.
		let mut tx = db.begin().await?;
		let mut timers = TimerWrites::new(armed_head);
		let mut executions = Vec::new();
		let mut nondurable = Vec::new();
		for transition in plan {
			if let Some(key) = &transition.fired {
				timers.remove(&mut tx, key)?;
			}
			for write in transition.writes {
				executor.write(write, &mut tx).await?;
			}
			// Timer deadlines are relative to the transition's own clock snapshot.
			let clocks = transition.clocks;
			for effect in transition.effects {
				match effect {
					RuntimeEffect::Execute(effect) => executions.push(effect),
					RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
						let timer = Bytes::from(M::serialize_timer(&timer)?);
						timers
							.arm(&mut tx, timer, deadline_unix_ms(clocks, at_boot_ms))
							.await?;
					}
					RuntimeEffect::CancelTimer(timer) => {
						let timer = Bytes::from(M::serialize_timer(&timer)?);
						timers.cancel(&mut tx, timer).await?;
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
			None if next_deadline != armed_deadline => {
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
		db.commit(tx).await?;
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

		Ok(machine.state)
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
			entropy: rivet_state_machine::Entropy([0; 32]),
		};

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
	fired: Option<MachineTimerKey>,
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
		fired: Option<MachineTimerKey>,
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

	async fn arm(
		&mut self,
		tx: &mut Transaction,
		timer: Bytes,
		deadline_unix_ms: i64,
	) -> Result<()> {
		self.cancel(tx, timer.clone()).await?;

		let key = MachineTimerKey::new(OrderedI64(deadline_unix_ms), timer.clone());
		tx.put(&key.pack(), &key.serialize(())?)?;

		let by_id = MachineTimerByIdKey::new(timer);
		tx.put(
			&by_id.pack(),
			&by_id.serialize(wire::TimerEntry { deadline_unix_ms })?,
		)?;

		self.new_armed_head = Some(
			self.new_armed_head
				.map_or(deadline_unix_ms, |earliest| earliest.min(deadline_unix_ms)),
		);

		Ok(())
	}

	async fn cancel(&mut self, tx: &mut Transaction, timer: Bytes) -> Result<()> {
		let by_id = MachineTimerByIdKey::new(timer.clone());

		let Some(entry) = tx.get(&by_id.pack()).await? else {
			return Ok(());
		};

		let entry = by_id.deserialize(entry.into())?;

		self.remove(
			tx,
			&MachineTimerKey::new(OrderedI64(entry.deadline_unix_ms), timer),
		)
	}

	fn remove(&mut self, tx: &mut Transaction, key: &MachineTimerKey) -> Result<()> {
		tx.delete(&key.pack())?;
		tx.delete(&MachineTimerByIdKey::new(key.timer.clone()).pack())?;

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

struct Machine<M: DurableStateMachine> {
	state: M::State,
	/// The state's bytes as read, or `None` for a machine `GetOrCreate` supplied.
	stored_state: Option<Vec<u8>>,
	data: wire::Data,
	data_changed: bool,
}

impl<M: DurableStateMachine> Machine<M> {
	/// Reads the machine, or builds the one `GetOrCreate` supplies when none is stored. Nothing
	/// is written: the state is saved by the transition's transaction either way.
	async fn read(tx: &mut Transaction, method: Method<M>) -> Result<Self> {
		let state_key_packed = MachineStateKey::new().pack();
		let data_key = MachineDataKey::new();
		let data_key_packed = data_key.pack();

		let state = tx.get(&state_key_packed).await?;
		let data = tx.get(&data_key_packed).await?;

		let machine = if let Some(state) = state {
			Some(Machine {
				state: M::deserialize_state(&state)?,
				stored_state: Some(state.to_vec()),
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
			(Method::Get, None) => bail!("machine not found"),
			(Method::GetOrCreate { state }, None) => Ok(Machine {
				state,
				stored_state: None,
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


