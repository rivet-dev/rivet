use rivet_error::*;

/// A `Method::Get` event reached a volume that holds no machine. Nothing deletes a machine, so the
/// machine was never committed: for example, an effect started before the commit that would have
/// created it, and that commit failed.
#[derive(RivetError)]
#[error(
	"state_machine",
	"machine_not_found",
	"The state machine does not exist."
)]
pub struct MachineNotFound;

/// Whether a failed event, including one returned across a depot command, found no machine.
pub fn is_machine_not_found(error: &anyhow::Error) -> bool {
	let structured = RivetError::extract(error);
	structured.group() == "state_machine" && structured.code() == "machine_not_found"
}
