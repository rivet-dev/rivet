//! Raw key layout the migration API reads and writes outside pegboard's own key types.
//!
//! Gasoline's key types are private to gasoline, so the few gasoline keys the park touches are
//! packed here from the same tuple layout. `tests/migration.rs` runs a real `pegboard_actor2`
//! workflow and fails if any of these stop matching what gasoline writes.

use rivet_util::Id;
use universaldb::{prelude::*, utils::Subspace};

/// Gasoline's KV subspace.
pub fn gasoline_subspace() -> Subspace {
	Subspace::new(&(RIVET, GASOLINE, KV))
}

/// The workflow's name. Value: UTF-8 string.
pub fn workflow_name(workflow_id: Id) -> Vec<u8> {
	gasoline_subspace().pack(&(WORKFLOW, DATA, workflow_id, NAME))
}

/// The workflow's state, stored as JSON split into chunks keyed by their index under this
/// prefix.
pub fn workflow_state_subspace(workflow_id: Id) -> Subspace {
	gasoline_subspace().subspace(&(WORKFLOW, DATA, workflow_id, STATE))
}

/// The workflow's armed deadline. Value: big-endian `i64` milliseconds.
pub fn wake_deadline(workflow_id: Id) -> Vec<u8> {
	gasoline_subspace().pack(&(WORKFLOW, DATA, workflow_id, WAKE_DEADLINE))
}

/// Present while a worker holds or is acquiring the workflow's lease.
pub fn lease(workflow_id: Id) -> Vec<u8> {
	gasoline_subspace().pack(&(WORKFLOW, LEASE, workflow_id))
}

/// Set on a silenced workflow. Gasoline drops a silenced workflow whenever a pull leases it and
/// never reclaims its lease, so a parked workflow stays inert even if a wake row survives the
/// park. Value: big-endian `i64` milliseconds.
pub fn silence_ts(workflow_id: Id) -> Vec<u8> {
	gasoline_subspace().pack(&(WORKFLOW, DATA, workflow_id, SILENCE_TS))
}

/// Present while a worker holds the workflow.
pub fn worker_id(workflow_id: Id) -> Vec<u8> {
	gasoline_subspace().pack(&(WORKFLOW, DATA, workflow_id, WORKER_ID))
}

/// Prefix of the signal names the sleeping workflow listens for. Each row is the prefix followed by
/// the packed signal name, with an empty value.
pub fn wake_signal_subspace(workflow_id: Id) -> Subspace {
	gasoline_subspace().subspace(&(WORKFLOW, DATA, workflow_id, WAKE_SIGNAL))
}

/// Prefix of the workflow's pending (unacknowledged) signals.
pub fn pending_signal_subspace(workflow_id: Id) -> Subspace {
	gasoline_subspace().subspace(&(WORKFLOW, SIGNAL, workflow_id, PENDING))
}

/// Gasoline wake condition variants, in key order.
const WAKE_IMMEDIATE: usize = 0;
const WAKE_DEADLINE_VARIANT: usize = 1;

/// The wake index row that fires the workflow at its deadline. Value: big-endian `u16` nonce.
pub fn deadline_wake_row(workflow_name: &str, workflow_id: Id, deadline_ts: i64) -> Vec<u8> {
	gasoline_subspace().pack(&(
		WAKE,
		WORKFLOW,
		workflow_name,
		deadline_ts,
		workflow_id,
		WAKE_DEADLINE_VARIANT,
	))
}

/// A wake index row that makes the next pull run the workflow right away.
pub fn immediate_wake_row(workflow_name: &str, workflow_id: Id, ts: i64) -> Vec<u8> {
	gasoline_subspace().pack(&(
		WAKE,
		WORKFLOW,
		workflow_name,
		ts,
		workflow_id,
		WAKE_IMMEDIATE,
	))
}

/// A fresh nonce for a written wake row. Gasoline clears the rows it pulled with a compare against
/// the nonce it read, so a rewritten row needs a new one.
pub fn wake_row_nonce() -> Vec<u8> {
	rand::random::<u16>().to_be_bytes().to_vec()
}

/// The newest cutover that a confirm or restore undid. A cutover request at or before it is a late
/// duplicate and must not cut the actor again. Value: big-endian `i64`.
pub fn uncut(actor_id: Id) -> Vec<u8> {
	pegboard::keys::subspace().pack(&(MIGRATION, UNCUT, actor_id))
}

/// The parked record of a migrated actor, in pegboard's subspace.
pub fn parked(actor_id: Id) -> Vec<u8> {
	pegboard::keys::subspace().pack(&(MIGRATION, PARKED, actor_id))
}
