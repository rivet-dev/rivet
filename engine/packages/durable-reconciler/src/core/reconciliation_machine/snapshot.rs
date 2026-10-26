#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
	pub done: bool,
}

impl rivet_state_machine::Snapshot for Snapshot {}
