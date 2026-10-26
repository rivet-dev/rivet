use anyhow::{Context, Result};
use rivetkit_core::{ConnId, StateDelta};
use serde::Serialize;

pub fn state_delta<S: Serialize>(state: &S) -> Result<StateDelta> {
	let mut encoded = Vec::new();
	ciborium::into_writer(state, &mut encoded).context("encode actor state as cbor")?;
	Ok(StateDelta::ActorState(encoded))
}

pub fn state_deltas<S: Serialize>(state: &S) -> Result<Vec<StateDelta>> {
	Ok(vec![state_delta(state)?])
}

pub fn conn_hibernation_delta(conn: ConnId, bytes: Vec<u8>) -> StateDelta {
	StateDelta::ConnHibernation { conn, bytes }
}

pub fn conn_hibernation_removed_delta(conn: ConnId) -> StateDelta {
	StateDelta::ConnHibernationRemoved(conn)
}


