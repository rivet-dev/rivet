use std::collections::HashMap;

use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
	activity::ActivityId,
	db::{
		ActivityEventRow, LoopEventRow, MessageSendEventRow, PulledWorkflow, PulledWorkflowRow,
		SignalEventRow, SignalSendEventRow, SubWorkflowEventRow,
	},
	error::{WorkflowError, WorkflowResult},
	util::Location,
};

/// An event that happened in the workflow run.
///
/// This is used to replay events.
#[derive(Debug)]
pub enum Event {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SignalSend(SignalSendEvent),
	MessageSend(MessageSendEvent),
	SubWorkflow(SubWorkflowEvent),
	Loop(LoopEvent),
	// Used as a placeholder for branching locations
	Branch,
}

impl std::fmt::Display for Event {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Event::Activity(activity) => write!(f, "activity {:?}", activity.activity_id.name),
			Event::Signal(signal) => write!(f, "signal {:?}", signal.name),
			Event::SignalSend(signal_send) => write!(f, "signal send {:?}", signal_send.name),
			Event::MessageSend(message_send) => write!(f, "message send {:?}", message_send.name),
			Event::SubWorkflow(sub_workflow) => write!(f, "sub workflow {:?}", sub_workflow.name),
			Event::Loop(_) => write!(f, "loop"),
			Event::Branch => write!(f, "branch"),
		}
	}
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub activity_id: ActivityId,
	pub create_ts: i64,

	/// If the activity succeeds, this will be some.
	pub(crate) output: Option<serde_json::Value>,
	pub error_count: usize,
}

impl ActivityEvent {
	pub fn parse_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
		self.output
			.clone()
			.map(serde_json::from_value)
			.transpose()
			.map_err(WorkflowError::DeserializeActivityOutput)
	}
}

impl TryFrom<ActivityEventRow> for ActivityEvent {
	type Error = WorkflowError;

	fn try_from(value: ActivityEventRow) -> WorkflowResult<Self> {
		Ok(ActivityEvent {
			activity_id: ActivityId::from_bytes(value.activity_name, value.input_hash)?,
			create_ts: value.create_ts,
			output: value.output,
			error_count: value
				.error_count
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

#[derive(Debug)]
pub struct SignalEvent {
	pub name: String,
	pub body: serde_json::Value,
}

impl TryFrom<SignalEventRow> for SignalEvent {
	type Error = WorkflowError;

	fn try_from(value: SignalEventRow) -> WorkflowResult<Self> {
		Ok(SignalEvent {
			name: value.signal_name,
			body: value.body,
		})
	}
}

#[derive(Debug)]
pub struct SignalSendEvent {
	pub signal_id: Uuid,
	pub name: String,
}

impl TryFrom<SignalSendEventRow> for SignalSendEvent {
	type Error = WorkflowError;

	fn try_from(value: SignalSendEventRow) -> WorkflowResult<Self> {
		Ok(SignalSendEvent {
			signal_id: value.signal_id,
			name: value.signal_name,
		})
	}
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
}

impl TryFrom<MessageSendEventRow> for MessageSendEvent {
	type Error = WorkflowError;

	fn try_from(value: MessageSendEventRow) -> WorkflowResult<Self> {
		Ok(MessageSendEvent {
			name: value.message_name,
		})
	}
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Uuid,
	pub name: String,
}

impl TryFrom<SubWorkflowEventRow> for SubWorkflowEvent {
	type Error = WorkflowError;

	fn try_from(value: SubWorkflowEventRow) -> WorkflowResult<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.sub_workflow_id,
			name: value.sub_workflow_name,
		})
	}
}

#[derive(Debug)]
pub struct LoopEvent {
	/// If the loop completes, this will be some.
	pub(crate) output: Option<serde_json::Value>,
	pub iteration: usize,
}

impl LoopEvent {
	pub fn parse_output<O: DeserializeOwned>(&self) -> WorkflowResult<Option<O>> {
		self.output
			.clone()
			.map(serde_json::from_value)
			.transpose()
			.map_err(WorkflowError::DeserializeLoopOutput)
	}
}

impl TryFrom<LoopEventRow> for LoopEvent {
	type Error = WorkflowError;

	fn try_from(value: LoopEventRow) -> WorkflowResult<Self> {
		Ok(LoopEvent {
			output: value.output,
			iteration: value
				.iteration
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

/// Takes all workflow events (each with their own location) and combines them via enum into a hashmap of the
/// following structure:
///
/// Given the location [1, 2, 3], 3 is the index and [1, 2] is the root location
///
/// HashMap {
/// 	[1, 2]: [
/// 		example signal event,
/// 		example activity event,
/// 		example sub workflow event,
/// 		example activity event (this is [1, 2, 3])
/// 	],
/// }
pub fn combine_events(
	workflow_rows: Vec<PulledWorkflowRow>,
	activity_events: Vec<ActivityEventRow>,
	signal_events: Vec<SignalEventRow>,
	signal_send_events: Vec<SignalSendEventRow>,
	msg_send_events: Vec<MessageSendEventRow>,
	sub_workflow_events: Vec<SubWorkflowEventRow>,
	loop_events: Vec<LoopEventRow>,
) -> WorkflowResult<Vec<PulledWorkflow>> {
	// Map workflow rows by workflow id
	let mut workflows_by_id = workflow_rows
		.into_iter()
		.map(|row| {
			let events_by_location: HashMap<Location, Vec<(i64, Event)>> = HashMap::new();

			(row.workflow_id, (row, events_by_location))
		})
		.collect::<HashMap<_, _>>();

	for event in activity_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Activity(event.try_into()?)));
	}

	for event in signal_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Signal(event.try_into()?)));
	}

	for event in signal_send_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::SignalSend(event.try_into()?)));
	}

	for event in msg_send_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::MessageSend(event.try_into()?)));
	}

	for event in sub_workflow_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::SubWorkflow(event.try_into()?)));
	}

	for event in loop_events {
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event.workflow_id)
			.expect("unreachable, workflow for event not found");
		let (root_location, idx) = split_location(&event.location);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Loop(event.try_into()?)));
	}

	let workflows = workflows_by_id
		.into_values()
		.map(|(row, mut events_by_location)| {
			// TODO(RVT-3754): This involves inserting, sorting, then recollecting into lists and recollecting
			// into a hashmap
			// Sort all of the events because we are inserting from two different lists. Both lists are
			// already sorted themselves so this should be fairly cheap
			for (_, list) in events_by_location.iter_mut() {
				list.sort_by_key(|(idx, _)| *idx);
			}

			// Remove idx from lists
			let event_history = events_by_location
				.into_iter()
				.map(|(k, events)| {
					let mut expected_idx = 0;

					// Check for missing indexes and insert a `Branch` placeholder event for each missing spot
					let events = events
						.into_iter()
						.flat_map(|(idx, v)| {
							assert!(expected_idx <= idx, "invalid history");

							let offset = (idx - expected_idx) as usize;
							expected_idx = idx + 1;

							std::iter::repeat_with(|| Event::Branch)
								.take(offset)
								.chain(std::iter::once(v))
						})
						.collect();

					(k, events)
				})
				.collect();

			PulledWorkflow {
				workflow_id: row.workflow_id,
				workflow_name: row.workflow_name,
				create_ts: row.create_ts,
				ray_id: row.ray_id,
				input: row.input,
				wake_deadline_ts: row.wake_deadline_ts,
				events: event_history,
			}
		})
		.collect();

	Ok(workflows)
}

fn split_location(location: &[i64]) -> (Location, i64) {
	(
		location
			.iter()
			.take(location.len().saturating_sub(1))
			.map(|x| *x as usize)
			.collect::<Location>(),
		*location.last().unwrap(),
	)
}
