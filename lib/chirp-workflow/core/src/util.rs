use std::{
	collections::HashMap,
	time::{SystemTime, UNIX_EPOCH},
};

use global_error::{macros::*, GlobalError, GlobalResult};
use rand::Rng;
use tokio::time::{self, Duration};
use uuid::Uuid;

use crate::{
	error::WorkflowError, event::Event, ActivityEventRow, SignalEventRow, SubWorkflowEventRow,
	WorkflowResult,
};

pub type Location = Box<[usize]>;

// How often the `inject_fault` function fails in percent
const FAULT_RATE: usize = 80;

/// Allows for checking if a global error returned from an activity is recoverable.
pub trait GlobalErrorExt {
	fn is_workflow_recoverable(&self) -> bool;
}

impl GlobalErrorExt for GlobalError {
	fn is_workflow_recoverable(&self) -> bool {
		match self {
			GlobalError::Raw(inner_err) => inner_err
				.downcast_ref::<WorkflowError>()
				.map(|err| err.is_recoverable())
				.unwrap_or_default(),
			_ => false,
		}
	}
}

impl<T> GlobalErrorExt for GlobalResult<T> {
	fn is_workflow_recoverable(&self) -> bool {
		match self {
			Err(GlobalError::Raw(inner_err)) => inner_err
				.downcast_ref::<WorkflowError>()
				.map(|err| err.is_recoverable())
				.unwrap_or_default(),
			_ => false,
		}
	}
}

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
	let mut events_by_location: HashMap<Location, Vec<(i64, Event)>> = HashMap::new();

	for event in activity_events {
		let (root_location, idx) = split_location(&event.location);

		insert_placeholder(&mut events_by_location, &event.location, idx);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Activity(event.try_into()?)));
	}

	for event in signal_events {
		let (root_location, idx) = split_location(&event.location);

		insert_placeholder(&mut events_by_location, &event.location, idx);

		events_by_location
			.entry(root_location)
			.or_default()
			.push((idx, Event::Signal(event.try_into()?)));
	}

	for event in sub_workflow_events {
		let (root_location, idx) = split_location(&event.location);

		insert_placeholder(&mut events_by_location, &event.location, idx);

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

// Insert placeholder record into parent location list (ex. for [4, 0] insert into the [] list at
// the 4th index)
fn insert_placeholder(
	events_by_location: &mut HashMap<Location, Vec<(i64, Event)>>,
	location: &[i64],
	idx: i64,
) {
	if idx == 0 && location.len() > 1 {
		let parent_location = location
			.iter()
			.take(location.len().saturating_sub(2))
			.map(|x| *x as usize)
			.collect::<Location>();
		let parent_idx = *location.get(location.len().saturating_sub(2)).unwrap();

		events_by_location
			.entry(parent_location)
			.or_default()
			.push((parent_idx, Event::Branch));
	}
}

pub fn inject_fault() -> GlobalResult<()> {
	if rand::thread_rng().gen_range(0..100) < FAULT_RATE {
		bail!("This is a random panic!");
	}

	Ok(())
}

pub(crate) fn new_conn(
	shared_client: &chirp_client::SharedClientHandle,
	pools: &rivet_pools::Pools,
	cache: &rivet_cache::Cache,
	ray_id: Uuid,
	req_id: Uuid,
	name: &str,
) -> rivet_connection::Connection {
	let client = shared_client.clone().wrap(
		req_id,
		ray_id,
		vec![chirp_client::TraceEntry {
			context_name: name.into(),
			req_id: Some(req_id.into()),
			ts: rivet_util::timestamp::now(),
			run_context: match rivet_util::env::run_context() {
				rivet_util::env::RunContext::Service => chirp_client::RunContext::Service,
				rivet_util::env::RunContext::Test => chirp_client::RunContext::Test,
			} as i32,
		}],
	);

	rivet_connection::Connection::new(client, pools.clone(), cache.clone())
}
