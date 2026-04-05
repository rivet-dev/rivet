use anyhow::Result;
use epoxy::{
	keys::{
		self, ChangelogKey, CommittedValue, KvAcceptedKey, KvAcceptedValue, KvBallotKey,
		KvOptimisticCacheKey, KvValueKey, LegacyCommittedValueKey,
	},
	ops::propose::{
		self, CheckAndSetCommand, Command, CommandKind, Proposal, ProposalResult, SetCommand,
	},
};
use epoxy_protocol::protocol::{self, ReplicaId};
use futures_util::TryStreamExt;
use gas::prelude::TestCtx as WorkflowTestCtx;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{FormalKey, IsolationLevel::Serializable, keys::CHANGELOG},
};

#[allow(dead_code)]
pub async fn execute_command(
	ctx: &WorkflowTestCtx,
	command: CommandKind,
	_wait_for_propagation: bool,
) -> Result<ProposalResult> {
	let result = ctx
		.op(propose::Input {
			proposal: Proposal {
				commands: vec![Command { kind: command }],
			},
			mutable: false,
			purge_cache: false,
			target_replicas: None,
		})
		.await?;

	Ok(result)
}

#[allow(dead_code)]
pub async fn set_if_absent(
	ctx: &WorkflowTestCtx,
	key: &[u8],
	value: &[u8],
) -> Result<ProposalResult> {
	execute_command(
		ctx,
		CommandKind::SetCommand(SetCommand {
			key: key.to_vec(),
			value: Some(value.to_vec()),
		}),
		false,
	)
	.await
}

#[allow(dead_code)]
pub async fn check_and_set_absent(
	ctx: &WorkflowTestCtx,
	key: &[u8],
	value: &[u8],
) -> Result<ProposalResult> {
	execute_command(
		ctx,
		CommandKind::CheckAndSetCommand(CheckAndSetCommand {
			key: key.to_vec(),
			expect_one_of: vec![None],
			new_value: Some(value.to_vec()),
		}),
		false,
	)
	.await
}

#[allow(dead_code)]
pub async fn set_mutable(
	ctx: &WorkflowTestCtx,
	key: &[u8],
	value: &[u8],
) -> Result<ProposalResult> {
	let result = ctx
		.op(propose::Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::SetCommand(SetCommand {
						key: key.to_vec(),
						value: Some(value.to_vec()),
					}),
				}],
			},
			mutable: true,
			purge_cache: false,
			target_replicas: None,
		})
		.await?;

	Ok(result)
}

#[allow(dead_code)]
pub async fn get_local(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	let output = ctx
		.op(epoxy::ops::kv::get_local::Input {
			replica_id,
			key: key.to_vec(),
		})
		.await?;
	Ok(output.value)
}

#[allow(dead_code)]
pub async fn read_v2_committed_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<CommittedValue>> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.read_opt(&KvValueKey::new(key), Serializable).await
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_v2_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	Ok(read_v2_committed_value(ctx, replica_id, key)
		.await?
		.map(|value| value.value))
}

#[allow(dead_code)]
pub async fn write_v2_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value: &[u8],
) -> Result<()> {
	write_v2_committed_value(
		ctx,
		replica_id,
		key,
		CommittedValue {
			value: value.to_vec(),
			version: 1,
			mutable: false,
		},
	)
	.await
}

#[allow(dead_code)]
pub async fn write_v2_committed_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value: CommittedValue,
) -> Result<()> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			let value = value.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.write(&KvValueKey::new(key), value)?;
				Ok(())
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_legacy_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				let tx = tx.with_subspace(keys::legacy_subspace(replica_id));
				tx.read_opt(&LegacyCommittedValueKey::new(key), Serializable)
					.await
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn write_legacy_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value: &[u8],
) -> Result<()> {
	let key = key.to_vec();
	let value = value.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			let value = value.clone();
			async move {
				let tx = tx.with_subspace(keys::legacy_subspace(replica_id));
				tx.write(&LegacyCommittedValueKey::new(key), value)?;
				Ok(())
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn write_legacy_v2_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value: &[u8],
) -> Result<()> {
	let key = key.to_vec();
	let value = value.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			let value = value.clone();
			async move {
				let legacy_subspace = keys::legacy_subspace(replica_id);
				let packed_key = legacy_subspace.pack(&KvValueKey::new(key));
				tx.set(&packed_key, &value);
				Ok(())
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_cache_committed_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<CommittedValue>> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.read_opt(&KvOptimisticCacheKey::new(key), Serializable)
					.await
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn write_cache_committed_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value: CommittedValue,
) -> Result<()> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			let value = value.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.write(&KvOptimisticCacheKey::new(key), value)?;
				Ok(())
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_cache_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	Ok(read_cache_committed_value(ctx, replica_id, key)
		.await?
		.map(|value| value.value))
}

#[allow(dead_code)]
pub async fn read_ballot(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<protocol::Ballot>> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.read_opt(&KvBallotKey::new(key), Serializable).await
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_accepted_value(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Result<Option<KvAcceptedValue>> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.read_opt(&KvAcceptedKey::new(key), Serializable).await
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn write_ballot(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	ballot: protocol::Ballot,
) -> Result<()> {
	let key = key.to_vec();
	ctx.udb()?
		.run(move |tx| {
			let key = key.clone();
			let ballot = ballot.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				tx.write(&KvBallotKey::new(key), ballot)?;
				Ok(())
			}
		})
		.await
}

#[allow(dead_code)]
pub async fn read_changelog_entries(
	ctx: &WorkflowTestCtx,
	replica_id: ReplicaId,
) -> Result<Vec<protocol::ChangelogEntry>> {
	ctx.udb()?
		.run(move |tx| async move {
			let replica_subspace = keys::subspace(replica_id);
			let changelog_subspace = replica_subspace.subspace(&(CHANGELOG,));
			let mut range: RangeOption<'static> = (&changelog_subspace).into();
			range.mode = StreamingMode::WantAll;

			let mut entries = Vec::new();
			let mut stream = tx.get_ranges_keyvalues(range, Serializable);
			while let Some(entry) = stream.try_next().await? {
				let changelog_key = replica_subspace.unpack::<ChangelogKey>(entry.key())?;
				entries.push(changelog_key.deserialize(entry.value())?);
			}

			Ok(entries)
		})
		.await
}
