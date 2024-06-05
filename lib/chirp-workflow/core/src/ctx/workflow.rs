use std::{collections::HashMap, sync::Arc};

use anyhow::*;
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
const RETRY_TIMEOUT: Duration = Duration::from_millis(100);
// Poll interval when polling for signals in-process
const SIGNAL_RETRY: Duration = Duration::from_millis(100);
// Most in-process signal poll tries
const MAX_SIGNAL_RETRIES: usize = 16;
// Poll interval when polling for a sub workflow in-process
const SUB_WORKFLOW_RETRY: Duration = Duration::from_millis(100);
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
	pub(crate) input: Arc<String>,

	root_location: Location,
	location_idx: usize,
}

impl WorkflowCtx {
	pub fn new(
		registry: RegistryHandle,
		db: DatabaseHandle,
		conn: rivet_connection::Connection,
		workflow: PulledWorkflow,
	) -> WorkflowResult<Self> {
		WorkflowResult::Ok(WorkflowCtx {
			workflow_id: workflow.workflow_id,
			name: workflow.workflow_name,

			registry,
			db,

			conn,

			event_history: Arc::new(util::combine_events(
				workflow.activity_events,
				workflow.signal_events,
				workflow.sub_workflow_events,
			)?),
			input: Arc::new(workflow.input),

			root_location: Box::new([]),
			location_idx: 0,
		})
	}

	/// Creates a new workflow run with one more depth in the location.
	pub fn branch(&mut self) -> Self {
		let branch = WorkflowCtx {
			workflow_id: self.workflow_id,
			name: self.name.clone(),

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

	/// Like `branch` but it does not add another layer of depth.
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
	pub(crate) async fn run_workflow(mut self) {
		if let Err(err) = Self::run_workflow_inner(&mut self).await {
			tracing::error!(?err, "unhandled error");
		}
	}

	async fn run_workflow_inner(&mut self) -> Result<()> {
		tracing::info!(id=%self.workflow_id, "running workflow");

		// Lookup workflow
		let workflow = self.registry.get_workflow(&self.name)?;

		// Run workflow
		match (workflow.run)(self).await {
			Result::Ok(output) => {
				tracing::info!(id=%self.workflow_id, "workflow success");

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
				tracing::warn!(id=%self.workflow_id, ?err, "workflow error");

				// TODO(RVT-3751): Save error to workflow

				let deadline = if err.is_recoverable_with_replay() {
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
	pub async fn run_activity<A: Activity>(
		&mut self,
		input: &A::Input,
		activity_id: &ActivityId,
	) -> WorkflowResult<A::Output> {
		let mut ctx = ActivityCtx::new(
			self.db.clone(),
			self.conn.clone(),
			self.workflow_id,
			A::name(),
		);

		match A::run(&mut ctx, input).await {
			Result::Ok(output) => {
				tracing::debug!("activity success");

				// Write output
				let input_str =
					serde_json::to_string(input).map_err(WorkflowError::SerializeActivityInput)?;
				let output_str = serde_json::to_string(&output)
					.map_err(WorkflowError::SerializeActivityOutput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						self.full_location().as_ref(),
						activity_id,
						&input_str,
						Some(&output_str),
					)
					.await?;

				Result::Ok(output)
			}
			Err(err) => {
				tracing::debug!(?err, "activity error");

				// Write empty output (failed state)
				let input_str =
					serde_json::to_string(input).map_err(WorkflowError::SerializeActivityInput)?;
				self.db
					.commit_workflow_activity_event(
						self.workflow_id,
						self.full_location().as_ref(),
						activity_id,
						&input_str,
						None,
					)
					.await?;

				// TODO: RVT-3751
				Err(WorkflowError::ActivityFailure(err))
			}
		}
	}

	/// Checks for a signal to this workflow with any of the given signal names. Meant to be implemented and
	/// not used directly in workflows.
	pub async fn listen_any(&mut self, signal_names: &[&'static str]) -> WorkflowResult<SignalRow> {
		// Fetch new pending signal
		let signal = self
			.db
			.pull_latest_signal(
				self.workflow_id,
				signal_names,
				self.full_location().as_ref(),
			)
			.await?;

		let Some(signal) = signal else {
			return Err(WorkflowError::NoSignalFound(Box::from(signal_names)));
		};

		tracing::info!(
			workflow_id=%self.workflow_id,
			signal_id=%signal.signal_id,
			name=%signal.signal_name,
			"signal received",
		);

		WorkflowResult::Ok(signal)
	}
}

impl WorkflowCtx {
	/// Dispatch another workflow.
	pub async fn dispatch_workflow<I>(&mut self, input: I) -> WorkflowResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let id = if let Some(event) = event {
			// Validate history is consistent
			let Event::SubWorkflow(sub_workflow) = event else {
				return Err(WorkflowError::HistoryDiverged);
			};

			if sub_workflow.sub_workflow_name != I::Workflow::name() {
				return Err(WorkflowError::HistoryDiverged);
			}

			tracing::debug!(
				id=%self.workflow_id,
				sub_workflow_id=%sub_workflow.sub_workflow_id,
				"replaying workflow dispatch"
			);

			sub_workflow.sub_workflow_id
		}
		// Dispatch new workflow
		else {
			let name = I::Workflow::name();

			tracing::debug!(%name, ?input, "dispatching workflow");

			let sub_workflow_id = Uuid::new_v4();

			// Serialize input
			let input_str =
				serde_json::to_string(&input).map_err(WorkflowError::SerializeWorkflowOutput)?;

			self.db
				.dispatch_sub_workflow(
					self.workflow_id,
					self.full_location().as_ref(),
					sub_workflow_id,
					&name,
					&input_str,
				)
				.await?;

			tracing::info!(%name, ?sub_workflow_id, "workflow dispatched");

			sub_workflow_id
		};

		// Move to next event
		self.location_idx += 1;

		WorkflowResult::Ok(id)
	}

	/// Wait for another workflow's response.
	pub async fn wait_for_workflow<W: Workflow>(
		&self,
		sub_workflow_id: Uuid,
	) -> WorkflowResult<W::Output> {
		tracing::info!(name = W::name(), ?sub_workflow_id, "waiting for workflow");

		let mut retries = 0;
		let mut interval = tokio::time::interval(SUB_WORKFLOW_RETRY);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;

			// Check if state finished
			let workflow = self
				.db
				.get_workflow(sub_workflow_id)
				.await?
				.ok_or(WorkflowError::WorkflowNotFound)?;

			if let Some(output) = workflow.parse_output::<W>()? {
				return WorkflowResult::Ok(output);
			} else {
				if retries > MAX_SUB_WORKFLOW_RETRIES {
					return Err(WorkflowError::SubWorkflowIncomplete(sub_workflow_id));
				}
				retries += 1;
			}
		}
	}

	// TODO(RVTEE-103): Run sub workflow inline as a branch of the parent workflow
	/// Trigger another workflow and wait for its response.
	pub async fn workflow<I>(
		&mut self,
		input: I,
	) -> WorkflowResult<<<I as WorkflowInput>::Workflow as Workflow>::Output>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let sub_workflow_id = self.dispatch_workflow(input).await?;
		let output = self
			.wait_for_workflow::<I::Workflow>(sub_workflow_id)
			.await?;
		WorkflowResult::Ok(output)
	}

	/// Run activity. Will replay on failure.
	pub async fn activity<I>(
		&mut self,
		input: I,
	) -> WorkflowResult<<<I as ActivityInput>::Activity as Activity>::Output>
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
				return Err(WorkflowError::HistoryDiverged);
			};

			if activity.activity_id != activity_id {
				return Err(WorkflowError::HistoryDiverged);
			}

			// Activity succeeded
			if let Some(output) = activity.get_output()? {
				output
			} else {
				// Activity failed, retry
				self.run_activity::<I::Activity>(&input, &activity_id)
					.await?
			}
		}
		// This is a new activity
		else {
			self.run_activity::<I::Activity>(&input, &activity_id)
				.await?
		};

		// Move to next event
		self.location_idx += 1;

		WorkflowResult::Ok(output)
	}

	/// Joins multiple executable actions (activities, closures) and awaits them simultaneously.
	pub async fn join<T: Executable>(&mut self, exec: T) -> WorkflowResult<T::Output> {
		exec.execute(self).await
	}

	/// Sends a signal.
	pub async fn signal<T: Signal + Serialize>(
		&mut self,
		workflow_id: Uuid,
		body: T,
	) -> WorkflowResult<Uuid> {
		let id = Uuid::new_v4();

		self.db
			.publish_signal(
				workflow_id,
				id,
				T::name(),
				&serde_json::to_string(&body).map_err(WorkflowError::SerializeSignalBody)?,
			)
			.await?;

		WorkflowResult::Ok(id)
	}

	/// Listens for a signal for a short time before setting the workflow to sleep. Once the signal is
	/// received, the workflow will be woken up and continue.
	pub async fn listen<T: Listen>(&mut self) -> WorkflowResult<T> {
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let signal = if let Some(event) = event {
			// Validate history is consistent
			let Event::Signal(signal) = event else {
				return Err(WorkflowError::HistoryDiverged);
			};

			tracing::debug!(id=%self.workflow_id, name=%signal.name, "replaying signal");

			T::parse(&signal.name, &signal.body)?
		}
		// Listen for new messages
		else {
			tracing::debug!(id=%self.workflow_id, "listening for signal");

			let mut retries = 0;
			let mut interval = tokio::time::interval(SIGNAL_RETRY);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;

				match T::listen(self).await {
					WorkflowResult::Ok(res) => break res,
					Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => {
						if retries > MAX_SIGNAL_RETRIES {
							return Err(err);
						}
						retries += 1;
					}
					err => return err,
				}
			}
		};

		// Move to next event
		self.location_idx += 1;

		WorkflowResult::Ok(signal)
	}

	/// Checks if the given signal exists in the database.
	pub async fn query_signal<T: Listen>(&mut self) -> WorkflowResult<Option<T>> {
		let event = { self.relevant_history().nth(self.location_idx) };

		// Signal received before
		let signal = if let Some(event) = event {
			tracing::debug!(id=%self.workflow_id, "replaying signal");

			// Validate history is consistent
			let Event::Signal(signal) = event else {
				return Err(WorkflowError::HistoryDiverged);
			};

			Some(T::parse(&signal.name, &signal.body)?)
		}
		// Listen for new message
		else {
			match T::listen(self).await {
				WorkflowResult::Ok(res) => Some(res),
				Err(err) if matches!(err, WorkflowError::NoSignalFound(_)) => None,
				Err(err) => return Err(err),
			}
		};

		// Move to next event
		self.location_idx += 1;

		WorkflowResult::Ok(signal)
	}

	// TODO: sleep_for, sleep_until
}
