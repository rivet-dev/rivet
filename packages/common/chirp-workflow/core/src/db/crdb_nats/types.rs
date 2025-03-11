use std::collections::HashMap;

use uuid::Uuid;

use crate::{
	db::{PulledWorkflowData, SignalData, WorkflowData},
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{
			ActivityEvent, Event, EventData, EventId, EventType, LoopEvent, MessageSendEvent,
			RemovedEvent, SignalEvent, SignalSendEvent, SleepEvent, SleepState, SubWorkflowEvent,
		},
		location::{Coordinate, Location},
	},
};

type RawJson = sqlx::types::Json<Box<serde_json::value::RawValue>>;

#[derive(sqlx::FromRow)]
pub struct WorkflowRow {
	workflow_id: Uuid,
	input: RawJson,
	output: Option<RawJson>,
	has_wake_condition: bool,
}

impl From<WorkflowRow> for WorkflowData {
	fn from(value: WorkflowRow) -> Self {
		WorkflowData {
			workflow_id: value.workflow_id,
			input: value.input.0,
			output: value.output.map(|x| x.0),
			has_wake_condition: value.has_wake_condition,
		}
	}
}

#[derive(sqlx::FromRow)]
pub struct PulledWorkflowRow {
	pub workflow_id: Uuid,
	workflow_name: String,
	create_ts: i64,
	ray_id: Uuid,
	input: RawJson,
	wake_deadline_ts: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct SignalRow {
	signal_id: Uuid,
	signal_name: String,
	body: RawJson,
	create_ts: i64,
}

impl From<SignalRow> for SignalData {
	fn from(value: SignalRow) -> Self {
		SignalData {
			signal_id: value.signal_id,
			signal_name: value.signal_name,
			body: value.body.0,
			create_ts: value.create_ts,
		}
	}
}

/// Stores data for all event types in one.
#[derive(Debug, sqlx::FromRow)]
pub struct AmalgamEventRow {
	workflow_id: Uuid,
	location: Vec<i64>,
	location2: Option<Location>,
	version: i64,
	event_type: i64,
	name: Option<String>,
	auxiliary_id: Option<Uuid>,
	hash: Option<Vec<u8>>,
	input: Option<RawJson>,
	output: Option<RawJson>,
	create_ts: Option<i64>,
	error_count: Option<i64>,
	iteration: Option<i64>,
	deadline_ts: Option<i64>,
	state: Option<i64>,
	inner_event_type: Option<i64>,
}

impl TryFrom<AmalgamEventRow> for Event {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		// Backwards compatibility
		let location_tail = value
			.location2
			.as_ref()
			.map(|x| x.tail().cloned().expect("empty location"))
			.unwrap_or_else(|| {
				Coordinate::simple(
					// NOTE: Add 1 because we switched from 0-based to 1-based
					*value.location.last().expect("empty location") as usize + 1,
				)
			});

		let event_type = value
			.event_type
			.try_into()
			.map_err(|_| WorkflowError::IntegerConversion)?;
		let event_type = EventType::from_repr(event_type)
			.ok_or_else(|| WorkflowError::InvalidEventType(value.event_type))?;

		Ok(Event {
			coordinate: location_tail,
			version: value
				.version
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
			data: match event_type {
				EventType::Activity => EventData::Activity(value.try_into()?),
				EventType::Signal => EventData::Signal(value.try_into()?),
				EventType::SignalSend => EventData::SignalSend(value.try_into()?),
				EventType::MessageSend => EventData::MessageSend(value.try_into()?),
				EventType::SubWorkflow => EventData::SubWorkflow(value.try_into()?),
				EventType::Loop => EventData::Loop(value.try_into()?),
				EventType::Sleep => EventData::Sleep(value.try_into()?),
				EventType::Branch => EventData::Branch,
				EventType::Removed => EventData::Removed(value.try_into()?),
				EventType::VersionCheck => EventData::VersionCheck,
			},
		})
	}
}

impl TryFrom<AmalgamEventRow> for ActivityEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(ActivityEvent {
			event_id: EventId::from_le_bytes(
				value.name.ok_or(WorkflowError::MissingEventData)?,
				value.hash.ok_or(WorkflowError::MissingEventData)?,
			)?,
			create_ts: value.create_ts.ok_or(WorkflowError::MissingEventData)?,
			output: value.output.map(|x| x.0),
			error_count: value
				.error_count
				.ok_or(WorkflowError::MissingEventData)?
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(SignalEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData)?,
			body: value
				.output
				.map(|x| x.0)
				.ok_or(WorkflowError::MissingEventData)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalSendEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(SignalSendEvent {
			signal_id: value.auxiliary_id.ok_or(WorkflowError::MissingEventData)?,
			name: value.name.ok_or(WorkflowError::MissingEventData)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for MessageSendEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(MessageSendEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SubWorkflowEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.auxiliary_id.ok_or(WorkflowError::MissingEventData)?,
			name: value.name.ok_or(WorkflowError::MissingEventData)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for LoopEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		Ok(LoopEvent {
			state: value.input.ok_or(WorkflowError::MissingEventData)?.0,
			output: value.output.map(|x| x.0),
			iteration: value
				.iteration
				.ok_or(WorkflowError::MissingEventData)?
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SleepEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		let state = value.state.ok_or(WorkflowError::MissingEventData)?;

		Ok(SleepEvent {
			deadline_ts: value.deadline_ts.ok_or(WorkflowError::MissingEventData)?,
			state: SleepState::from_repr(state.try_into()?)
				.ok_or_else(|| WorkflowError::InvalidSleepState(state))?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for RemovedEvent {
	type Error = WorkflowError;

	fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
		let event_type = value
			.inner_event_type
			.ok_or(WorkflowError::MissingEventData)?;

		Ok(RemovedEvent {
			name: value.name,
			event_type: EventType::from_repr(event_type.try_into()?)
				.ok_or_else(|| WorkflowError::InvalidEventType(event_type))?,
		})
	}
}

// IMPORTANT: Must match the hashing algorithm used in the `db-workflow` `loop_location2_hash` generated
/// column expression.
pub fn hash_location(location: &Location) -> Vec<u8> {
	let mut s = "[".to_string();

	let mut loc_iter = location.iter();

	// First coord
	if let Some(coord) = loc_iter.next() {
		let mut coord_iter = coord.iter();

		s.push_str("[");

		// First part
		if let Some(part) = coord_iter.next() {
			s.push_str(&part.to_string());
		}

		// Rest
		for part in coord_iter {
			// NOTE: The space here is important as it mimics the default behavior of casting JSONB to
			// TEXT in CRDB.
			s.push_str(", ");
			s.push_str(&part.to_string());
		}

		s.push_str("]");
	}

	// Rest
	for coord in loc_iter {
		// NOTE: The space here is important as it mimics the default behavior of casting JSONB to
		// TEXT in CRDB.
		s.push_str(", ");

		let mut coord_iter = coord.iter();

		s.push_str("[");

		// First part
		if let Some(part) = coord_iter.next() {
			s.push_str(&part.to_string());
		}

		// Rest
		for part in coord_iter {
			// NOTE: The space here is important as it mimics the default behavior of casting JSONB to
			// TEXT in CRDB.
			s.push_str(", ");
			s.push_str(&part.to_string());
		}

		s.push_str("]");
	}

	s.push_str("]");

	md5::compute(s).to_vec()
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
pub fn build_histories(
	workflow_rows: Vec<PulledWorkflowRow>,
	event_rows: Vec<AmalgamEventRow>,
) -> WorkflowResult<Vec<PulledWorkflowData>> {
	// Map workflow rows by workflow id
	let mut workflows_by_id = workflow_rows
		.into_iter()
		.map(|row| {
			let events_by_location: HashMap<Location, Vec<Event>> = HashMap::new();

			(row.workflow_id, (row, events_by_location))
		})
		.collect::<HashMap<_, _>>();

	for event_row in event_rows {
		// Backwards compatibility
		let location_root = event_row
			.location2
			.as_ref()
			.map(|x| x.root())
			.unwrap_or_else(|| {
				event_row
					.location
					.iter()
					.take(event_row.location.len().saturating_sub(1))
					// NOTE: Add 1 because we switched from 0-based to 1-based
					.map(|x| Coordinate::simple((*x) as usize + 1))
					.collect()
			});

		// Get workflow entry
		let (_, ref mut events_by_location) = workflows_by_id
			.get_mut(&event_row.workflow_id)
			.expect("unreachable, workflow for event not found");

		events_by_location
			.entry(location_root)
			.or_default()
			.push(event_row.try_into()?);
	}

	let workflows = workflows_by_id
		.into_values()
		.map(|(row, mut events_by_location)| {
			for events in events_by_location.values_mut() {
				// Events are already mostly sorted themselves so this should be fairly cheap
				events.sort_by_key(|event| event.coordinate().clone());

				// NOTE: The following is only for the purpose of backwards compatibility for workflows
				// that were created before the branch event was formally added.

				// This if statement handles the side effect of inserting a large amount of useless
				// `Empty` placeholders in loops. Because the direct children of a loop are only ever
				// branches, we skip inserting `Empty` placeholders if there are only branches in the
				// events list
				if !events.iter().all(|e| matches!(e.data, EventData::Branch)) {
					// Check for missing indexes and insert a `Empty` placeholder event for each missing
					// spot.
					let mut last_coord = Coordinate::simple(0);
					*events = events
						.drain(..)
						.flat_map(|event| {
							let last = last_coord.head();
							let curr = event.coordinate.head();
							assert!(last <= curr, "invalid history");

							let offset = (curr - last).saturating_sub(1);

							last_coord = event.coordinate.clone();

							(1..=offset)
								.map(move |i| Event {
									coordinate: Coordinate::simple(last + i),
									version: 0,
									data: EventData::Empty,
								})
								.chain(std::iter::once(event))
						})
						.collect();
				}
			}

			PulledWorkflowData {
				workflow_id: row.workflow_id,
				workflow_name: row.workflow_name,
				create_ts: row.create_ts,
				ray_id: row.ray_id,
				input: row.input.0,
				events: events_by_location,
				wake_deadline_ts: row.wake_deadline_ts,
			}
		})
		.collect();

	Ok(workflows)
}
