use anyhow::{Context, Result, bail, ensure};
use epoxy_protocol::protocol::{self, CommittedValue, ReplicaId};
use futures_util::{StreamExt, stream::FuturesUnordered};
use gas::prelude::*;
use universaldb::utils::IsolationLevel::Serializable;

use crate::{
	keys,
	ops::propose,
	replica::{
		ballot::{self, Ballot},
		commit_kv,
	},
	utils,
};

/// A read promise quorum intersects every write quorum. Inspect accepted *and* committed
/// state atomically at each voter; then complete acceptance of an unresolved value before
/// returning it. A majority of ordinary KvGet responses cannot establish this guarantee.
/// Like proposals, this requires a stable replica scope for the lifetime of the key.
pub(super) async fn read(
	ctx: &OperationCtx,
	key: &[u8],
	scope: Option<&[ReplicaId]>,
) -> Result<Option<CommittedValue>> {
	let config = ctx
		.op(crate::ops::read_cluster_config::Input {})
		.await?
		.config;
	let replica_id = ctx.config().epoxy_replica_id();
	let members = utils::resolve_active_quorum_members(&config, replica_id, scope)?;
	let quorum = utils::calculate_quorum(members.len(), utils::QuorumType::Slow);
	let mut minimum_counter = 0;
	for _ in 0..8 {
		let ballot = ctx
			.udb()?
			.txn("epoxy_read_reserve_ballot", |tx| async move {
				let local = tx.with_subspace(keys::subspace(replica_id));
				let current = local
					.read_opt(&keys::KvBallotKey::new(key.to_vec()), Serializable)
					.await?;
				let counter = current
					.map_or(0, |b| b.counter)
					.max(minimum_counter)
					.checked_add(1)
					.context("Epoxy ballot overflow")?;
				let ballot = Ballot::new(counter, replica_id);
				ballot::store_ballot(&tx, replica_id, key.to_vec(), ballot)?;
				Ok(ballot)
			})
			.await?;
		let mut pending = members
			.iter()
			.map(|&member| {
				super::read_state(ctx, &config, member, key.to_vec(), Some(ballot.into()))
			})
			.collect::<FuturesUnordered<_>>();
		let mut votes = 0;
		let mut preempted = false;
		let mut latest: Option<CommittedValue> = None;
		let mut latest_committed = false;
		while let Some(response) = pending.next().await {
			let Ok(state) = response else { continue };
			if let Some(promised) = &state.promised {
				minimum_counter = minimum_counter.max(promised.counter);
				preempted |= Ballot::from(promised.clone()) > ballot;
			}
			if state.promised.as_ref() != Some(&ballot.into()) {
				continue;
			}
			votes += 1;
			if let Some(value) = state.committed {
				select(&mut latest, &mut latest_committed, value, true)?;
			}
			if let Some(value) = state.accepted {
				select(
					&mut latest,
					&mut latest_committed,
					CommittedValue {
						value: value.value,
						version: value.version,
						mutable: value.mutable,
					},
					false,
				)?;
			}
			if votes >= quorum {
				break;
			}
		}
		drop(pending);
		if votes < quorum {
			if preempted {
				continue;
			}
			bail!("Epoxy read could not establish a promise quorum");
		}
		let Some(value) = latest else {
			return Ok(None);
		};
		if !latest_committed {
			// Re-propose the selected accepted value at this read's ballot. AlreadyCommitted
			// responses omit the version, so retry the read instead of counting them as votes.
			let mut accepts = members
				.iter()
				.map(|&member| {
					tokio::time::timeout(
						std::time::Duration::from_secs(2),
						propose::send_accept_request(
							ctx,
							&config,
							replica_id,
							member,
							key.to_vec(),
							value.clone(),
							ballot.into(),
						),
					)
				})
				.collect::<FuturesUnordered<_>>();
			let mut accepted = 0;
			while let Some(response) = accepts.next().await {
				if matches!(
					response,
					Ok(Ok(protocol::AcceptResponse::AcceptResponseOk(_)))
				) {
					accepted += 1;
					if accepted >= quorum {
						break;
					}
				}
			}
			drop(accepts);
			if accepted < quorum {
				continue;
			}
		}
		// Learning is monotonic by version. If a concurrent writer moved this replica ahead,
		// restart instead of publishing an older generation. No read invents a new version.
		let outcome = ctx
			.udb()?
			.txn("epoxy_read_learn", |tx| {
				let value = value.clone();
				async move {
					commit_kv::commit_kv(
						&tx,
						replica_id,
						key.to_vec(),
						value.value,
						ballot.into(),
						value.mutable,
						value.version,
					)
					.await
				}
			})
			.await?;
		match outcome {
			commit_kv::CommitKvOutcome::Committed => {}
			commit_kv::CommitKvOutcome::AlreadyCommitted {
				value: current,
				version,
			} if version == value.version && current == value.value => {}
			_ => continue,
		}
		// Best-effort dissemination is safe after agreement; a fixed owner that misses it
		// retains its locally accepted proposal and cannot serve a quiescent owner read.
		tokio::spawn({
			let ctx = ctx.clone();
			let config = config.clone();
			let key = key.to_vec();
			let value = value.clone();
			async move {
				if propose::broadcast_commits(&ctx, &config, replica_id, key, value, ballot.into())
					.await
					.is_err()
				{
					tracing::warn!("Epoxy read recovery commit propagation failed");
				}
			}
		});
		return Ok(Some(value));
	}
	bail!("Epoxy read contention exceeded retry limit")
}

fn select(
	latest: &mut Option<CommittedValue>,
	committed: &mut bool,
	value: CommittedValue,
	is_committed: bool,
) -> Result<()> {
	if let Some(current) = latest {
		if value.version < current.version {
			return Ok(());
		}
		if value.version == current.version {
			// Conflicting values are not evidence of freshness. Fail closed even if one of
			// them is merely an unchosen proposal; a later uncontended read can recover.
			ensure!(
				value == *current,
				"Epoxy read found divergent values at the same version"
			);
			*committed |= is_committed;
			return Ok(());
		}
	}
	*latest = Some(value);
	*committed = is_committed;
	Ok(())
}
