//! Hosts one key's state machine on its Depot volume: each command loads the record, runs one
//! transition and persists the record when the transition changed it.

use anyhow::Result;
use depot_worker::command::KvHandle;
use epoxy_protocol::protocol;
use rivet_keys::keys::KeyAddress;

use super::keys::kv::EpoxyRecordKey;
use crate::key_machine::{self, event::KeyEvent, state, types::KeyContext};

#[derive(Clone)]
pub(crate) struct LocalReplica {
	pub kv: KvHandle,
	replica_id: protocol::ReplicaId,
}

impl LocalReplica {
	pub fn new(kv: KvHandle, replica_id: u64) -> Self {
		Self { kv, replica_id }
	}

	/// Reads the committed value through a read handle, which may be a maintainer read session
	/// that lags the committed log.
	pub async fn get_read(
		read: &depot_worker::command::ReadHandle,
		_key: &[u8],
	) -> Result<Option<protocol::CommittedValue>> {
		let raw = read.get(&EpoxyRecordKey::new().pack()).await?;
		let state = state::decode(raw.as_deref())?;
		Ok(key_machine::snapshot::snapshot(&state).committed)
	}

	/// Runs `event` against the record inside one tracked transaction. A transition that leaves
	/// the record unchanged writes nothing; one that changes it answers only after its commit is
	/// durable. A conflicting commit reruns the transition on the newer record.
	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn run(&self, event: KeyEvent) -> Result<protocol::ReplicaCommandResponse> {
		let ctx = KeyContext {
			replica_id: self.replica_id,
		};
		loop {
			let mut txn = self.kv.begin().await?;
			let key = EpoxyRecordKey::new().pack();
			let raw = txn.get(&key).await?;
			let before = state::decode(raw.as_deref())?;
			let mut after = before.clone();
			let response = key_machine::on_event(&mut after, ctx, event.clone())?;
			if after == before {
				return Ok(record(&event, response));
			}

			let version = self
				.kv
				.write_version(rivet_data::families::VersionFamily::EPOXY_KEY_STATE)?;
			txn.put(&key, &state::encode(&after, version)?)?;
			match self.kv.commit(txn).await {
				Ok(_) => return Ok(record(&event, response)),
				Err(
					depot_client::filesystem::Error::Conflict
					| depot_client::filesystem::Error::TooOld,
				) => continue,
				Err(error) => return Err(error.into()),
			}
		}
	}
}

/// Counts one answered transition.
fn record(
	event: &KeyEvent,
	response: protocol::ReplicaCommandResponse,
) -> protocol::ReplicaCommandResponse {
	crate::metrics::KEY_TRANSITION_TOTAL
		.with_label_values(&[event.name(), key_machine::reply_name(&response)])
		.inc();
	response
}
