use super::{
	event::{
		ActivityEvent, Event, EventData, EventId, EventType, LoopEvent, MessageSendEvent,
		SignalEvent, SignalSendEvent, SleepEvent, SubWorkflowEvent,
	},
	location::{Coordinate, Location},
	removed::Removed,
	History,
};
use crate::error::{WorkflowError, WorkflowResult};

// NOTE: Cheap to clone because History is an `Arc`
/// Allows history traversal and comparison. This does not modify any history throughout the runtime of
/// the workflow.
#[derive(Clone)]
pub struct Cursor {
	events: History,
	root_location: Location,
	iter_idx: usize,

	prev_coord: Coordinate,
}

impl Cursor {
	pub(crate) fn new(events: History, root_location: Location) -> Self {
		Cursor {
			events,
			root_location,
			iter_idx: 0,

			// This is the only place a coordinate of `0` can exist. It is used as a left-most bound for
			// coordinates; no coordinates can come before 0.
			prev_coord: Coordinate::new(Box::new([0])),
		}
	}

	pub(crate) fn iter_idx(&self) -> usize {
		self.iter_idx
	}

	pub(crate) fn set_idx(&mut self, iter_idx: usize) {
		self.iter_idx = iter_idx;
	}

	pub fn current_coord(&self) -> Coordinate {
		self.coord_at(self.iter_idx)
	}

	fn coord_at(&self, idx: usize) -> Coordinate {
		let empty_vec = Vec::new();
		let branch = self.events.get(&self.root_location).unwrap_or(&empty_vec);

		// If event exists at the given index, return its coordinate
		if let Some(event) = branch.get(idx) {
			event.coordinate().clone()
		} else {
			// Otherwise, return the head int of the last event's coordinate and add the offset between
			// the iter idx and the end of the list
			let int = if let Some(last_event) = branch.last() {
				let head = last_event.coordinate().head();

				head + idx - branch.len() + 1
			} else {
				// IMPORTANT: We start at 1 so that we can apply version changes before the first event by
				// assigning them a coordinate starting with 0

				// No events, just use iter idx as the coord
				idx + 1
			};

			Coordinate::new(Box::new([int]))
		}
	}

	pub fn root(&self) -> &Location {
		&self.root_location
	}

	pub fn current_location(&self) -> Location {
		self.root_location
			.iter()
			.cloned()
			.chain(std::iter::once(self.current_coord()))
			.collect()
	}

	/// Returns the current location based on the history result of a comparison. The returned location
	/// should be used for the next inserted/compared workflow step/event.
	pub(crate) fn current_location_for<T>(&self, history_res: &HistoryResult<T>) -> Location {
		let curr = self.current_coord();

		let coord = match history_res {
			HistoryResult::Event(_) | HistoryResult::New => curr,
			// Pick a location between the previous and current location based on coordinate and version
			HistoryResult::Insertion => {
				// The difference between these two is `historical_prev` will always come from history,
				// whereas `prev` might be the last returned value from this function (not in history)
				let historical_prev = if self.iter_idx == 0 {
					Coordinate::new(Box::new([0]))
				} else {
					self.coord_at(self.iter_idx - 1)
				};
				let prev = &self.prev_coord;

				if &historical_prev == prev {
					// 1.1 vs 1.1.1 (cardinality)
					if prev.cardinality() >= curr.cardinality() {
						// prev + .1
						prev.into_iter()
							.cloned()
							.chain(std::iter::once(1))
							.collect::<Coordinate>()
					} else {
						// prev + .0.1
						prev.into_iter()
							.cloned()
							.chain(std::iter::once(0))
							.chain(std::iter::once(1))
							.collect::<Coordinate>()
					}
				} else {
					// Increment tail (1.2 -> 1.3)
					prev.with_tail(prev.tail() + 1)
				}
			}
		};

		self.root_location
			.iter()
			.cloned()
			.chain(std::iter::once(coord))
			.collect()
	}

	pub fn current_event(&self) -> Option<&Event> {
		if let Some(branch) = self.events.get(&self.root_location) {
			branch.get(self.iter_idx)
		} else {
			None
		}
	}

	pub(crate) fn inc(&mut self) {
		self.iter_idx += 1;
	}

	/// Advances the cursor based on the given location (should come from `Cursor::current_location_for`).
	pub(crate) fn update(&mut self, location: &Location) {
		let tail = location.tail().expect("empty location");

		// The location passed to this function should be the same one returned by
		// `Cursor::current_location_for`. Therefore, if it matches the current coord, it must be that the
		// last history result was a `HistoryResult::Event` and thus we must increment the cursor to move to
		// the next event. Otherwise, `Cursor::current_location_for` returned an inserted version which does
		// not constitute incrementing the cursor (as it only acts on history).
		if tail == &self.current_coord() {
			self.inc();
		}

		self.prev_coord = tail.clone();
	}
}

impl Cursor {
	/// Checks that there are no more events in the history.
	pub(crate) fn check_clear(&self) -> WorkflowResult<()> {
		let empty_vec = Vec::new();
		let branch = self.events.get(&self.root_location).unwrap_or(&empty_vec);

		if self.iter_idx < branch.len() {
			let latent = branch.len() - self.iter_idx;
			return Err(WorkflowError::LatentHistoryFound(format!(
				"expected {latent} more event{}",
				if latent == 1 { "s" } else { "" }
			)));
		};

		Ok(())
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_activity(
		&self,
		version: usize,
		event_id: &EventId,
	) -> WorkflowResult<HistoryResult<&ActivityEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of activity {} before {event} at {} (invalid due to versions: v{} < v{})",
					event_id.name,
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::Activity(activity) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found activity {}",
					self.current_location(),
					event_id.name
				)));
			};

			if &activity.event_id != event_id {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected activity {}#{:x} at {}, found activity {}#{:x}",
					activity.event_id.name,
					activity.event_id.input_hash,
					self.current_location(),
					event_id.name,
					event_id.input_hash,
				)));
			}

			Ok(HistoryResult::Event(activity))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_msg(
		&self,
		version: usize,
		msg_name: &str,
	) -> WorkflowResult<HistoryResult<&MessageSendEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of message send {} before {event} at {} (invalid due to versions: v{} < v{})",
					msg_name,
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::MessageSend(msg) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found message send {}",
					self.current_location(),
					msg_name,
				)));
			};

			if msg.name != msg_name {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found message send {}",
					self.current_location(),
					msg_name,
				)));
			}

			Ok(HistoryResult::Event(msg))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_signal_send(
		&self,
		version: usize,
		signal_name: &str,
	) -> WorkflowResult<HistoryResult<&SignalSendEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of signal send {} before {event} at {} (invalid due to versions: v{} < v{})",
					signal_name,
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::SignalSend(signal) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found signal send {}",
					self.current_location(),
					signal_name,
				)));
			};

			if signal.name != signal_name {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found signal send {}",
					self.current_location(),
					signal_name,
				)));
			}

			Ok(HistoryResult::Event(signal))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_sub_workflow(
		&self,
		version: usize,
		sub_workflow_name: &str,
	) -> WorkflowResult<HistoryResult<&SubWorkflowEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of sub workflow {} before {event} at {} (invalid due to versions: v{} < v{})",
					sub_workflow_name,
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::SubWorkflow(sub_workflow) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found sub workflow {}",
					self.current_location(),
					sub_workflow_name,
				)));
			};

			if sub_workflow.name != sub_workflow_name {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found sub_workflow {}",
					self.current_location(),
					sub_workflow_name,
				)));
			}

			Ok(HistoryResult::Event(sub_workflow))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_signal(&self, version: usize) -> WorkflowResult<HistoryResult<&SignalEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of signal before {event} at {} (invalid due to versions: v{} < v{})",
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::Signal(signal) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found signal",
					self.current_location(),
				)));
			};

			Ok(HistoryResult::Event(signal))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_loop(&self, version: usize) -> WorkflowResult<HistoryResult<&LoopEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of loop before {event} at {} (invalid due to versions: v{} < v{})",
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::Loop(loop_event) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found loop",
					self.current_location(),
				)));
			};

			Ok(HistoryResult::Event(loop_event))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_sleep(&self, version: usize) -> WorkflowResult<HistoryResult<&SleepEvent>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of sleep before {event} at {} (invalid due to versions: v{} < v{})",
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::Sleep(sleep) = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found sleep",
					self.current_location(),
				)));
			};

			Ok(HistoryResult::Event(sleep))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `true` if the current event is a replay.
	pub fn compare_branch(&self, version: usize) -> WorkflowResult<HistoryResult<()>> {
		if let Some(event) = self.current_event() {
			if event.version < version {
				return Err(WorkflowError::HistoryDiverged(format!(
					"attempted insertion of branch before {event} at {} (invalid due to versions: v{} < v{})",
					self.current_location(),
					version,
					event.version,
				)));
			} else if event.version > version {
				return Ok(HistoryResult::Insertion);
			}

			// Validate history is consistent
			let EventData::Branch = &event.data else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found branch",
					self.current_location(),
				)));
			};

			Ok(HistoryResult::Event(()))
		} else {
			Ok(HistoryResult::New)
		}
	}

	/// Returns `true` if the current event is a replay.
	pub fn compare_removed<T: Removed>(&self) -> WorkflowResult<bool> {
		if let Some(event) = self.current_event() {
			// Validate history is consistent
			let valid = if let EventData::Removed(removed) = &event.data {
				removed.name.as_deref() == T::name() && removed.event_type == T::event_type()
			} else {
				match T::event_type() {
					EventType::Activity => {
						if let EventData::Activity(activity) = &event.data {
							T::name().expect("bad impl") == activity.event_id.name
						} else {
							false
						}
					}
					EventType::SignalSend => {
						if let EventData::SignalSend(signal) = &event.data {
							T::name().expect("bad impl") == signal.name
						} else {
							false
						}
					}
					EventType::MessageSend => {
						if let EventData::MessageSend(msg) = &event.data {
							T::name().expect("bad impl") == msg.name
						} else {
							false
						}
					}
					EventType::Signal => matches!(event.data, EventData::Signal(_)),
					EventType::Loop => matches!(event.data, EventData::Loop(_)),
					EventType::Sleep => matches!(event.data, EventData::Sleep(_)),
					EventType::SubWorkflow => {
						if let EventData::SubWorkflow(sub_workflow) = &event.data {
							T::name().expect("bad impl") == sub_workflow.name
						} else {
							false
						}
					}
					EventType::Branch => matches!(event.data, EventData::Branch),
					_ => unreachable!("not implemented as a removable type"),
				}
			};

			if !valid {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found removed {}",
					self.current_location(),
					T::event_type(),
				)));
			};

			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Returns `Some` if the current event is a replay.
	pub fn compare_version_check(&self) -> WorkflowResult<Option<bool>> {
		if let Some(event) = self.current_event() {
			Ok(Some(matches!(event.data, EventData::VersionCheck)))
		} else {
			Ok(None)
		}
	}
}

pub enum HistoryResult<T> {
	/// An event for this location in history exists.
	Event(T),
	/// An event for this location in history exists, but it has a lower version. Therefore, an
	/// insertion is allowed.
	Insertion,
	/// No event for this location in history exists.
	New,
}

#[cfg(test)]
mod tests {
	use std::{collections::HashMap, sync::Arc};

	use super::{Cursor, HistoryResult};
	use crate::history::{
		event::{Event, EventData},
		location::{Coordinate, Location},
	};

	macro_rules! loc {
		($($i:expr),*) => {
			Location::new(Box::new([$($i),*]))
		};
	}
	macro_rules! coord {
		($($i:expr),*) => {
			Coordinate::new(Box::new([$($i),*]))
		};
	}

	/// Before 1 is 0.1
	#[test]
	fn insert_before_first() {
		let mut cursor = Cursor::new(Arc::new(HashMap::new()), Location::empty());

		let new = cursor.current_location_for(&HistoryResult::<()>::Insertion);

		// {0.1} comes before {1}
		assert_eq!(loc![coord![0, 1]], new);

		cursor.update(&new);
	}

	/// Between 2.1 and 3 should be 2.1.1
	#[test]
	fn between_complex_and_simple() {
		let events = [(
			loc![coord![1]],
			vec![
				Event {
					coordinate: coord![2, 1],
					version: 1,
					data: EventData::VersionCheck,
				},
				Event {
					coordinate: coord![3],
					version: 1,
					data: EventData::VersionCheck,
				},
			],
		)]
		.into_iter()
		.collect();
		let mut cursor = Cursor::new(Arc::new(events), loc![coord![1]]);

		cursor.update(&loc![coord![1], coord![2, 1]]);

		let new = cursor.current_location_for(&HistoryResult::<()>::Insertion);
		assert_eq!(loc![coord![1], coord![2, 1, 1]], new);

		cursor.update(&new);

		let new = cursor.current_location_for(&HistoryResult::<()>::Insertion);
		assert_eq!(loc![coord![1], coord![2, 1, 2]], new);
	}

	/// Between 2.1 and 2.2 should be 2.1.1
	#[test]
	fn cardinality() {
		let events = [(
			loc![coord![1]],
			vec![
				Event {
					coordinate: coord![2, 1],
					version: 1,
					data: EventData::VersionCheck,
				},
				Event {
					coordinate: coord![2, 2],
					version: 1,
					data: EventData::VersionCheck,
				},
			],
		)]
		.into_iter()
		.collect();
		let mut cursor = Cursor::new(Arc::new(events), loc![coord![1]]);

		cursor.update(&loc![coord![1], coord![2, 1]]);

		let new = cursor.current_location_for(&HistoryResult::<()>::Insertion);
		assert_eq!(loc![coord![1], coord![2, 1, 1]], new);
	}

	/// Between 2.1 and 2.1.1 should be 2.1.0.1
	#[test]
	fn cardinality2() {
		let events = [(
			loc![coord![1]],
			vec![
				Event {
					coordinate: coord![2, 1],
					version: 1,
					data: EventData::VersionCheck,
				},
				Event {
					coordinate: coord![2, 1, 1],
					version: 1,
					data: EventData::VersionCheck,
				},
			],
		)]
		.into_iter()
		.collect();
		let mut cursor = Cursor::new(Arc::new(events), loc![coord![1]]);

		cursor.update(&loc![coord![1], coord![2, 1]]);

		let new = cursor.current_location_for(&HistoryResult::<()>::Insertion);
		assert_eq!(loc![coord![1], coord![2, 1, 0, 1]], new);
	}
}
