use anyhow::Result;
use epoxy_protocol::protocol;
use universaldb::Transaction;

use crate::replica::commit_kv::{self, CommitKvOutcome};

#[tracing::instrument(skip_all, fields(%replica_id, key = ?commit_req.key))]
pub async fn commit(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	commit_req: protocol::CommitRequest,
) -> Result<protocol::CommitResponse> {
	let protocol::CommitRequest {
		key,
		value,
		ballot,
		mutable,
		version,
	} = commit_req;

	let response = match commit_kv::commit_kv(tx, replica_id, key, value, ballot, mutable, version)
		.await?
	{
		CommitKvOutcome::Committed => protocol::CommitResponse::CommitResponseOk,
		CommitKvOutcome::AlreadyCommitted { value, .. } => {
			protocol::CommitResponse::CommitResponseAlreadyCommitted(
				protocol::CommitResponseAlreadyCommitted { value },
			)
		}
		CommitKvOutcome::StaleBallot { .. } => {
			protocol::CommitResponse::CommitResponseStaleCommit
		}
	};

	Ok(response)
}
