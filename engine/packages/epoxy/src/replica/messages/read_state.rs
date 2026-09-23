use anyhow::{Result, ensure};
use epoxy_protocol::protocol;
use universaldb::{Transaction, utils::IsolationLevel::Serializable};

use crate::{keys, replica::ballot::Ballot};

/// Read both learned and in-flight state in the transaction that installs the promise.
pub async fn read_state(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	request: protocol::KvReadStateRequest,
) -> Result<protocol::KvReadStateResponse> {
	let config = crate::utils::read_config(tx, replica_id).await?;
	ensure!(
		config.epoch == request.epoch,
		"Epoxy read configuration changed"
	);
	ensure!(
		crate::utils::get_quorum_members(&config).contains(&replica_id),
		"Epoxy read replica is inactive"
	);
	let local = tx.with_subspace(keys::subspace(replica_id));
	let key = request.key;
	let value_key = keys::KvValueKey::new(key.clone());
	let accepted_key = keys::KvAccepted2Key::new(key.clone());
	let legacy_accepted_key = keys::KvAcceptedKey::new(key.clone());
	let ballot_key = keys::KvBallotKey::new(key.clone());
	let (committed, accepted, legacy_accepted, promised) = tokio::try_join!(
		local.read_opt(&value_key, Serializable),
		local.read_opt(&accepted_key, Serializable),
		local.read_opt(&legacy_accepted_key, Serializable),
		local.read_opt(&ballot_key, Serializable),
	)?;
	let committed = match committed {
		Some(value) => Some(value),
		None => {
			let legacy = tx.with_subspace(keys::legacy_subspace(replica_id));
			let legacy_key = keys::LegacyCommittedValueKey::new(key.clone());
			let (raw, versioned) = tokio::try_join!(
				legacy.read_opt(&legacy_key, Serializable),
				legacy.read_opt(&value_key, Serializable),
			)?;
			raw.map(|value| protocol::CommittedValue {
				value: Some(value),
				version: 0,
				mutable: false,
			})
			.or(versioned)
		}
	};
	let accepted = accepted.or_else(|| {
		legacy_accepted.map(|value| protocol::AcceptedValue {
			value: Some(value.value),
			ballot: value.ballot,
			version: value.version,
			mutable: value.mutable,
		})
	});
	let promised = if let Some(ballot) = request.ballot {
		if promised
			.as_ref()
			.is_none_or(|current| Ballot::from(current.clone()) <= Ballot::from(ballot.clone()))
		{
			local.write(&keys::KvBallotKey::new(key), ballot.clone())?;
			Some(ballot)
		} else {
			promised
		}
	} else {
		promised
	};
	Ok(protocol::KvReadStateResponse {
		committed,
		accepted,
		promised,
	})
}
