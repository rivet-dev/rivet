//! Replays recovered maintainer log ops into the store, merged with each volume's own log.
//!
//! An op that names a volume carries the position the volume's apply stood at when the live
//! maintainer decided it, and the live maintainer held that apply there until the op was in
//! the store. Replay does the same: it holds the volume's apply at the op's position, applies
//! the op once the volume's apply reaches it, then lets the apply continue. An op whose volume
//! is already cold past its position was covered by the pass that moved cold there, so it is
//! skipped. An op without a volume, or of a volume with nothing left to apply, runs in log
//! order at once.
//!
//! Until every recovered op is applied, sweeps, forgets and trims of the log wait, and new
//! lifecycle work on a volume with ops still to replay is refused with a retryable answer:
//! they decide by store rows the replay may still change. For the same reason a recovered
//! delete of such a name waits for the replay before it reads its incarnation's record: a
//! restore's publication writes that record Deleted. The incarnation an op waits on still registers and
//! applies to the op's position meanwhile. An incarnation that waits for its name's replay
//! is offered again after each of the name's ops, since the op may have written its row; its
//! own ops wait for it only while the publication that creates it is still to replay, and
//! an op of an unregistered incarnation runs behind the name's earlier ops, so the ops of a
//! name replay in log order.

use super::{LogToken, MaintainerState};
use crate::tablet_maintainer_machine::Effects;
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_maintainer_log_v1::{ForkPublishOp, MaintainerOp, MaintainerOpTarget},
	shared_depot_types_v1::VolumeLogPosition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// One recovered op still to apply.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Pending {
	pub token: LogToken,
	pub op: MaintainerOp,
	/// Recovered at startup, as opposed to a live op this lifetime queued. Only recovered
	/// ops hold back lifecycle work, sweeps and forgets.
	pub recovered: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Replay {
	/// Recovered ops not yet sorted to a volume, in log order.
	pub(crate) queue: VecDeque<Pending>,
	/// Ops waiting for their volume's apply, in log order per volume.
	pub(crate) by_volume: BTreeMap<VolumeLogKey, VecDeque<Pending>>,
	/// Ops whose store apply is in flight, by token, with their volume and whether they were
	/// recovered.
	pub(crate) applying: BTreeMap<LogToken, (Option<VolumeLogKey>, bool)>,
	/// The volume key an applying op without a target names, such as a fork publication's
	/// destination, so lifecycle work on that key waits for it as for any other replayed op.
	pub(crate) applying_keys: BTreeMap<LogToken, depot_types::VolumeKey>,
	/// Recovered ops not applied yet, queued, waiting or in flight.
	pub(crate) recovered: u64,
}

/// The volume and position an op follows.
pub(crate) fn target(op: &MaintainerOp) -> Option<(VolumeLogKey, VolumeLogPosition)> {
	op_target(op).map(
		|MaintainerOpTarget {
		     volume_log_key,
		     after,
		 }| { (VolumeLogKey(volume_log_key.clone()), after.clone()) },
	)
}

fn op_target(op: &MaintainerOp) -> Option<&MaintainerOpTarget> {
	match op {
		MaintainerOp::CheckpointOp(op) => op.target.as_ref(),
		MaintainerOp::RetentionCheckpointOp(op) => Some(&op.target),
		MaintainerOp::ForkReferenceOp(op) => op.target.as_ref(),
		MaintainerOp::ForkPublishOp(op) => op.target.as_ref(),
		MaintainerOp::ForkFailOp(_) | MaintainerOp::RepairAuditOp(_) => None,
	}
}

/// The volume and position trim waits on before it releases the op's entry: the volume's
/// cold must reach the position, or a replay from that cold would apply the op out of order.
/// A restore's publication names the replaced incarnation only to order its replay; that
/// incarnation's cold never moves again, and the publication is in cold once a pass covers
/// it, so trim does not wait on it.
pub(crate) fn trim_target(op: &MaintainerOp) -> Option<(VolumeLogKey, VolumeLogPosition)> {
	match op {
		MaintainerOp::ForkPublishOp(_) => None,
		MaintainerOp::CheckpointOp(_)
		| MaintainerOp::RetentionCheckpointOp(_)
		| MaintainerOp::ForkReferenceOp(_)
		| MaintainerOp::ForkFailOp(_)
		| MaintainerOp::RepairAuditOp(_) => target(op),
	}
}

/// The volume key an op changes, to tell which tablet of a split owns it.
fn volume_key(op: &MaintainerOp) -> Option<&depot_types::VolumeKey> {
	match op {
		MaintainerOp::CheckpointOp(op) => Some(&op.request.volume_key),
		MaintainerOp::RetentionCheckpointOp(op) => Some(&op.target.volume_log_key.volume_key),
		MaintainerOp::ForkReferenceOp(op) => Some(&op.request.spec().parent_volume_key),
		MaintainerOp::ForkPublishOp(op) => Some(&op.spec.destination_volume_key),
		MaintainerOp::ForkFailOp(op) => Some(&op.spec.destination_volume_key),
		MaintainerOp::RepairAuditOp(op) => Some(&op.row.request.envelope.volume_log_key.volume_key),
	}
}

/// Takes one op recovered from this tablet's own log. A frozen tablet applies nothing: its
/// split children replay its log.
pub(crate) fn recovered(
	state: &mut MaintainerState,
	token: LogToken,
	op: MaintainerOp,
	frozen: bool,
) {
	if frozen {
		return;
	}
	state.maintainer_log.replay.recovered += 1;
	state.maintainer_log.replay.queue.push_back(Pending {
		token,
		op,
		recovered: true,
	});
}

/// Takes one op recovered from the split parent's log. Ops of volumes in this child's range
/// are logged again here, so a restart of this child no longer needs the parent's log, and
/// replayed like this log's own.
pub(crate) fn recovered_from_parent(state: &mut MaintainerState, op: MaintainerOp) {
	if volume_key(&op).is_none_or(|key| !super::super::in_tablet(state, key)) {
		return;
	}
	let target = trim_target(&op);
	let token = super::submit(state, op.clone(), target, super::Waiter::Replay);
	state.maintainer_log.replay.recovered += 1;
	state.maintainer_log.replay.queue.push_back(Pending {
		token,
		op,
		recovered: true,
	});
}

/// Logs a live op that follows `key`'s apply position and applies it through the replay
/// queue once it commits: its volume's apply waits at the op's position until then.
pub(crate) fn submit_live(state: &mut MaintainerState, op: MaintainerOp, key: VolumeLogKey) {
	let target = trim_target(&op);
	let token = super::submit(state, op.clone(), target, super::Waiter::Replay);
	state
		.maintainer_log
		.replay
		.by_volume
		.entry(key)
		.or_default()
		.push_back(Pending {
			token,
			op,
			recovered: false,
		});
}

/// Logs a live op that follows no volume's apply and applies it through the replay queue, in
/// log order, once its entry commits.
pub(crate) fn submit_unordered(state: &mut MaintainerState, op: MaintainerOp) {
	let token = super::submit(state, op.clone(), None, super::Waiter::Replay);
	state.maintainer_log.replay.queue.push_back(Pending {
		token,
		op,
		recovered: false,
	});
}

/// Whether ops of `key` wait in the replay queue or are being applied.
pub(crate) fn has_ops(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let replay = &state.maintainer_log.replay;
	replay.by_volume.contains_key(key)
		|| replay
			.applying
			.values()
			.any(|(volume, _)| volume.as_ref() == Some(key))
}

/// Whether recovered ops are still to apply.
pub(crate) fn replaying(state: &MaintainerState) -> bool {
	state.maintainer_log.replay.recovered > 0
}

/// Whether `key` has recovered ops still to apply, so new lifecycle work on it waits.
pub(crate) fn pending_for(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	let replay = &state.maintainer_log.replay;
	if replay.recovered == 0 {
		return false;
	}
	if replay
		.queue
		.iter()
		.any(|pending| pending.recovered && volume_key(&pending.op) == Some(key))
	{
		return true;
	}
	replay
		.by_volume
		.range(super::super::incarnations(key))
		.any(|(_, ops)| ops.iter().any(|pending| pending.recovered))
		|| replay.applying.values().any(|(volume, recovered)| {
			*recovered && volume.as_ref().is_some_and(|v| v.0.volume_key == *key)
		}) || replay.applying_keys.iter().any(|(token, applying_key)| {
		applying_key == key
			&& replay
				.applying
				.get(token)
				.is_some_and(|(_, recovered)| *recovered)
	})
}

/// The apply limit of a registered volume: the position of its next op to replay.
pub(crate) fn apply_limit(state: &MaintainerState, key: &VolumeLogKey) -> Option<u64> {
	state
		.maintainer_log
		.replay
		.by_volume
		.get(key)
		.and_then(|ops| ops.front())
		.and_then(|pending| target(&pending.op))
		.map(|(_, after)| after.volume_seq)
}

/// The position the volume's next op to replay follows, as commit evidence. The maintainer
/// that logged the op took it from the volume's applied position, its cold, or a committed
/// checkpoint, and never from a drain's final tail: no op of an incarnation is logged while a
/// drain of it is held. So the log through it is committed. A
/// restart needs this when the volume's last claim is one it no longer knows, such as a drain
/// that fixed the tail and never appended: its members then hold an older epoch's tail under a
/// newer claim, which proves nothing on its own, and the recovery that would prove it waits
/// for this replay.
pub(crate) fn committed_through<'a>(
	state: &'a MaintainerState,
	key: &VolumeLogKey,
) -> Option<&'a VolumeLogPosition> {
	let pending = state.maintainer_log.replay.by_volume.get(key)?.front()?;
	op_target(&pending.op).map(|target| &target.after)
}

/// The volume's cold position as the store durably holds it: its registered record once a
/// pass or a lookup confirmed it, else its stored row. A volume this maintainer does not hold
/// has no log above its stored row, so nothing of it waits. An incarnation whose stored record
/// is lost never registers again, so an applied entry of its ops never waits on it either:
/// trimming still needs the entry applied and covered by a pass, so the op's own rows are in
/// the store, and the only thing waived is an apply that cannot come. Without this one such
/// entry would pin the log for the lifetime.
pub(crate) fn durable_cold(state: &MaintainerState, key: &VolumeLogKey) -> u64 {
	if super::super::filesystem::is_record_missing(state, key) {
		return u64::MAX;
	}
	if let Some(volume) = state.filesystem.volumes.get(key) {
		return if volume.record_persisted {
			volume.record.cold.volume_seq
		} else {
			state
				.loaded_volumes
				.get(key)
				.and_then(|loaded| loaded.stored_row.as_ref())
				.map_or(0, |row| row.cold.volume_seq)
		};
	}
	match state.loaded_volumes.get(key) {
		Some(loaded) => loaded
			.stored_row
			.as_ref()
			.map_or(0, |row| row.cold.volume_seq),
		None => u64::MAX,
	}
}

/// Sorts queued ops to their volumes and dispatches every op that may apply now.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	let replay = &state.maintainer_log.replay;
	if state.phase != super::super::MaintainerPhase::Serving
		|| (replay.queue.is_empty() && replay.by_volume.is_empty())
	{
		return;
	}
	// Ops leave the queue in log order. One that names no volume, or a volume that cannot
	// apply anything more, applies once every earlier op of the queue has been sorted.
	while let Some(pending) = state.maintainer_log.replay.queue.pop_front() {
		// An op logged again by this lifetime waits for its entry to commit, and the ops
		// behind it wait with it to keep log order.
		if pending.token.seq > state.maintainer_log.committed && target(&pending.op).is_none() {
			state.maintainer_log.replay.queue.push_front(pending);
			break;
		}
		match target(&pending.op) {
			Some(_) if failed_later(state, &pending.op) => {
				// The client was told this restore failed: the failure op behind it fails it
				// again instead.
				super::applied(state, pending.token, fx);
				if pending.recovered {
					let key = volume_key(&pending.op).cloned();
					recovered_done(state, key, fx);
				}
			}
			// A restore's publication names the replaced incarnation only to wait for its
			// apply; a pass that moved that incarnation's cold did not publish anything, so
			// the store decides whether the publication is there.
			Some((key, after))
				if !matches!(pending.op, MaintainerOp::ForkPublishOp(_))
					&& after.volume_seq < durable_cold(state, &key) =>
			{
				// The pass that moved the volume's cold past the op covered it.
				super::applied(state, pending.token, fx);
				if pending.recovered {
					let key = volume_key(&pending.op).cloned();
					recovered_done(state, key, fx);
				}
			}
			Some((key, _)) => {
				state
					.maintainer_log
					.replay
					.by_volume
					.entry(key.clone())
					.or_default()
					.push_back(pending);
				super::super::filesystem::load(state, key.clone());
				super::super::filesystem::mark_candidate(state, &key);
			}
			None if failed_later(state, &pending.op) => {
				// The client was told this fork failed: the failure op behind it fails it
				// again instead.
				super::applied(state, pending.token, fx);
				if pending.recovered {
					let key = volume_key(&pending.op).cloned();
					recovered_done(state, key, fx);
				}
			}
			None => {
				// Its store change waits for other store work of the same volume.
				if busy(state, &pending.op) {
					state.maintainer_log.replay.queue.push_front(pending);
					break;
				}
				if let Some(key) = volume_key(&pending.op) {
					state
						.maintainer_log
						.replay
						.applying_keys
						.insert(pending.token, key.clone());
				}
				state
					.maintainer_log
					.replay
					.applying
					.insert(pending.token, (None, pending.recovered));
				super::super::logged_ops::replay(state, pending.token, pending.op, None, fx);
			}
		}
	}
	let keys = state
		.maintainer_log
		.replay
		.by_volume
		.keys()
		.cloned()
		.collect::<Vec<_>>();
	for key in keys {
		dispatch_ready(state, &key, fx);
	}
}

/// Dispatches the next op of `key` once its volume's apply reached the op's position, or at
/// once when the volume cannot apply its own log any further.
pub(crate) fn dispatch_ready(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if state
		.maintainer_log
		.replay
		.applying
		.values()
		.any(|(volume, _)| volume.as_ref() == Some(key))
	{
		return;
	}
	let Some(pending) = state
		.maintainer_log
		.replay
		.by_volume
		.get(key)
		.and_then(|ops| ops.front())
		.cloned()
	else {
		return;
	};
	let Some((_, after)) = target(&pending.op) else {
		return;
	};
	// A live op queued here applies only once its entry commits.
	if pending.token.seq > state.maintainer_log.committed {
		return;
	}
	// A volume that became eligible during this event, as the replayed publication that
	// created it finished, registers now rather than in the filesystem drive that follows
	// this one: its op must follow its own apply, never run in its place.
	if !state.filesystem.volumes.contains_key(key) {
		super::super::filesystem::load(state, key.clone());
	}
	let ready = match state.filesystem.volumes.get(key) {
		Some(volume) => {
			let busy = state
				.operations
				.index()
				.filesystem_applies
				.contains_key(key)
				|| state.operations.index().filesystem_reads.contains_key(key);
			!busy && volume.applied.volume_seq >= after.volume_seq
		}
		// A volume that is not registered applies nothing of its own log here: it is not
		// loaded, it is deleted with nothing above its stored cold, or it waits for this
		// replay itself. Its op then runs in log order behind the name's earlier ops, which
		// may be what makes the volume current, as a chained restore's first publication is
		// for the second.
		None => {
			!super::super::filesystem::owns(state, key)
				&& !waits_for_registration(state, key)
				&& !earlier_op_of_name_pending(state, &key.0.volume_key, &pending.token)
		}
	};
	if !ready || busy(state, &pending.op) {
		return;
	}
	let ops = state
		.maintainer_log
		.replay
		.by_volume
		.get_mut(key)
		.expect("pending volume");
	let pending = ops.pop_front().expect("pending op");
	if ops.is_empty() {
		state.maintainer_log.replay.by_volume.remove(key);
	}
	// The volume's apply limit moved to the next op, or went away.
	super::super::filesystem::mark_candidate(state, key);
	state
		.maintainer_log
		.replay
		.applying
		.insert(pending.token, (Some(key.clone()), pending.recovered));
	super::super::logged_ops::replay(state, pending.token, pending.op, Some(key.clone()), fx);
}

/// Whether `op` is a fork publication that a failure op of the same spec, still queued behind
/// it, failed after it was logged.
fn failed_later(state: &MaintainerState, op: &MaintainerOp) -> bool {
	let MaintainerOp::ForkPublishOp(publish) = op else {
		return false;
	};
	state.maintainer_log.replay.queue.iter().any(
		|pending| matches!(&pending.op, MaintainerOp::ForkFailOp(fail) if fail.spec == publish.spec),
	)
}

/// Whether store work in flight on the op's volume conflicts with applying the op, which
/// writes that volume's rows. The replay drive runs after every event and tries it again.
fn busy(state: &MaintainerState, op: &MaintainerOp) -> bool {
	super::super::logged_ops::op_volume(op).is_some_and(|volume_key| {
		super::super::footprint::volume_conflict(
			state,
			&super::super::footprint::StoreVolume::from(&volume_key),
			super::super::footprint::VolumeAccess::Write,
		)
	})
}

/// Whether a recovered op of `name` logged before `token` is still queued, waiting or in
/// flight, so an op dispatched without its volume's apply must wait for it.
fn earlier_op_of_name_pending(
	state: &MaintainerState,
	name: &depot_types::VolumeKey,
	token: &LogToken,
) -> bool {
	earliest_pending_token(state, name).is_some_and(|earliest| earliest < *token)
}

/// The lowest token among the recovered ops of `name` still queued, waiting for a volume's
/// apply or in flight, if any. Ops in flight count by the volume they follow or the key they
/// name.
pub(crate) fn earliest_pending_token(
	state: &MaintainerState,
	name: &depot_types::VolumeKey,
) -> Option<LogToken> {
	let replay = &state.maintainer_log.replay;
	let queued = replay
		.queue
		.iter()
		.filter(|pending| volume_key(&pending.op) == Some(name))
		.map(|pending| pending.token);
	let waiting = replay
		.by_volume
		.range(super::super::incarnations(name))
		.flat_map(|(_, ops)| ops.iter())
		.map(|pending| pending.token);
	let applying = replay
		.applying
		.iter()
		.filter(|(applying, (volume, _))| {
			volume
				.as_ref()
				.is_some_and(|volume| volume.0.volume_key == *name)
				|| replay.applying_keys.get(applying) == Some(name)
		})
		.map(|(applying, _)| *applying);
	queued.chain(waiting).chain(applying).min()
}

/// Whether a loaded volume will register for apply and should be waited for. An incarnation
/// a restore or recreate replaced never registers again, except one whose replacing restore
/// is still to replay: its ops and that restore's publication follow its apply, and the store
/// names it live until the publication is replayed. One that waits for its name's replay
/// registers once the publication that creates it is replayed, so its ops wait for that.
fn waits_for_registration(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	if state.filesystem.awaiting_replay.contains(key) {
		return publication_pending(state, key);
	}
	state.loaded_volumes.get(key).is_some_and(|volume| {
		(!volume.is_deleted()
			&& (!super::super::deletion::is_replaced(state, key) || restore_pending(state, key)))
			|| super::super::drain::active(state, key)
	})
}

/// Whether a recovered publication that creates `key`, a restore's or a fork's, is still to
/// replay: queued, waiting for its target's apply, or in flight.
fn publication_pending(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	publish_op_pending(state, &key.0.volume_key, |publish| {
		publish.spec.operation_id == key.0.volume_incarnation
	})
}

/// Whether a recovered restore publication that replaces `key` is still to replay.
fn restore_pending(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	publish_op_pending(state, &key.0.volume_key, |publish| {
		publish.spec.restore == Some(key.0.volume_incarnation)
	})
}

/// Whether a recovered `ForkPublishOp` of `name` matching `matches` is still to replay:
/// queued, waiting for its target's apply, or in flight.
fn publish_op_pending(
	state: &MaintainerState,
	name: &depot_types::VolumeKey,
	matches: impl Fn(&ForkPublishOp) -> bool,
) -> bool {
	let publishes = |op: &MaintainerOp| match op {
		MaintainerOp::ForkPublishOp(publish) => {
			publish.spec.destination_volume_key == *name && matches(publish)
		}
		MaintainerOp::CheckpointOp(_)
		| MaintainerOp::RetentionCheckpointOp(_)
		| MaintainerOp::ForkReferenceOp(_)
		| MaintainerOp::ForkFailOp(_)
		| MaintainerOp::RepairAuditOp(_) => false,
	};
	let replay = &state.maintainer_log.replay;
	replay.queue.iter().any(|pending| publishes(&pending.op))
		|| replay
			.by_volume
			.range(super::super::incarnations(name))
			.flat_map(|(_, ops)| ops.iter())
			.any(|pending| publishes(&pending.op))
		|| replay
			.applying
			.keys()
			.any(|token| super::op(state, *token).is_some_and(|op| publishes(&op)))
}

/// One replayed op's store apply finished.
pub(crate) fn finished(state: &mut MaintainerState, token: LogToken, fx: &mut Effects) {
	let Some((volume, recovered)) = state.maintainer_log.replay.applying.remove(&token) else {
		return;
	};
	let applying_key = state.maintainer_log.replay.applying_keys.remove(&token);
	let volume_key = applying_key.or_else(|| volume.as_ref().map(|key| key.0.volume_key.clone()));
	super::applied(state, token, fx);
	if let Some(key) = volume {
		super::super::filesystem::mark_candidate(state, &key);
		dispatch_ready(state, &key, fx);
	}
	if recovered {
		recovered_done(state, volume_key, fx);
	}
}

/// One recovered op of `key` is done. Work that waited for the volume's replay may start
/// once none of its ops is left, and all held work once every recovered op is done.
fn recovered_done(
	state: &mut MaintainerState,
	key: Option<depot_types::VolumeKey>,
	fx: &mut Effects,
) {
	state.maintainer_log.replay.recovered -= 1;
	if !replaying(state) {
		super::super::logged_ops::replay_finished(state, fx);
		return;
	}
	let Some(key) = key else {
		return;
	};
	if !pending_for(state, &key) {
		super::super::logged_ops::volume_replayed(state, &key, fx);
		return;
	}
	// The op may have written the row an incarnation of the name waits for: offer the
	// name's waiting incarnations again, except ones whose own publication is still to
	// replay. One whose row still names another incarnation waits again.
	let offered = state
		.filesystem
		.awaiting_replay
		.range(super::super::incarnations(&key))
		.filter(|waiting| !publication_pending(state, waiting))
		.cloned()
		.collect::<Vec<_>>();
	for waiting in offered {
		state.filesystem.awaiting_replay.remove(&waiting);
	}
	super::super::filesystem::load_incarnations(state, &key);
}

/// Whether recovered ops of the volume key `scope` names are still to apply, so store work
/// on that scope's rows waits.
pub(crate) fn pending_for_scope(
	state: &MaintainerState,
	scope: &depot_fs_core::keys::slatedb::Scope,
) -> bool {
	let replay = &state.maintainer_log.replay;
	if replay.recovered == 0 {
		return false;
	}
	let matches = |key: &depot_types::VolumeKey| {
		depot_types::partition_key_hash(key) == scope.hash
			&& key.name.as_deref().map(|name| name.to_vec()) == scope.name
	};
	replay
		.queue
		.iter()
		.filter(|pending| pending.recovered)
		.filter_map(|pending| volume_key(&pending.op))
		.any(|key| matches(key))
		|| replay.by_volume.iter().any(|(key, ops)| {
			matches(&key.0.volume_key) && ops.iter().any(|pending| pending.recovered)
		}) || replay
		.applying
		.iter()
		.filter(|(_, (_, recovered))| *recovered)
		.any(|(token, (volume, _))| {
			volume
				.as_ref()
				.is_some_and(|volume| matches(&volume.0.volume_key))
				|| replay
					.applying_keys
					.get(token)
					.is_some_and(|key| matches(key))
		})
}
