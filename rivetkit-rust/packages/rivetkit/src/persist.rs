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

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use serde::{Deserialize, Serialize};

	use super::*;

	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct PersistedState {
		name: String,
		count: u32,
	}

	#[test]
	fn state_deltas_round_trip() -> Result<()> {
		let state = PersistedState {
			name: "alpha".into(),
			count: 7,
		};

		let deltas = state_deltas(&state)?;
		assert_eq!(deltas.len(), 1);

		let StateDelta::ActorState(bytes) = &deltas[0] else {
			panic!("expected actor-state delta");
		};
		let decoded: PersistedState = ciborium::from_reader(bytes.as_slice())
			.context("decode persisted state delta from cbor")?;
		assert_eq!(decoded, state);

		Ok(())
	}
}
