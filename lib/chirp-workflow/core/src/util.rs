use std::{
	collections::HashMap,
	time::{SystemTime, UNIX_EPOCH},
};

use global_error::{macros::*, GlobalResult};
use rand::Rng;
use tokio::time::{self, Duration};

use crate::{schema::Event, ActivityEventRow, SignalEventRow, SubWorkflowEventRow, WorkflowResult};

pub type Location = Box<[usize]>;

// How often the `inject_fault` function fails in percent
const FAULT_RATE: usize = 80;

pub async fn sleep_until_ts(ts: i64) {
	let target_time = UNIX_EPOCH + Duration::from_millis(ts as u64);
	if let Ok(sleep_duration) = target_time.duration_since(SystemTime::now()) {
		time::sleep(sleep_duration).await;
	}
}

/// Takes activity, signal, and sub workflow events (each with their own location) and combines them via enum
/// into a hashmap of the following structure:
///
/// Given the location [1, 2, 3], 3 is the index and [1, 2] is the root location
///
/// HashMap {
/// 	[1, 2]: [
/// 		example signal event,
/// 		example activity event,
/// 		example activity event (this is [1, 2, 3])
/// 	],
/// }
pub fn combine_events(
	activity_events: Vec<ActivityEventRow>,
	signal_events: Vec<SignalEventRow>,
	sub_workflow_events: Vec<SubWorkflowEventRow>,
) -> WorkflowResult<HashMap<Location, Vec<Event>>> {
	let mut events_by_location: HashMap<_, Vec<(i64, Event)>> = HashMap::new();

	for event in activity_events {
		let root_location = event
			.location
			.iter()
			.take(event.location.len().saturating_sub(1))
			.map(|x| *x as usize)
			.collect::<Location>();
		let idx = *event.location.last().unwrap();

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Activity(event.try_into()?)));
	}

	for event in signal_events {
		let root_location = event
			.location
			.iter()
			.take(event.location.len().saturating_sub(1))
			.map(|x| *x as usize)
			.collect::<Location>();
		let idx = *event.location.last().unwrap();

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Signal(event.try_into()?)));
	}

	for event in sub_workflow_events {
		let root_location = event
			.location
			.iter()
			.take(event.location.len().saturating_sub(1))
			.map(|x| *x as usize)
			.collect::<Location>();
		let idx = *event.location.last().unwrap();

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::SubWorkflow(event.try_into()?)));
	}

	// TODO(RVT-3754): This involves inserting, sorting, then recollecting into lists and recollecting into a
	// hashmap. Could be improved by iterating over both lists simultaneously and sorting each item at a
	// time before inserting
	// Sort all of the events because we are inserting from two different lists. Both lists are already
	// sorted themselves so this should be fairly cheap
	for (_, list) in events_by_location.iter_mut() {
		list.sort_by_key(|(idx, _)| *idx);
	}

	// Remove idx from lists
	let event_history = events_by_location
		.into_iter()
		.map(|(k, v)| (k, v.into_iter().map(|(_, v)| v).collect()))
		.collect();

	Ok(event_history)
}

pub fn inject_fault() -> GlobalResult<()> {
	if rand::thread_rng().gen_range(0..100) < FAULT_RATE {
		bail!("This is a random panic!");
	}

	Ok(())
}
