use anyhow::*;
use chirp_workflow::history::{
	event::{EventType, RemovedEvent, SleepEvent, SleepState},
	location::{Coordinate, Location},
};
use rivet_term::console::Style;
use uuid::Uuid;

#[derive(Debug)]
pub struct Event {
	pub location: Location,
	pub version: usize,
	pub forgotten: bool,
	pub data: EventData,
}

impl Event {
	pub fn style(&self) -> Style {
		match &self.data {
			EventData::Activity(_) => Style::new().yellow(),
			EventData::Signal(_) => Style::new().cyan(),
			EventData::SignalSend(_) => Style::new().bright().blue(),
			EventData::MessageSend(_) => Style::new().bright().blue(),
			EventData::SubWorkflow(_) => Style::new().green(),
			EventData::Loop(_) => Style::new().magenta(),
			EventData::Sleep(_) => Style::new().magenta(),
			EventData::Removed(_) => Style::new().red(),
			EventData::VersionCheck => Style::new().red(),
			EventData::Branch => Style::new(),
			EventData::Empty => Style::new(),
		}
	}

	pub fn print_name(&self) {
		let style = if self.forgotten {
			Style::new().red().dim()
		} else {
			self.style()
		};

		match &self.data {
			EventData::Activity(activity) => print!(
				"{} {}",
				style.apply_to("activity").bold(),
				style.apply_to(&activity.name)
			),
			EventData::Signal(signal) => print!(
				"{} {}",
				style.apply_to("signal receive").bold(),
				style.apply_to(&signal.name)
			),
			EventData::SignalSend(signal_send) => print!(
				"{} {}",
				style.apply_to("signal send").bold(),
				style.apply_to(&signal_send.name)
			),
			EventData::MessageSend(message_send) => print!(
				"{} {}",
				style.apply_to("message send").bold(),
				style.apply_to(&message_send.name)
			),
			EventData::SubWorkflow(sub_workflow) => print!(
				"{} {}",
				style.apply_to("sub workflow").bold(),
				style.apply_to(&sub_workflow.name)
			),
			EventData::Loop(_) => print!("{}", style.apply_to("loop").bold()),
			EventData::Sleep(_) => print!("{}", style.apply_to("sleep").bold()),
			EventData::Removed(removed) => {
				if let Some(name) = &removed.name {
					print!(
						"{} {}",
						style
							.apply_to(format!("removed {}", removed.event_type))
							.bold(),
						style.apply_to(name)
					)
				} else {
					print!(
						"{}",
						style
							.apply_to(format!("removed {}", removed.event_type))
							.bold()
					)
				}
			}
			EventData::VersionCheck => print!("{}", style.apply_to("version check").bold()),
			EventData::Branch => print!("{}", style.apply_to("branch").bold()),
			EventData::Empty => print!("{}", style.apply_to("empty").bold()),
		}
	}
}

#[derive(Debug)]
pub enum EventData {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SignalSend(SignalSendEvent),
	MessageSend(MessageSendEvent),
	SubWorkflow(SubWorkflowEvent),
	Loop(LoopEvent),
	Sleep(SleepEvent),
	Removed(RemovedEvent),
	VersionCheck,
	Branch,

	/// NOTE: Strictly used as a placeholder for backfilling. When using this, the coordinate of the `Event`
	/// must still be valid.
	Empty,
}

impl std::fmt::Display for EventData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self {
			EventData::Activity(activity) => write!(f, "activity {}", activity.name),
			EventData::Signal(signal) => write!(f, "signal receive {}", signal.name),
			EventData::SignalSend(signal_send) => write!(f, "signal send {}", signal_send.name),
			EventData::MessageSend(message_send) => {
				write!(f, "message send {}", message_send.name)
			}
			EventData::SubWorkflow(sub_workflow) => {
				write!(f, "sub workflow {}", sub_workflow.name)
			}
			EventData::Loop(_) => write!(f, "loop"),
			EventData::Sleep(_) => write!(f, "sleep"),
			EventData::Removed(removed) => {
				if let Some(name) = &removed.name {
					write!(f, "removed {} {name}", removed.event_type)
				} else {
					write!(f, "removed {}", removed.event_type)
				}
			}
			EventData::VersionCheck => write!(f, "version check"),
			EventData::Branch => write!(f, "branch"),
			EventData::Empty => write!(f, "empty"),
		}
	}
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub name: String,
	pub input: serde_json::Value,
	pub output: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct SignalEvent {
	pub signal_id: Uuid,
	pub name: String,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SignalSendEvent {
	pub signal_id: Uuid,
	pub name: String,
	pub tags: Option<serde_json::Value>,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
	pub tags: serde_json::Value,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Uuid,
	pub name: String,
	pub tags: serde_json::Value,
	pub input: serde_json::Value,
	// pub output: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct LoopEvent {
	pub state: serde_json::Value,
	/// If the loop completes, this will be some.
	pub output: Option<serde_json::Value>,
	pub iteration: usize,
}

#[derive(sqlx::FromRow)]
pub struct AmalgamEventRow {
	location: Vec<i64>,
	location2: Option<Location>,
	tags: Option<serde_json::Value>,
	version: i64,
	event_type: i64,
	name: Option<String>,
	auxiliary_id: Option<Uuid>,
	input: Option<serde_json::Value>,
	output: Option<serde_json::Value>,
	iteration: Option<i64>,
	deadline_ts: Option<i64>,
	state: Option<i64>,
	inner_event_type: Option<i64>,
	forgotten: bool,
}

impl TryFrom<AmalgamEventRow> for Event {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		// Backwards compatibility
		let location = value.location2.clone().unwrap_or_else(|| {
			// NOTE: Add 1 because we switched from 0-based to 1-based
			value
				.location
				.iter()
				.map(|x| Coordinate::simple(*x as usize + 1))
				.collect()
		});

		let event_type = value.event_type.try_into().context("integer conversion")?;
		let event_type = EventType::from_repr(event_type)
			.with_context(|| format!("invalid event type: {}", value.event_type))?;

		Ok(Event {
			location,
			version: value.version.try_into().context("integer conversion")?,
			forgotten: value.forgotten,
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
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(ActivityEvent {
			name: value.name.context("missing event data")?,
			input: value.input.context("missing event data")?,
			output: value.output,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SignalEvent {
			signal_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			body: value.output.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalSendEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SignalSendEvent {
			signal_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			tags: value.tags,
			body: value.input.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for MessageSendEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(MessageSendEvent {
			name: value.name.context("missing event data")?,
			tags: value.tags.context("missing event data")?,
			body: value.input.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SubWorkflowEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			tags: value.tags.context("missing event data")?,
			input: value.input.context("missing event data")?,
			// output: value.output,
		})
	}
}

impl TryFrom<AmalgamEventRow> for LoopEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(LoopEvent {
			state: value.input.context("missing event data")?,
			output: value.output,
			iteration: value
				.iteration
				.context("missing event data")?
				.try_into()
				.context("integer conversion")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SleepEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		let state = value.state.context("missing event data")?;

		Ok(SleepEvent {
			deadline_ts: value.deadline_ts.context("missing event data")?,
			state: SleepState::from_repr(state.try_into()?)
				.with_context(|| format!("invalid sleep state type: {}", state))?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for RemovedEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		let event_type = value.inner_event_type.context("missing event data")?;

		Ok(RemovedEvent {
			name: value.name,
			event_type: EventType::from_repr(event_type.try_into()?)
				.with_context(|| format!("invalid event type: {}", event_type))?,
		})
	}
}

pub fn build(event_rows: Vec<AmalgamEventRow>) -> Result<Vec<Event>> {
	let mut events = event_rows
		.into_iter()
		.map(TryInto::<Event>::try_into)
		.collect::<Result<Vec<_>>>()?;

	// Events are already mostly sorted themselves so this should be fairly cheap
	events.sort_by_key(|event| event.location.clone());

	// NOTE: The following is only for the purpose of backwards compatibility for workflows
	// that were created before the branch event was formally added.

	// Check for missing indexes and insert a `Empty` placeholder event for each missing
	// spot.
	let mut last_location = Location::new(Box::new([Coordinate::simple(0)]));
	let mut skip = false;
	events = events
		.drain(..)
		.flat_map(|event| {
			let last_coord_head = if last_location.root().len() < event.location.root().len() {
				0
			} else {
				// Get common root via cardinality
				last_location
					.iter()
					.take(last_location.len().min(event.location.len()))
					.cloned()
					.collect::<Location>()
					.tail()
					.expect("empty location")
					.head()
			};
			let curr_coord_head = event.location.tail().expect("empty location").head();

			// assert!(last_coord_head <= curr_coord_head, "invalid history");
			if last_coord_head > curr_coord_head {
				tracing::error!("============ THIS WORKFLOW HAS INVALID HISTORY ============");
			}

			let offset = if skip {
				0
			} else {
				(curr_coord_head - last_coord_head).saturating_sub(1)
			};

			last_location = event.location.clone();
			// Skip the next empty section for a loop
			skip = matches!(event.data, EventData::Loop(_));

			let root = event.location.root();

			(1..=offset)
				.map(move |i| Event {
					location: root.join(Coordinate::simple(last_coord_head + i)),
					version: 0,
					forgotten: false,
					data: EventData::Empty,
				})
				.chain(std::iter::once(event))
		})
		.collect();

	Ok(events)
}
