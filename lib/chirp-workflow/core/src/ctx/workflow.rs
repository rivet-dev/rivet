use std::{sync::Arc, time::Instant};

use global_error::{GlobalError, GlobalResult};
use serde::{de::DeserializeOwned, Serialize};
use tokio::time::Duration;
use uuid::Uuid;

use crate::{
	activity::{Activity, ActivityInput},
	builder::workflow as builder,
	ctx::{
		common::{RETRY_TIMEOUT_MS, SUB_WORKFLOW_RETRY},
		ActivityCtx, ListenCtx, MessageCtx, VersionedWorkflowCtx,
	},
	db::{DatabaseHandle, PulledWorkflow},
	error::{WorkflowError, WorkflowResult},
	executable::{AsyncResult, Executable},
	history::{
		cursor::{Cursor, HistoryResult},
		event::{EventId, SleepState},
		location::Location,
		removed::Removed,
		History,
	},
	listen::{CustomListener, Listen},
	message::Message,
	metrics,
	registry::RegistryHandle,
	signal::Signal,
	utils::{
		time::{DurationToMillis, TsToMillis},
		GlobalErrorExt,
	},
	worker,
	workflow::{Workflow, WorkflowInput},
};

/// Poll interval when polling for signals in-process
const SIGNAL_RETRY: Duration = Duration::from_millis(100);
/// Most in-process signal poll tries
const MAX_SIGNAL_RETRIES: usize = 16;
/// Most in-process sub workflow poll tries
const MAX_SUB_WORKFLOW_RETRIES: usize = 4;
/// Retry interval for failed db actions
const DB_ACTION_RETRY: Duration = Duration::from_millis(150);
/// Most db action retries
const MAX_DB_ACTION_RETRIES: usize = 5;

// NOTE: Clonable because of inner arcs
#[derive(Clone)]
pub struct WorkflowCtx {
	workflow_id: Uuid,
	/// Name of the workflow to run in the registry.
	name: String,
	create_ts: i64,
	ts: i64,
	ray_id: Uuid,
	version: usize,

	registry: RegistryHandle,
	db: DatabaseHandle,

	conn: rivet_connection::Connection,

	/// Input data passed to this workflow.
	input: Arc<Box<serde_json::value::RawValue>>,
	/// All events that have ever been recorded on this workflow.
	event_history: History,
	cursor: Cursor,

	/// If this context is currently in a loop, this is the location of the where the loop started.
	loop_location: Option<Location>,

	msg_ctx: MessageCtx,
}

impl WorkflowCtx {
	pub async fn new(
		registry: RegistryHandle,
		db: DatabaseHandle,
		conn: rivet_connection::Connection,
		workflow: PulledWorkflow,
	) -> GlobalResult<Self> {
		let msg_ctx = MessageCtx::new(&conn, workflow.ray_id).await?;
		let event_history = Arc::new(workflow.events);

		Ok(WorkflowCtx {
			workflow_id: workflow.workflow_id,
			name: workflow.workflow_name,
			create_ts: workflow.create_ts,
			ts: rivet_util::timestamp::now(),
			ray_id: workflow.ray_id,
			version: 1,

			registry,
			db,

			conn,

			input: Arc::new(workflow.input),

			event_history: event_history.clone(),
			cursor: Cursor::new(event_history, Location::empty()),
			loop_location: None,

			msg_ctx,
		})
	}

	/// Creates a workflow ctx reference with a given version.
	pub fn v<'a>(&'a mut self, version: usize) -> VersionedWorkflowCtx<'a> {
		VersionedWorkflowCtx::new(self, version)
	}

	/// Errors if the given version is less than the current version.
	pub(crate) fn compare_version(
		&self,
		step: impl std::fmt::Display,
		version: usize,
	) -> WorkflowResult<()> {
		if version < self.version {
			Err(WorkflowError::HistoryDiverged(format!(
				"version of {step} is less than that of the current context (v{} < v{})",
				version, self.version
			)))
		} else {
			Ok(())
		}
	}

	pub(crate) async fn run(mut self) -> WorkflowResult<()> {
		tracing::info!(name=%self.name, id=%self.workflow_id, "running workflow");

		// Lookup workflow
		let workflow = self.registry.get_workflow(&self.name)?;

		// Run workflow
		let mut res = (workflow.run)(&mut self).await;

		// Validate no leftover events
		if res.is_ok() {
			if let Err(err) = self.cursor().check_clear() {
				res = Err(err);
			}
		}

		match res {
			Ok(output) => {
				tracing::info!(name=%self.name, id=%self.workflow_id, "workflow completed");

				let mut retries = 0;
				let mut interval = tokio::time::interval(DB_ACTION_RETRY);
				interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

				// Retry loop
				loop {
					interval.tick().await;

					// Write output
					if let Err(err) = self.db.commit_workflow(self.workflow_id, &output).await {
						if retries > MAX_DB_ACTION_RETRIES {
							return Err(err.into());
						}
						retries += 1;
					} else {
						break;
					}
				}
			}
			Err(err) => {
				// Retry the workflow if its recoverable
				let deadline_ts = if let Some(deadline_ts) = err.deadline_ts() {
					Some(deadline_ts)
				} else if err.is_retryable() {
					Some(rivet_util::timestamp::now() + RETRY_TIMEOUT_MS as i64)
				} else {
					None
				};

				// These signals come from a `listen` call that did not receive any signals. The workflow will
				// be retried when a signal is published
				let wake_signals = err.signals();

				// This sub workflow comes from a `wait_for_workflow` call on a workflow that did not
				// finish. This workflow will be retried when the sub workflow completes
				let wake_sub_workflow = err.sub_workflow();

				if err.is_recoverable() && !err.is_retryable() {
					tracing::info!(name=%self.name, id=%self.workflow_id, ?err, "workflow sleeping");
				} else {
					tracing::error!(name=%self.name, id=%self.workflow_id, ?err, "workflow error");

					metrics::WORKFLOW_ERRORS
						.with_label_values(&[&self.name, err.to_string().as_str()])
						.inc();
				}

				let err_str = err.to_string();

				let mut retries = 0;
				let mut interval = tokio::time::interval(DB_ACTION_RETRY);
				interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

				// Retry loop
				loop {
					interval.tick().await;

					// Write output
					let res = self
						.db
						.fail_workflow(
							self.workflow_id,
							false,
							deadline_ts,
							wake_signals,
							wake_sub_workflow,
							&err_str,
						)
						.await;

					if let Err(err) = res {
						if retries > MAX_DB_ACTION_RETRIES {
							return Err(err.into());
						}
						retries += 1;
					} else {
						break;
					}
				}
			}
		}

		Ok(())
	}

	/// Run then handle the result of an activity.
	async fn run_activity<A: Activity>(
		&mut self,
		input: &A::Input,
		event_id: &EventId,
		location: &Location,
		create_ts: i64,
	) -> WorkflowResult<A::Output> {
		tracing::debug!(name=%self.name, id=%self.workflow_id, activity_name=%A::NAME, "running activity");

		let ctx = ActivityCtx::new(
			self.workflow_id,
			self.db.clone(),
			&self.conn,
			self.create_ts,
			self.ray_id,
			A::NAME,
		);

		let start_instant = Instant::now();

		let res = tokio::time::timeout(A::TIMEOUT, A::run(&ctx, input))
			.await
			.map_err(|_| WorkflowError::ActivityTimeout(0));

		let dt = start_instant.elapsed().as_secs_f64();

		match res {
			Ok(Ok(output)) => {
				tracing::debug!("activity success");

				// Write output
				let input_val = serde_json::value::to_raw_value(input)
					.map_err(WorkflowError::SerializeActivityInput)?;
				let output_val = serde_json::value::to_raw_value(&output)
					.map_err(WorkflowError::SerializeActivityOutput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						location,
						self.version,
						event_id,
						create_ts,
						&input_val,
						Ok(&output_val),
						self.loop_location(),
					)
					.await?;

				metrics::ACTIVITY_DURATION
					.with_label_values(&[&self.name, A::NAME, ""])
					.observe(dt);

				Ok(output)
			}
			Ok(Err(err)) => {
				tracing::debug!(?err, "activity error");

				let err_str = err.to_string();
				let input_val = serde_json::value::to_raw_value(input)
					.map_err(WorkflowError::SerializeActivityInput)?;

				// Write error (failed state)
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						location,
						self.version,
						event_id,
						create_ts,
						&input_val,
						Err(&err_str),
						self.loop_location(),
					)
					.await?;

				if !err.is_workflow_recoverable() {
					metrics::ACTIVITY_ERRORS
						.with_label_values(&[&self.name, A::NAME, &err_str])
						.inc();
				}
				metrics::ACTIVITY_DURATION
					.with_label_values(&[&self.name, A::NAME, &err_str])
					.observe(dt);

				Err(WorkflowError::ActivityFailure(err, 0))
			}
			Err(err) => {
				tracing::debug!("activity timeout");

				let err_str = err.to_string();
				let input_val = serde_json::value::to_raw_value(input)
					.map_err(WorkflowError::SerializeActivityInput)?;

				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						location,
						self.version,
						event_id,
						create_ts,
						&input_val,
						Err(&err_str),
						self.loop_location(),
					)
					.await?;

				metrics::ACTIVITY_ERRORS
					.with_label_values(&[&self.name, A::NAME, &err_str])
					.inc();
				metrics::ACTIVITY_DURATION
					.with_label_values(&[&self.name, A::NAME, &err_str])
					.observe(dt);

				Err(err)
			}
		}
	}

	/// Creates a new workflow run with one more depth in the location.
	/// - **Not to be used directly by workflow users. For implementation uses only.**
	/// - **Remember to validate history after this branch is used.**
	pub(crate) async fn branch(&mut self) -> WorkflowResult<Self> {
		self.branch_inner(self.input.clone(), self.version, None)
			.await
	}

	pub(crate) async fn branch_inner(
		&mut self,
		input: Arc<Box<serde_json::value::RawValue>>,
		version: usize,
		// We allow providing a custom location in the case of loops, which take the spot of the branch event.
		custom_location: Option<Location>,
	) -> WorkflowResult<Self> {
		let location = if let Some(location) = custom_location {
			location
		} else {
			let history_res = self.cursor.compare_branch(version)?;
			let location = self.cursor.current_location_for(&history_res);

			// Validate history is consistent
			if !matches!(history_res, HistoryResult::Event(_)) {
				self.db
					.commit_workflow_branch_event(
						self.workflow_id,
						&location,
						self.version,
						self.loop_location.as_ref(),
					)
					.await?;
			}

			location
		};

		Ok(WorkflowCtx {
			workflow_id: self.workflow_id,
			name: self.name.clone(),
			create_ts: self.create_ts,
			ts: self.ts,
			ray_id: self.ray_id,
			version,

			registry: self.registry.clone(),
			db: self.db.clone(),

			conn: self.conn.clone(),

			input,

			event_history: self.event_history.clone(),
			cursor: Cursor::new(self.event_history.clone(), location),
			loop_location: self.loop_location.clone(),

			msg_ctx: self.msg_ctx.clone(),
		})
	}

	/// Like `branch` but it does not add another layer of depth.
	/// - **Not to be used directly by workflow users. For implementation uses only.**
	pub fn step(&mut self) -> Self {
		let branch = self.clone();

		self.cursor.inc();

		branch
	}
}

impl WorkflowCtx {
	/// Wait for another workflow's response. If no response was found after polling the database, this
	/// workflow will go to sleep until the sub workflow completes.
	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		sub_workflow_id: Uuid,
	) -> GlobalResult<W::Output> {
		tracing::info!(name=%self.name, id=%self.workflow_id, sub_workflow_name=%W::NAME, ?sub_workflow_id, "waiting for workflow");

		let mut retries = 0;
		let mut interval = tokio::time::interval(SUB_WORKFLOW_RETRY);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;

			// Check if state finished
			let workflow = self
				.db
				.get_workflow(sub_workflow_id)
				.await
				.map_err(GlobalError::raw)?
				.ok_or(WorkflowError::WorkflowNotFound)
				.map_err(GlobalError::raw)?;

			if let Some(output) = workflow.parse_output::<W>().map_err(GlobalError::raw)? {
				return Ok(output);
			} else {
				if retries > MAX_SUB_WORKFLOW_RETRIES {
					return Err(WorkflowError::SubWorkflowIncomplete(sub_workflow_id))
						.map_err(GlobalError::raw);
				}
				retries += 1;
			}
		}
	}

	/// Creates a sub workflow builder.
	pub fn workflow<I>(&mut self, input: I) -> builder::sub_workflow::SubWorkflowBuilder<I>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		builder::sub_workflow::SubWorkflowBuilder::new(self, self.version, input)
	}

	/// Run activity. Will replay on failure.
	pub async fn activity<I>(
		&mut self,
		input: I,
	) -> GlobalResult<<<I as ActivityInput>::Activity as Activity>::Output>
	where
		I: ActivityInput,
		<I as ActivityInput>::Activity: Activity<Input = I>,
	{
		let event_id = EventId::new(I::Activity::NAME, &input);

		let history_res = self
			.cursor
			.compare_activity(self.version, &event_id)
			.map_err(GlobalError::raw)?;
		let location = self.cursor.current_location_for(&history_res);

		// Activity was ran before
		let output = if let HistoryResult::Event(activity) = history_res {
			// Activity succeeded
			if let Some(output) = activity.parse_output().map_err(GlobalError::raw)? {
				tracing::debug!(name=%self.name, id=%self.workflow_id, activity_name=%I::Activity::NAME, "replaying activity");

				output
			}
			// Activity failed, retry
			else {
				let error_count = activity.error_count;

				match self
					.run_activity::<I::Activity>(&input, &event_id, &location, activity.create_ts)
					.await
				{
					Err(err) => {
						// Convert error in the case of max retries exceeded. This will only act on retryable
						// errors
						let err = match err {
							WorkflowError::ActivityFailure(err, _) => {
								if error_count + 1 >= I::Activity::MAX_RETRIES {
									WorkflowError::ActivityMaxFailuresReached(err)
								} else {
									// Add error count to the error for backoff calculation
									WorkflowError::ActivityFailure(err, error_count)
								}
							}
							WorkflowError::ActivityTimeout(_) => {
								if error_count + 1 >= I::Activity::MAX_RETRIES {
									WorkflowError::ActivityMaxFailuresReached(GlobalError::raw(err))
								} else {
									// Add error count to the error for backoff calculation
									WorkflowError::ActivityTimeout(error_count)
								}
							}
							WorkflowError::OperationTimeout(_) => {
								if error_count + 1 >= I::Activity::MAX_RETRIES {
									WorkflowError::ActivityMaxFailuresReached(GlobalError::raw(err))
								} else {
									// Add error count to the error for backoff calculation
									WorkflowError::OperationTimeout(error_count)
								}
							}
							_ => err,
						};

						return Err(GlobalError::raw(err));
					}
					x => x.map_err(GlobalError::raw)?,
				}
			}
		}
		// This is a new activity
		else {
			self.run_activity::<I::Activity>(
				&input,
				&event_id,
				&location,
				rivet_util::timestamp::now(),
			)
			.await
			.map_err(GlobalError::raw)?
		};

		// Move to next event
		self.cursor.update(&location);

		Ok(output)
	}

	/// Joins multiple executable actions (activities, closures) and awaits them simultaneously. This does not
	/// short circuit in the event of an error to make sure activity side effects are recorded.
	pub async fn join<T: Executable>(&mut self, exec: T) -> GlobalResult<T::Output> {
		exec.execute(self).await
	}

	/// Tests if the given error is unrecoverable. If it is, allows the user to run recovery code safely.
	/// Should always be used when trying to handle activity errors manually.
	pub fn catch_unrecoverable<T>(
		&mut self,
		res: GlobalResult<T>,
	) -> GlobalResult<GlobalResult<T>> {
		match res {
			Err(GlobalError::Raw(inner_err)) => {
				match inner_err.downcast::<WorkflowError>() {
					Ok(inner_err) => {
						// Despite "history diverged" errors being unrecoverable, they should not have be returned
						// by this function because the state of the history is already messed up and no new
						// workflow items can be run.
						if !inner_err.is_recoverable()
							&& !matches!(*inner_err, WorkflowError::HistoryDiverged(_))
						{
							self.cursor.inc();

							return Ok(Err(GlobalError::Raw(inner_err)));
						} else {
							return Err(GlobalError::Raw(inner_err));
						}
					}
					Err(err) => {
						return Err(GlobalError::Raw(err));
					}
				}
			}
			Err(err) => Err(err),
			Ok(x) => Ok(Ok(x)),
		}
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&mut self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(self, self.version, body)
	}

	/// Listens for a signal for a short time before setting the workflow to sleep. Once the signal is
	/// received, the workflow will be woken up and continue.
	pub async fn listen<T: Listen>(&mut self) -> GlobalResult<T> {
		let history_res = self
			.cursor
			.compare_signal(self.version)
			.map_err(GlobalError::raw)?;
		let location = self.cursor.current_location_for(&history_res);

		// Signal received before
		let signal = if let HistoryResult::Event(signal) = history_res {
			tracing::debug!(
				name=%self.name,
				id=%self.workflow_id,
				signal_name=%signal.name,
				"replaying signal"
			);

			T::parse(&signal.name, &signal.body).map_err(GlobalError::raw)?
		}
		// Listen for new messages
		else {
			tracing::info!(name=%self.name, id=%self.workflow_id, "listening for signal");

			let mut retries = 0;
			let mut interval = tokio::time::interval(SIGNAL_RETRY);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			let mut ctx = ListenCtx::new(self, &location);

			loop {
				interval.tick().await;

				match T::listen(&mut ctx).await {
					Ok(res) => break res,
					Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => {
						if retries > MAX_SIGNAL_RETRIES {
							return Err(err).map_err(GlobalError::raw);
						}
						retries += 1;
					}
					Err(err) => return Err(GlobalError::raw(err)),
				}
			}
		};

		// Move to next event
		self.cursor.update(&location);

		Ok(signal)
	}

	/// Execute a custom listener.
	pub async fn custom_listener<T: CustomListener>(
		&mut self,
		listener: &T,
	) -> GlobalResult<<T as CustomListener>::Output> {
		let history_res = self
			.cursor
			.compare_signal(self.version)
			.map_err(GlobalError::raw)?;
		let location = self.cursor.current_location_for(&history_res);

		// Signal received before
		let signal = if let HistoryResult::Event(signal) = history_res {
			tracing::debug!(
				name=%self.name,
				id=%self.workflow_id,
				signal_name=%signal.name,
				"replaying signal",
			);

			T::parse(&signal.name, &signal.body).map_err(GlobalError::raw)?
		}
		// Listen for new messages
		else {
			tracing::info!(name=%self.name, id=%self.workflow_id, "listening for signal");

			let mut retries = 0;
			let mut interval = tokio::time::interval(SIGNAL_RETRY);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			let mut ctx = ListenCtx::new(self, &location);

			loop {
				interval.tick().await;

				match listener.listen(&mut ctx).await {
					Ok(res) => break res,
					Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => {
						if retries > MAX_SIGNAL_RETRIES {
							return Err(err).map_err(GlobalError::raw);
						}
						retries += 1;
					}
					Err(err) => return Err(GlobalError::raw(err)),
				}
			}
		};

		// Move to next event
		self.cursor.update(&location);

		Ok(signal)
	}

	// TODO: Currently implemented wrong, if no signal is received it should still write a signal row to the
	// database so that upon replay it again receives no signal
	// /// Checks if the given signal exists in the database.
	// pub async fn query_signal<T: Listen>(&mut self) -> GlobalResult<Option<T>> {
	// 	let event = self.current_history_event();

	// 	// Signal received before
	// 	let signal = if let Some(event) = event {
	// 		tracing::debug!(name=%self.name, id=%self.workflow_id, "replaying signal");

	// 		// Validate history is consistent
	// 		let Event::Signal(signal) = event else {
	// 			return Err(WorkflowError::HistoryDiverged(format!(
	// 				"expected {event} at {}, found signal",
	// 				self.loc(),
	// 			)))
	// 			.map_err(GlobalError::raw);
	// 		};

	// 		Some(T::parse(&signal.name, signal.body.clone()).map_err(GlobalError::raw)?)
	// 	}
	// 	// Listen for new message
	// 	else {
	// 		let mut ctx = ListenCtx::new(self);

	// 		match T::listen(&mut ctx).await {
	// 			Ok(res) => Some(res),
	// 			Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => None,
	// 			Err(err) => return Err(err).map_err(GlobalError::raw),
	// 		}
	// 	};

	// 	// Move to next event
	// 	self.cursor.update();

	// 	Ok(signal)
	// }

	/// Creates a message builder.
	pub fn msg<M>(&mut self, body: M) -> builder::message::MessageBuilder<M>
	where
		M: Message,
	{
		builder::message::MessageBuilder::new(self, self.version, body)
	}

	/// Runs workflow steps in a loop. **Ensure that there are no side effects caused by the code in this
	/// callback**. If you need side causes or side effects, use a native rust loop.
	pub async fn repeat<F, T>(&mut self, mut cb: F) -> GlobalResult<T>
	where
		F: for<'a> FnMut(&'a mut WorkflowCtx) -> AsyncResult<'a, Loop<T>>,
		T: Serialize + DeserializeOwned,
	{
		let history_res = self
			.cursor
			.compare_loop(self.version)
			.map_err(GlobalError::raw)?;
		let loop_location = self.cursor.current_location_for(&history_res);

		// Loop existed before
		let (iteration, output) = if let HistoryResult::Event(loop_event) = history_res {
			let output = loop_event.parse_output().map_err(GlobalError::raw)?;

			(loop_event.iteration, output)
		} else {
			(0, None)
		};

		let mut loop_branch = self
			.branch_inner(
				self.input.clone(),
				self.version,
				Some(loop_location.clone()),
			)
			.await
			.map_err(GlobalError::raw)?;
		// Shift by iteration count
		loop_branch.cursor.set_idx(iteration);

		// Loop complete
		let output = if let Some(output) = output {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "replaying loop output");

			output
		}
		// Run loop
		else {
			tracing::info!(name=%self.name, id=%self.workflow_id, "running loop");

			loop {
				// HACK: We have to temporarily set the loop location to the current loop so that the branch
				// event created in `WorkflowCtx::branch` has the correct loop location.
				let old_loop_location = loop_branch.loop_location.replace(loop_location.clone());
				let mut iteration_branch = loop_branch.branch().await.map_err(GlobalError::raw)?;

				// Set back to previous loop location
				loop_branch.loop_location = old_loop_location;

				// Set branch loop location to the current loop
				iteration_branch.loop_location = Some(loop_location.clone());

				// Run loop
				match cb(&mut iteration_branch).await? {
					Loop::Continue => {
						self.db
							.upsert_loop(
								self.workflow_id,
								&loop_location,
								self.version,
								loop_branch.cursor.iter_idx(),
								None,
								self.loop_location(),
							)
							.await?;

						// Validate no leftover events
						iteration_branch
							.cursor
							.check_clear()
							.map_err(GlobalError::raw)?;

						// Move to next event
						self.cursor.update(iteration_branch.cursor().root());
					}
					Loop::Break(res) => {
						let output_val = serde_json::value::to_raw_value(&res)
							.map_err(WorkflowError::SerializeLoopOutput)
							.map_err(GlobalError::raw)?;

						self.db
							.upsert_loop(
								self.workflow_id,
								&loop_location,
								self.version,
								loop_branch.cursor.iter_idx(),
								Some(&output_val),
								self.loop_location(),
							)
							.await?;

						// Validate no leftover events
						iteration_branch
							.cursor
							.check_clear()
							.map_err(GlobalError::raw)?;

						// Move to next event
						self.cursor.update(iteration_branch.cursor().root());

						break res;
					}
				}
			}
		};

		Ok(output)
	}

	pub async fn sleep(&mut self, duration: impl DurationToMillis) -> GlobalResult<()> {
		let ts = rivet_util::timestamp::now() as u64 + duration.to_millis()?;

		self.sleep_until(ts as i64).await
	}

	pub async fn sleep_until(&mut self, time: impl TsToMillis) -> GlobalResult<()> {
		let history_res = self
			.cursor
			.compare_sleep(self.version)
			.map_err(GlobalError::raw)?;
		let location = self.cursor.current_location_for(&history_res);

		// Slept before
		let (deadline_ts, replay) = if let HistoryResult::Event(sleep) = history_res {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "replaying sleep");

			(sleep.deadline_ts, true)
		}
		// Sleep
		else {
			let deadline_ts = time.to_millis()?;

			self.db
				.commit_workflow_sleep_event(
					self.workflow_id,
					&location,
					self.version,
					deadline_ts,
					self.loop_location(),
				)
				.await?;

			(deadline_ts, false)
		};

		let duration = deadline_ts.saturating_sub(rivet_util::timestamp::now());

		// No-op
		if duration <= 0 {
			if !replay && duration < -50 {
				tracing::warn!(name=%self.name, id=%self.workflow_id, %duration, "tried to sleep for a negative duration");
			}
		}
		// Sleep in memory if duration is shorter than the worker tick
		else if duration < worker::TICK_INTERVAL.as_millis() as i64 + 1 {
			tracing::info!(name=%self.name, id=%self.workflow_id, %deadline_ts, "sleeping in memory");

			tokio::time::sleep(std::time::Duration::from_millis(duration.try_into()?)).await;
		}
		// Workflow sleep
		else {
			tracing::info!(name=%self.name, id=%self.workflow_id, %deadline_ts, "sleeping");

			return Err(WorkflowError::Sleep(deadline_ts)).map_err(GlobalError::raw);
		}

		// Move to next event
		self.cursor.update(&location);

		Ok(())
	}

	/// Listens for a signal for a short time with a timeout before setting the workflow to sleep. Once the
	/// signal is received, the workflow will be woken up and continue.
	///
	/// Internally this is a sleep event and a signal event.
	pub async fn listen_with_timeout<T: Listen>(
		&mut self,
		duration: impl DurationToMillis,
	) -> GlobalResult<Option<T>> {
		let time = (rivet_util::timestamp::now() as u64 + duration.to_millis()?) as i64;
		let history_res = self
			.cursor
			.compare_sleep(self.version)
			.map_err(GlobalError::raw)?;
		let sleep_location = self.cursor.current_location_for(&history_res);

		// Slept before
		let (deadline_ts, state, replay) = if let HistoryResult::Event(sleep) = history_res {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "replaying sleep");

			(sleep.deadline_ts, sleep.state, true)
		}
		// Sleep
		else {
			let deadline_ts = TsToMillis::to_millis(time)?;

			self.db
				.commit_workflow_sleep_event(
					self.workflow_id,
					&sleep_location,
					self.version,
					deadline_ts,
					self.loop_location(),
				)
				.await?;

			(deadline_ts, SleepState::Normal, false)
		};

		// Location of the signal event (comes after the sleep event)
		let sleep_location2 = self.cursor.current_location_for(&history_res);

		// Move to next event
		self.cursor.update(&sleep_location);

		// Signal received before
		if matches!(state, SleepState::Interrupted) {
			let history_res = self
				.cursor
				.compare_signal(self.version)
				.map_err(GlobalError::raw)?;
			let location = self.cursor.current_location_for(&history_res);

			if let HistoryResult::Event(signal) = history_res {
				tracing::debug!(
					name=%self.name,
					id=%self.workflow_id,
					signal_name=%signal.name,
					"replaying signal",
				);

				let signal = T::parse(&signal.name, &signal.body).map_err(GlobalError::raw)?;

				// Move to next event
				self.cursor.update(&location);

				// Short circuit
				return Ok(Some(signal));
			} else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected signal at {}, found nothing",
					location,
				)))
				.map_err(GlobalError::raw);
			}
		}

		let duration = deadline_ts.saturating_sub(rivet_util::timestamp::now());

		// Duration is now 0, timeout is over
		let signal = if duration <= 0 {
			if !replay && duration < -50 {
				tracing::warn!(
					name=%self.name,
					id=%self.workflow_id,
					%duration,
					"tried to sleep for a negative duration",
				);
			}

			// After timeout is over, check once for signal
			if matches!(state, SleepState::Normal) {
				let mut ctx = ListenCtx::new(self, &sleep_location2);

				match T::listen(&mut ctx).await {
					Ok(x) => Some(x),
					Err(WorkflowError::NoSignalFound(_)) => None,
					Err(err) => return Err(GlobalError::raw(err)),
				}
			} else {
				None
			}
		}
		// Sleep in memory if duration is shorter than the worker tick
		else if duration < worker::TICK_INTERVAL.as_millis() as i64 + 1 {
			tracing::info!(name=%self.name, id=%self.workflow_id, %deadline_ts, "sleeping in memory");

			let res = tokio::time::timeout(
				std::time::Duration::from_millis(duration.try_into()?),
				async {
					tracing::info!(name=%self.name, id=%self.workflow_id, "listening for signal with timeout");

					let mut interval = tokio::time::interval(SIGNAL_RETRY);
					interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

					let mut ctx = ListenCtx::new(self, &sleep_location2);

					loop {
						interval.tick().await;

						match T::listen(&mut ctx).await {
							// Retry
							Err(WorkflowError::NoSignalFound(_)) => {}
							x => return x,
						}
					}
				},
			)
			.await;

			match res {
				Ok(res) => Some(res.map_err(GlobalError::raw)?),
				Err(_) => {
					tracing::info!(name=%self.name, id=%self.workflow_id, "timed out listening for signal");

					None
				}
			}
		}
		// Workflow sleep for long durations
		else {
			tracing::info!(name=%self.name, id=%self.workflow_id, "listening for signal with timeout");

			let mut retries = 0;
			let mut interval = tokio::time::interval(SIGNAL_RETRY);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			let mut ctx = ListenCtx::new(self, &sleep_location2);

			loop {
				interval.tick().await;

				match T::listen(&mut ctx).await {
					Ok(res) => break Some(res),
					Err(WorkflowError::NoSignalFound(signals)) => {
						if retries > MAX_SIGNAL_RETRIES {
							return Err(WorkflowError::NoSignalFoundAndSleep(signals, deadline_ts))
								.map_err(GlobalError::raw);
						}
						retries += 1;
					}
					Err(err) => return Err(GlobalError::raw(err)),
				}
			}
		};

		// Update sleep state
		if signal.is_some() {
			self.db
				.update_workflow_sleep_event_state(
					self.workflow_id,
					&sleep_location,
					SleepState::Interrupted,
				)
				.await?;

			// Move to next event
			self.cursor.update(&sleep_location2);
		} else if matches!(state, SleepState::Normal) {
			self.db
				.update_workflow_sleep_event_state(
					self.workflow_id,
					&sleep_location,
					SleepState::Uninterrupted,
				)
				.await?;
		}

		Ok(signal)
	}

	/// Represents a removed workflow step.
	pub async fn removed<T: Removed>(&mut self) -> GlobalResult<()> {
		// Existing event
		if self
			.cursor
			.compare_removed::<T>()
			.map_err(GlobalError::raw)?
		{
			tracing::debug!(
				name=%self.name,
				id=%self.workflow_id,
				"skipping removed step",
			);
		}
		// New "removed" event
		else {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "inserting removed step");

			self.db
				.commit_workflow_removed_event(
					self.workflow_id,
					&self.cursor.current_location(),
					T::event_type(),
					T::name(),
					self.loop_location(),
				)
				.await?;
		};

		// Move to next event
		self.cursor.inc();

		Ok(())
	}

	/// Returns true if the workflow has never reached this point before and is consistent for all future
	/// executions of this workflow.
	pub async fn is_new(&mut self) -> GlobalResult<bool> {
		// Existing event
		let is_new = if let Some(is_new) = self
			.cursor
			.compare_version_check()
			.map_err(GlobalError::raw)?
		{
			is_new
		} else {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "inserting version check");

			self.db
				.commit_workflow_version_check_event(
					self.workflow_id,
					&self.cursor.current_location(),
					self.loop_location(),
				)
				.await?;

			true
		};

		if is_new {
			// Move to next event
			self.cursor.inc();
		}

		Ok(is_new)
	}
}

impl WorkflowCtx {
	pub(crate) fn input(&self) -> &Arc<Box<serde_json::value::RawValue>> {
		&self.input
	}

	pub(crate) fn loop_location(&self) -> Option<&Location> {
		self.loop_location.as_ref()
	}

	pub(crate) fn db(&self) -> &DatabaseHandle {
		&self.db
	}

	pub(crate) fn msg_ctx(&self) -> &MessageCtx {
		&self.msg_ctx
	}

	pub(crate) fn cursor(&self) -> &Cursor {
		&self.cursor
	}

	pub(crate) fn cursor_mut(&mut self) -> &mut Cursor {
		&mut self.cursor
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn workflow_id(&self) -> Uuid {
		self.workflow_id
	}

	pub fn ray_id(&self) -> Uuid {
		self.ray_id
	}

	pub fn version(&self) -> usize {
		self.version
	}

	pub(crate) fn set_version(&mut self, version: usize) {
		self.version = version;
	}

	/// Timestamp at which this workflow run started.
	pub fn ts(&self) -> i64 {
		self.ts
	}

	/// Timestamp at which the workflow was created.
	pub fn create_ts(&self) -> i64 {
		self.create_ts
	}

	/// Time between when the timestamp was processed and when it was published.
	pub fn req_dt(&self) -> i64 {
		self.ts.saturating_sub(self.create_ts)
	}
}

pub enum Loop<T> {
	Continue,
	Break(T),
}
