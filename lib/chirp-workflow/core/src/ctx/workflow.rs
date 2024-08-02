use std::{collections::HashMap, sync::Arc};

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use tokio::time::Duration;
use uuid::Uuid;

use crate::{
	schema::{ActivityId, Event},
	util::{self, Location},
	Activity, ActivityCtx, ActivityInput, DatabaseHandle, Executable, Listen, PulledWorkflow,
	RegistryHandle, Signal, SignalRow, Workflow, WorkflowError, WorkflowInput, WorkflowResult,
};

// Time to delay a worker from retrying after an error
const RETRY_TIMEOUT: Duration = Duration::from_millis(2000);
// Poll interval when polling for signals in-process
const SIGNAL_RETRY: Duration = Duration::from_millis(100);
// Most in-process signal poll tries
const MAX_SIGNAL_RETRIES: usize = 16;
// Poll interval when polling for a sub workflow in-process
const SUB_WORKFLOW_RETRY: Duration = Duration::from_millis(150);
// Most in-process sub workflow poll tries
const MAX_SUB_WORKFLOW_RETRIES: usize = 4;
// Retry interval for failed db actions
const DB_ACTION_RETRY: Duration = Duration::from_millis(150);
// Most db action retries
const MAX_DB_ACTION_RETRIES: usize = 5;

// TODO: Use generics to store input instead of a string
#[derive(Clone)]
pub struct WorkflowCtx {
	pub workflow_id: Uuid,
	/// Name of the workflow to run in the registry.
	pub name: String,
	create_ts: i64,
	ts: i64,
	ray_id: Uuid,

	registry: RegistryHandle,
	db: DatabaseHandle,

	conn: rivet_connection::Connection,

	/// All events that have ever been recorded on this workflow.
	///
	/// If replaying, the workflow will check that the `ActivityId` is the same for all activities
	/// to make sure the workflow hasn't diverged.
	///
	/// The reason this type is a hashmap is to allow querying by location.
	event_history: Arc<HashMap<Location, Vec<Event>>>,
	/// Input data passed to this workflow.
	pub(crate) input: Arc<serde_json::Value>,

	root_location: Location,
	location_idx: usize,
}

impl WorkflowCtx {
	pub fn new(
		registry: RegistryHandle,
		db: DatabaseHandle,
		conn: rivet_connection::Connection,
		workflow: PulledWorkflow,
	) -> GlobalResult<Self> {
		Ok(WorkflowCtx {
			workflow_id: workflow.workflow_id,
			name: workflow.workflow_name,
			create_ts: workflow.create_ts,
			ts: rivet_util::timestamp::now(),

			ray_id: workflow.ray_id,

			registry,
			db,

			conn,

			event_history: Arc::new(
				util::combine_events(
					workflow.activity_events,
					workflow.signal_events,
					workflow.sub_workflow_events,
				)
				.map_err(GlobalError::raw)?,
			),
			input: Arc::new(workflow.input),

			root_location: Box::new([]),
			location_idx: 0,
		})
	}

	/// Creates a new workflow run with one more depth in the location. Meant to be implemented and not used
	/// directly in workflows.
	pub fn branch(&mut self) -> Self {
		let branch = WorkflowCtx {
			workflow_id: self.workflow_id,
			name: self.name.clone(),
			create_ts: self.create_ts,
			ts: self.ts,
			ray_id: self.ray_id,

			registry: self.registry.clone(),
			db: self.db.clone(),

			conn: self.conn.clone(),

			event_history: self.event_history.clone(),
			input: self.input.clone(),

			root_location: self
				.root_location
				.iter()
				.cloned()
				.chain(std::iter::once(self.location_idx))
				.collect(),
			location_idx: 0,
		};

		self.location_idx += 1;

		branch
	}

	/// Like `branch` but it does not add another layer of depth. Meant to be implemented and not used
	/// directly in workflows.
	pub fn step(&mut self) -> Self {
		let branch = self.clone();

		self.location_idx += 1;

		branch
	}

	/// Returns only the history relevant to this workflow run (based on location).
	fn relevant_history(&self) -> impl Iterator<Item = &Event> {
		self.event_history
			.get(&self.root_location)
			// `into_iter` and `flatten` are for the `Option`
			.into_iter()
			.flatten()
	}

	fn full_location(&self) -> Location {
		self.root_location
			.iter()
			.cloned()
			.chain(std::iter::once(self.location_idx))
			.collect()
	}

	// Purposefully infallible
	pub(crate) async fn run(mut self) {
		if let Err(err) = Self::run_inner(&mut self).await {
			tracing::error!(?err, "unhandled error");
		}
	}

	async fn run_inner(&mut self) -> WorkflowResult<()> {
		tracing::info!(name=%self.name, id=%self.workflow_id, "running workflow");

		// Lookup workflow
		let workflow = self.registry.get_workflow(&self.name)?;

		// Run workflow
		match (workflow.run)(self).await {
			Ok(output) => {
				tracing::info!(name=%self.name, id=%self.workflow_id, "workflow success");

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
				tracing::warn!(name=%self.name, id=%self.workflow_id, ?err, "workflow error");

				// Retry the workflow if its recoverable
				let deadline = if err.is_recoverable() {
					Some(rivet_util::timestamp::now() + RETRY_TIMEOUT.as_millis() as i64)
				} else {
					None
				};

				// These signals come from a `listen` call that did not receive any signals. The workflow will
				// be retried when a signal is published
				let wake_signals = err.signals();

				// This sub workflow come from a `wait_for_workflow` call on a workflow that did not
				// finish. This workflow will be retried when the sub workflow completes
				let wake_sub_workflow = err.sub_workflow();

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
							deadline,
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
		activity_id: &ActivityId,
		create_ts: i64,
	) -> WorkflowResult<A::Output> {
		let ctx = ActivityCtx::new(
			self.db.clone(),
			&self.conn,
			self.create_ts,
			self.ray_id,
			A::NAME,
		);

		let res = tokio::time::timeout(A::TIMEOUT, A::run(&ctx, input))
			.await
			.map_err(|_| WorkflowError::ActivityTimeout);

		match res {
			Ok(Ok(output)) => {
				tracing::debug!("activity success");

				// Write output
				let input_val =
					serde_json::to_value(input).map_err(WorkflowError::SerializeActivityInput)?;
				let output_val = serde_json::to_value(&output)
					.map_err(WorkflowError::SerializeActivityOutput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						self.full_location().as_ref(),
						activity_id,
						create_ts,
						input_val,
						Ok(output_val),
					)
					.await?;

				Ok(output)
			}
			Ok(Err(err)) => {
				tracing::debug!(?err, "activity error");

				// Write error (failed state)
				let input_val =
					serde_json::to_value(input).map_err(WorkflowError::SerializeActivityInput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						self.full_location().as_ref(),
						activity_id,
						create_ts,
						input_val,
						Err(&err.to_string()),
					)
					.await?;

				Err(WorkflowError::ActivityFailure(err))
			}
			Err(err) => {
				tracing::debug!("activity timeout");

				let input_val =
					serde_json::to_value(input).map_err(WorkflowError::SerializeActivityInput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						self.full_location().as_ref(),
						activity_id,
						create_ts,
						input_val,
						Err(&err.to_string()),
					)
					.await?;

				Err(err)
			}
		}
	}

	/// Checks for a signal to this workflow with any of the given signal names. Meant to be implemented and
	/// not used directly in workflows.
	pub async fn listen_any(&mut self, signal_names: &[&'static str]) -> WorkflowResult<SignalRow> {
		// Fetch new pending signal
		let signal = self
			.db
			.pull_next_signal(
				self.workflow_id,
				signal_names,
				self.full_location().as_ref(),
			)
			.await?;

		let Some(signal) = signal else {
			return Err(WorkflowError::NoSignalFound(Box::from(signal_names)));
		};

		tracing::info!(
			workflow_name=%self.name,
			workflow_id=%self.workflow_id,
			signal_id=%signal.signal_id,
			signal_name=%signal.signal_name,
			"signal received",
		);

		Ok(signal)
	}
}

impl WorkflowCtx {
	/// Dispatch another workflow.
	pub async fn dispatch_workflow<I>(&mut self, input: I) -> GlobalResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		self.dispatch_workflow_inner(None, input).await
	}

	/// Dispatch another workflow with tags.
	pub async fn dispatch_tagged_workflow<I>(&mut self, tags: &serde_json::Value, input: I) -> GlobalResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		self.dispatch_workflow_inner(Some(tags), input).await
	}

	async fn dispatch_workflow_inner<I>(&mut self, tags: Option<&serde_json::Value>, input: I) -> GlobalResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let id = if let Some(event) = event {
			// Validate history is consistent
			let Event::SubWorkflow(sub_workflow) = event else {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			};

			if sub_workflow.sub_workflow_name != I::Workflow::NAME {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			}

			tracing::debug!(
				name=%self.name,
				id=%self.workflow_id,
				sub_workflow_id=%sub_workflow.sub_workflow_id,
				"replaying workflow dispatch"
			);

			sub_workflow.sub_workflow_id
		}
		// Dispatch new workflow
		else {
			let name = I::Workflow::NAME;

			tracing::debug!(%name, ?tags, ?input, "dispatching workflow");

			let sub_workflow_id = Uuid::new_v4();

			// Serialize input
			let input_val = serde_json::to_value(input)
				.map_err(WorkflowError::SerializeWorkflowOutput)
				.map_err(GlobalError::raw)?;

			self.db
				.dispatch_sub_workflow(
					self.ray_id,
					self.workflow_id,
					self.full_location().as_ref(),
					sub_workflow_id,
					&name,
					tags,
					input_val,
				)
				.await
				.map_err(GlobalError::raw)?;

			tracing::info!(%name, ?sub_workflow_id, "workflow dispatched");

			sub_workflow_id
		};

		// Move to next event
		self.location_idx += 1;

		Ok(id)
	}

	/// Wait for another workflow's response. If no response was found after polling the database, this
	/// workflow will go to sleep until the sub workflow completes.
	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		sub_workflow_id: Uuid,
	) -> GlobalResult<W::Output> {
		tracing::info!(name = W::NAME, ?sub_workflow_id, "waiting for workflow");

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

	/// Runs a sub workflow in the same process as the current workflow (if possible) and returns its
	/// response.
	pub async fn workflow<I>(
		&mut self,
		input: I,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		self.workflow_inner(None, input).await
	}

	/// Runs a sub workflow with tags in the same process as the current workflow (if possible) and returns
	/// its response.
	pub async fn tagged_workflow<I>(
		&mut self,
		input: I,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		self.workflow_inner(None, input).await
	}

	async fn workflow_inner<I>(
		&mut self,
		tags: Option<&serde_json::Value>,
		input: I,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		// Lookup workflow
		let Ok(workflow) = self.registry.get_workflow(I::Workflow::NAME) else {
			tracing::warn!(
				name=%self.name,
				id=%self.workflow_id,
				sub_workflow_name=%I::Workflow::NAME,
				"sub workflow not found in current registry",
			);

			// TODO(RVT-3755): If a sub workflow is dispatched, then the worker is updated to include the sub
			// worker in the registry, this will diverge in history because it will try to run the sub worker
			// in-process during the replay
			// If the workflow isn't in the current registry, dispatch the workflow instead
			let sub_workflow_id = self.dispatch_workflow_inner(tags, input).await?;
			let output = self
				.wait_for_workflow::<I::Workflow>(sub_workflow_id)
				.await?;

			return Ok(output);
		};

		tracing::info!(name=%self.name, id=%self.workflow_id, sub_workflow_name=%I::Workflow::NAME, "running sub workflow");

		// Create a new branched workflow context for the sub workflow
		let mut ctx = WorkflowCtx {
			workflow_id: self.workflow_id,
			name: I::Workflow::NAME.to_string(),
			create_ts: rivet_util::timestamp::now(),
			ts: rivet_util::timestamp::now(),
			ray_id: self.ray_id,

			registry: self.registry.clone(),
			db: self.db.clone(),

			conn: self
				.conn
				.wrap(Uuid::new_v4(), self.ray_id, I::Workflow::NAME),

			event_history: self.event_history.clone(),

			// TODO(RVT-3756): This is redundant with the deserialization in `workflow.run` in the registry
			input: Arc::new(serde_json::to_value(input)?),

			root_location: self
				.root_location
				.iter()
				.cloned()
				.chain(std::iter::once(self.location_idx))
				.collect(),
			location_idx: 0,
		};

		self.location_idx += 1;

		// Run workflow
		let output = (workflow.run)(&mut ctx).await?;

		// TODO: RVT-3756
		// Deserialize output
		serde_json::from_value(output)
			.map_err(WorkflowError::DeserializeWorkflowOutput)
			.map_err(GlobalError::raw)
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
		let activity_id = ActivityId::new::<I::Activity>(&input);

		let event = { self.relevant_history().nth(self.location_idx) };

		// Activity was ran before
		let output = if let Some(event) = event {
			// Validate history is consistent
			let Event::Activity(activity) = event else {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			};

			if activity.activity_id != activity_id {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			}

			// Activity succeeded
			if let Some(output) = activity.parse_output().map_err(GlobalError::raw)? {
				output
			}
			// Activity failed, retry
			else {
				let error_count = activity.error_count;

				match self
					.run_activity::<I::Activity>(&input, &activity_id, activity.create_ts)
					.await
				{
					Err(err) => {
						// Convert error in the case of max retries exceeded. This will only act on retryable
						// errors
						let err = match err {
							WorkflowError::ActivityFailure(err) => {
								if error_count + 1 >= I::Activity::MAX_RETRIES {
									WorkflowError::ActivityMaxFailuresReached(err)
								} else {
									WorkflowError::ActivityFailure(err)
								}
							}
							WorkflowError::ActivityTimeout | WorkflowError::OperationTimeout => {
								if error_count + 1 >= I::Activity::MAX_RETRIES {
									WorkflowError::ActivityMaxFailuresReached(GlobalError::raw(err))
								} else {
									err
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
			self.run_activity::<I::Activity>(&input, &activity_id, rivet_util::timestamp::now())
				.await
				.map_err(GlobalError::raw)?
		};

		// Move to next event
		self.location_idx += 1;

		Ok(output)
	}

	/// Joins multiple executable actions (activities, closures) and awaits them simultaneously.
	pub async fn join<T: Executable>(&mut self, exec: T) -> GlobalResult<T::Output> {
		exec.execute(self).await
	}

	/// Sends a signal.
	pub async fn signal<T: Signal + Serialize>(
		&mut self,
		workflow_id: Uuid,
		body: T,
	) -> GlobalResult<Uuid> {
		tracing::debug!(name=%T::NAME, %workflow_id, "dispatching signal");

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::to_value(&body)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		self.db
			.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(signal_id)
	}

	/// Sends a tagged signal.
	pub async fn tagged_signal<T: Signal + Serialize>(
		&mut self,
		tags: &serde_json::Value,
		body: T,
	) -> GlobalResult<Uuid> {
		tracing::debug!(name=%T::NAME, ?tags, "dispatching tagged signal");

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::to_value(&body)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		self.db
			.publish_tagged_signal(self.ray_id, tags, signal_id, T::NAME, input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(signal_id)
	}

	/// Listens for a signal for a short time before setting the workflow to sleep. Once the signal is
	/// received, the workflow will be woken up and continue.
	pub async fn listen<T: Listen>(&mut self) -> GlobalResult<T> {
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let signal = if let Some(event) = event {
			// Validate history is consistent
			let Event::Signal(signal) = event else {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			};

			tracing::debug!(name=%self.name, id=%self.workflow_id, signal_name=%signal.name, "replaying signal");

			T::parse(&signal.name, signal.body.clone()).map_err(GlobalError::raw)?
		}
		// Listen for new messages
		else {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "listening for signal");

			let mut retries = 0;
			let mut interval = tokio::time::interval(SIGNAL_RETRY);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;

				match T::listen(self).await {
					Ok(res) => break res,
					Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => {
						if retries > MAX_SIGNAL_RETRIES {
							return Err(err).map_err(GlobalError::raw);
						}
						retries += 1;
					}
					err => return err.map_err(GlobalError::raw),
				}
			}
		};

		// Move to next event
		self.location_idx += 1;

		Ok(signal)
	}

	/// Checks if the given signal exists in the database.
	pub async fn query_signal<T: Listen>(&mut self) -> GlobalResult<Option<T>> {
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let signal = if let Some(event) = event {
			tracing::debug!(name=%self.name, id=%self.workflow_id, "replaying signal");

			// Validate history is consistent
			let Event::Signal(signal) = event else {
				return Err(WorkflowError::HistoryDiverged).map_err(GlobalError::raw);
			};

			Some(T::parse(&signal.name, signal.body.clone()).map_err(GlobalError::raw)?)
		}
		// Listen for new message
		else {
			match T::listen(self).await {
				Ok(res) => Some(res),
				Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => None,
				Err(err) => return Err(err).map_err(GlobalError::raw),
			}
		};

		// Move to next event
		self.location_idx += 1;

		Ok(signal)
	}

	// TODO: sleep_for, sleep_until
}

impl WorkflowCtx {
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
