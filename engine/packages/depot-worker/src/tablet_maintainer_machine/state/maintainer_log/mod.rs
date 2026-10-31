//! The tablet maintainer's own durable log.
//!
//! Rows the maintainer writes on its own, such as a checkpoint without a holder or a fork's
//! hold or publication, have no volume log whose replay could restore them after a crash. Each becomes an op in an entry of this log instead,
//! durable on a quorum of the tablet's replica set before it is applied to the store, and
//! made cold later by an ordinary pass, exactly like volume data.
//!
//! The log is a reserved volume log (`MaintainerLogVolume`) on the tablet's own replica set.
//! The maintainer is its only writer. Every request carries the tablet grant and a client
//! claim at an epoch whose high 32 bits are the grant, so a maintainer of a superseded grant
//! is refused by the shards and every lifetime's claim sorts above every earlier lifetime's.
//!
//! Recovery runs once per maintainer start, after the opening listing: load the replicas,
//! claim a quorum, read the longest log a claim quorum holds, append a claim entry, and
//! wait for it to commit. Every entry above the store's recorded cold position is then
//! replayed (`replay`). A split child also reads its parent's log, whose untrimmed ops the
//! frozen parent could not apply before the split copied its store.
//!
//! Trim follows cold, as for any volume: an entry is trimmed only after every op in it was
//! applied before a pass that completed, every volume an op names has its own cold at or
//! above the op's position, and a pass wrote the new cold position to the store.

pub(crate) mod replay;

use super::{MaintainerPhase, MaintainerState, Operation, fence, membership};
use crate::tablet_maintainer_machine::types::MAX_MAINTAINER_LOG_BYTES;
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::{
		PayloadCodec, PayloadWrite, VolumeLogEntry, VolumeLogEntryBody, VolumeLogReplicaState,
	},
	depot_maintainer_log_v1::{MaintainerEntry, MaintainerLogRecord, MaintainerOp},
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest as Request,
		StorageWorkerResponse as Response,
	},
	shared_depot_types_v1::{
		ClientIdentity, TabletPhase, VolumeLogPosition, VolumeLogReplicaStatus,
	},
};
use rivet_state_machine::Entropy;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use vbare::Versioned;

/// The maintainer log of `tablet`: its reserved volume at the start of the tablet's range.
pub(crate) fn log_key(tablet_id: [u8; 16], range_start: [u8; 16]) -> VolumeLogKey {
	let volume = crate::keys::partition::MaintainerLogVolume::new(tablet_id, range_start);
	VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: volume.volume_key(),
		volume_incarnation: tablet_id,
	})
}

/// Whether `key` names a maintainer log of any tablet. Such a key is never a client volume:
/// admission refuses it and a listing hands it to this module.
pub(crate) fn is_log_volume(key: &depot_types::VolumeKey) -> bool {
	crate::keys::partition::MaintainerLogVolume::from_volume_key(key).is_ok()
}

/// One op's place in the log: the entry it rides and its index there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct LogToken {
	pub seq: u64,
	pub index: u32,
}

/// Who waits for an op to commit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Waiter {
	/// The store operation that owns the op. It keeps its volume's footprint while the op is
	/// in flight, which holds the volume's apply at the op's position, and applies the op once
	/// it commits.
	Store(OpId),
	/// An op the replay queue applies once it commits: a recovered parent's op logged again
	/// here, or a scheduled retention checkpoint.
	Replay,
	/// A fork destination's publication, which the fork job applies once it commits.
	Fork(depot_types::VolumeKey),
	/// The failure of a fork whose publication was logged. The fork job writes the failure
	/// and replies once it commits, and applies it when the job ends.
	ForkFailed(depot_types::VolumeKey),
}

/// One op of an entry, with what trim needs to know about it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct OpSlot {
	pub op: MaintainerOp,
	/// The volume and position trim waits on, when the op has one; see `replay::trim_target`.
	pub target: Option<(VolumeLogKey, VolumeLogPosition)>,
	pub applied: bool,
	pub waiter: Option<Waiter>,
}

/// One entry above the log's cold position.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Entry {
	pub entry: VolumeLogEntry,
	pub ops: Vec<OpSlot>,
	/// The pass dispatch count when the last op was applied. A pass dispatched later covers
	/// every op of the entry once it completes.
	pub applied_at: Option<u64>,
	/// When this lifetime appended or recovered the entry, for the tiering age.
	pub since_boot_ms: u64,
}

impl Entry {
	pub(crate) fn position(&self) -> VolumeLogPosition {
		VolumeLogPosition {
			volume_seq: self.entry.volume_seq,
			client_epoch: self.entry.client_epoch,
		}
	}

	fn bytes(&self) -> u64 {
		depot_types::log_shard::entry_bytes(&self.entry)
	}
}

/// One member's replica of the log, as this lifetime last observed it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Member {
	pub state: Option<VolumeLogReplicaState>,
	/// The request in flight to this member. At most one is, so requests to one member
	/// apply in the order sent.
	pub in_flight: Option<OpId>,
	/// The member failed or refused; the retry timer clears it.
	pub waiting: bool,
	/// The trim in flight to this member. A trim only releases entries below every position
	/// an append names, so it runs beside the member's other request, and a slow trim never
	/// holds up appends.
	pub trimming: Option<OpId>,
	/// An ordered request was sent while the trim was in flight, so the trim's reply may
	/// describe the member before that request applied.
	pub sent_after_trim: bool,
	/// The last trim failed; the retry timer clears it.
	pub trim_waiting: bool,
	/// The member answered that it holds no replica.
	pub absent: bool,
}

/// Recovery and serving progress.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum LogPhase {
	/// Nothing sent yet: the store record or the opening listing is outstanding.
	#[default]
	Idle,
	/// Members load and claim for `client`. Once a quorum of in-sync replicas is claimed, the
	/// longest log among them is the log's history.
	Claiming,
	/// Reading the fixed history from `source`, which holds it through `through`.
	Reading {
		source: WorkerId,
		through: VolumeLogPosition,
	},
	/// The history is known and the claim entry at `claim_entry` is being replicated.
	CommittingClaimEntry { claim_entry: u64 },
	/// A split child reads its parent's log before it serves.
	ReadingParent(ParentRead),
	/// A split child logged its parent's ops again here and waits for that entry to commit
	/// before it serves, so a later lifetime that finds the ops in this log, or finds this
	/// tablet's own store record, knows the import happened.
	CommittingImport,
	/// The log takes new ops.
	Serving,
}

/// A split child's read of its parent's log, from every member and then from the member
/// holding the longest log.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ParentRead {
	pub key: VolumeLogKey,
	/// Members' replica states, or `None` for a member that holds no replica.
	pub answered: BTreeMap<WorkerId, Option<VolumeLogReplicaState>>,
	/// Once a quorum answered: the member read from and the next sequence to read.
	pub reading: Option<(WorkerId, u64, u64)>,
	pub entries: Vec<VolumeLogEntry>,
}

/// The maintainer log record a pass stages with its volume records, at the record row of
/// the log volume `volume_key`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordWrite {
	pub volume_key: depot_types::VolumeKey,
	pub record: MaintainerLogRecord,
	/// The `depot.maintainer_log` version the record row is encoded at. The record's own
	/// marks cover it.
	pub version: u16,
}

/// What a request of this module asked for, so its reply is read in context.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum LogRequest {
	Load,
	Claim,
	Trim {
		cold: VolumeLogPosition,
	},
	Truncate,
	/// An append whose last entry is at `through`.
	Append {
		through: u64,
	},
	MarkInSync,
	/// A page of recovery's own history.
	ReadTail,
	/// A page or status read of the parent's log.
	ReadParent,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaintainerLog {
	pub(crate) phase: LogPhase,
	/// The store's record, as read at opening. `None` until the cold store opened.
	pub(crate) record: Option<MaintainerLogRecord>,
	/// The store held a record this tablet wrote, not one copied from a split parent.
	pub(crate) own_record_stored: bool,
	/// Members listed holding another tablet's maintainer log in this tablet's range: a
	/// retired split parent's, which its cleanup forgets once every child imported it.
	pub(crate) foreign: BTreeMap<VolumeLogKey, BTreeSet<WorkerId>>,
	/// This lifetime read the split parent's log and logged its ops again here, so the
	/// import's commit makes them this tablet's own.
	pub(crate) parent_read: bool,
	/// Everything through this position is in the store's cold record and may be trimmed.
	pub(crate) cold: VolumeLogPosition,
	/// The writer identity of this lifetime, drawn when recovery starts.
	pub(crate) client: Option<ClientIdentity>,
	pub(crate) members: BTreeMap<WorkerId, Member>,
	/// Every entry above `cold` this lifetime knows, committed or not, in order.
	pub(crate) entries: VecDeque<Entry>,
	/// The highest committed sequence.
	pub(crate) committed: u64,
	/// Ops submitted since the last entry was built. They ride the next entry together.
	pub(crate) open: Vec<OpSlot>,
	/// Pass dispatches so far; see `Entry::applied_at`.
	pub(crate) pass_dispatches: u64,
	/// The highest dispatch count of a pass that completed.
	pub(crate) passes_completed: u64,
	/// The cold position a dispatched pass is writing to the store, by its dispatch count.
	pub(crate) writing: BTreeMap<u64, VolumeLogPosition>,
	/// The oldest store write outside any log that only an ordinary pass makes durable, such
	/// as a deleted record's removal, and the dispatch count a covering pass must exceed.
	pub(crate) uncovered: Option<Uncovered>,
	/// A tiering pass is owed to the log's uncovered entries.
	pub(crate) pass_wanted: bool,
	/// When `MaintainerTimer::MaintainerLogDue` is armed for.
	pub(crate) due_at: Option<u64>,
	pub(crate) replay: replay::Replay,
	/// Recovery found no replica of the log on a quorum of complete listings, so nothing
	/// exists above cold and missing replicas are created in sync.
	pub(crate) fresh: bool,
}

/// Store writes that wait for an ordinary pass to make them durable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Uncovered {
	/// When the oldest of them was written, which starts its tiering age.
	pub since_boot_ms: u64,
	/// The pass dispatch count when the newest of them was written. A pass dispatched after
	/// it covers them all once it completes.
	pub through_dispatch: u64,
}

impl Default for MaintainerLog {
	fn default() -> Self {
		Self {
			phase: LogPhase::Idle,
			record: None,
			own_record_stored: false,
			foreign: BTreeMap::new(),
			parent_read: false,
			cold: zero(),
			client: None,
			members: BTreeMap::new(),
			entries: VecDeque::new(),
			committed: 0,
			open: Vec::new(),
			pass_dispatches: 0,
			passes_completed: 0,
			writing: BTreeMap::new(),
			uncovered: None,
			pass_wanted: false,
			due_at: None,
			replay: Default::default(),
			fresh: false,
		}
	}
}

fn zero() -> VolumeLogPosition {
	VolumeLogPosition {
		volume_seq: 0,
		client_epoch: 0,
	}
}

impl MaintainerLog {
	/// The position of the newest entry, or cold when there is none.
	pub(crate) fn appended(&self) -> VolumeLogPosition {
		self.entries
			.back()
			.map(Entry::position)
			.unwrap_or_else(|| self.cold.clone())
	}

	/// The position this log holds at `seq`, or `None` when it holds none there.
	fn position(&self, seq: u64) -> Option<VolumeLogPosition> {
		if seq == self.cold.volume_seq {
			return Some(self.cold.clone());
		}
		let first = self.entries.front()?.entry.volume_seq;
		let index = usize::try_from(seq.checked_sub(first)?).ok()?;
		self.entries.get(index).map(Entry::position)
	}

	fn entry_index(&self, seq: u64) -> Option<usize> {
		let first = self.entries.front()?.entry.volume_seq;
		let index = usize::try_from(seq.checked_sub(first)?).ok()?;
		(index < self.entries.len()).then_some(index)
	}

	/// Encoded bytes of the entries above cold and of the ops waiting for an entry.
	pub(crate) fn bytes(&self) -> u64 {
		self.entries.iter().map(Entry::bytes).sum::<u64>() + self.open.len() as u64 * OPEN_OP_BYTES
	}

	/// Whether the log takes new ops.
	pub(crate) fn serving(&self) -> bool {
		self.phase == LogPhase::Serving
	}
}

/// What one op waiting for an entry counts against the byte bound before it is encoded.
const OPEN_OP_BYTES: u64 = 512;

/// The log key of this maintainer's tablet.
pub(crate) fn own_key(state: &MaintainerState) -> VolumeLogKey {
	log_key(state.tablet.id, state.tablet.hash_range.start)
}

/// The cold store opened and returned the tablet's record of its log, if any. A record of
/// another tablet is a split parent's, copied with its store: its cold position does not
/// count, since the parent's log is a different log. Its version marks do, since the child's
/// store holds the parent's data; the record kept here carries them into the child's first
/// record write. Opening checked the marks before this.
pub(crate) fn record_read(state: &mut MaintainerState, mut record: Option<MaintainerLogRecord>) {
	// Decoded marks may be in any order; raising them relies on family order.
	if let Some(record) = &mut record {
		rivet_data::families::normalize_marks(&mut record.version_marks);
	}
	if let Some(record) = &record
		&& record.tablet_id == state.tablet.id
	{
		state.maintainer_log.cold = record.cold.clone();
		state.maintainer_log.own_record_stored = true;
	}
	state.maintainer_log.record = Some(record.unwrap_or(MaintainerLogRecord {
		tablet_id: state.tablet.id,
		cold: zero(),
		version_marks: Vec::new(),
	}));
}

/// Takes a listed replica of any maintainer log out of the volume listing. Returns whether
/// `key` was one.
pub(crate) fn listed(
	state: &mut MaintainerState,
	worker: &WorkerId,
	key: &VolumeLogKey,
	replica: &VolumeLogReplicaState,
) -> bool {
	if !is_log_volume(&key.0.volume_key) {
		return false;
	}
	// The log's own claims use an epoch space separate from volume client claims and do not
	// raise the claim floor.
	if *key == own_key(state) {
		state
			.maintainer_log
			.members
			.entry(worker.clone())
			.or_default()
			.state
			.get_or_insert(replica.clone());
	} else {
		state
			.maintainer_log
			.foreign
			.entry(key.clone())
			.or_default()
			.insert(worker.clone());
		// A previously listed disk may have been skipped while excluded, without ever
		// answering its forget. Every listing of its retained replica owes cleanup again.
		super::retired::discovered(state, key, worker.clone());
	}
	true
}

/// Whether recovery finished and the maintainer may leave `Opening`.
pub(crate) fn ready(state: &MaintainerState) -> bool {
	state.maintainer_log.phase == LogPhase::Serving
}

/// Starts recovery once the opening listing reached a quorum and the store record is known.
/// Members the listing found without the log's replica create it empty and in sync when no
/// listed member holds one, so a brand-new log needs no repair; otherwise a missing replica
/// starts repairing.
fn start(state: &mut MaintainerState, entropy: &Entropy, fx: &mut Effects) {
	let log = &mut state.maintainer_log;
	if log.phase != LogPhase::Idle || log.record.is_none() {
		return;
	}
	let listed_quorum = membership::quorum(&state.tablet.replica_set, |w| state.listed.contains(w));
	if !listed_quorum {
		return;
	}
	let floor = log
		.members
		.values()
		.filter_map(|member| member.state.as_ref())
		.map(epoch_floor)
		.max()
		.unwrap_or(0)
		.max(log.cold.client_epoch);
	let Some(client_epoch) = log_claim_epoch(floor, state.tablet.tablet_grant_epoch) else {
		tracing::error!(
			grant_epoch = state.tablet.tablet_grant_epoch,
			floor,
			"maintainer log claim epoch is exhausted for this grant; the coordinator must not grant above u32::MAX"
		);
		fence(state, fx);
		return;
	};
	let log = &mut state.maintainer_log;
	log.client = Some(ClientIdentity {
		client_epoch,
		client_incarnation: *entropy.clone().rng().uuid_v4().as_bytes(),
	});
	log.fresh = !log.members.values().any(|member| member.state.is_some());
	for worker in membership::destinations(&state.tablet.replica_set) {
		log.members.entry(worker).or_default();
	}
	log.phase = LogPhase::Claiming;
}

/// The log's client epoch for a lifetime under `grant_epoch`, above `floor`, or `None` once
/// the grant no longer fits its band. The listing that sets the floor may miss a member, so
/// an earlier lifetime may have claimed that member above it. The grant epoch in the high 32
/// bits orders every grant's claims above all earlier grants' claims, and every lifetime runs
/// under a grant no earlier lifetime held (see `claim_epochs`). `floor + 1` stays the bound
/// that keeps the epoch above everything observed.
fn log_claim_epoch(floor: u64, grant_epoch: u64) -> Option<u64> {
	let band = super::claim_epochs::band(grant_epoch)?;
	Some(floor.checked_add(1)?.max(band))
}

/// The highest epoch a replica holds or was claimed for.
pub(crate) fn epoch_floor(replica: &VolumeLogReplicaState) -> u64 {
	replica
		.cold
		.client_epoch
		.max(replica.appended.client_epoch)
		.max(
			replica
				.claimed_client
				.as_ref()
				.map_or(0, |client| client.client_epoch),
		)
}

fn header(state: &MaintainerState) -> wire::ReplicaRequestHeader {
	wire::ReplicaRequestHeader::GrantClientRequestHeader(wire::GrantClientRequestHeader {
		volume_log_key: own_key(state).0,
		tablet_grant: state.grant(),
		client: state
			.maintainer_log
			.client
			.clone()
			.expect("recovery drew the writer identity"),
	})
}

fn grant_header(state: &MaintainerState) -> wire::ReplicaRequestHeader {
	wire::ReplicaRequestHeader::MaintainerRequestHeader(wire::MaintainerRequestHeader {
		volume_log_key: own_key(state).0,
		tablet_grant: state.grant(),
	})
}

fn send(
	state: &mut MaintainerState,
	worker: WorkerId,
	request: LogRequest,
	wire_request: Request,
	fx: &mut Effects,
) {
	let request_kind = match &request {
		LogRequest::Trim { .. } => RequestKind::Trim,
		LogRequest::Load
		| LogRequest::Claim
		| LogRequest::Truncate
		| LogRequest::Append { .. }
		| LogRequest::MarkInSync
		| LogRequest::ReadTail
		| LogRequest::ReadParent => RequestKind::Ordered,
	};
	let op_id = state.begin(
		Operation::MaintainerLog {
			worker: worker.clone(),
			request,
		},
		fx,
	);
	let member = state
		.maintainer_log
		.members
		.entry(worker.clone())
		.or_default();
	if matches!(request_kind, RequestKind::Trim) {
		member.trimming = Some(op_id);
		// A request already in flight may apply after the trim, too.
		member.sent_after_trim = member.in_flight.is_some();
	} else {
		member.in_flight = Some(op_id);
		if member.trimming.is_some() {
			member.sent_after_trim = true;
		}
	}
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: worker,
		request: wire_request,
	});
}

/// Which slot of a member a request occupies.
enum RequestKind {
	Trim,
	Ordered,
}

/// Submits `op` to ride the next entry and returns its place in the log. The caller checks
/// `admits` first. `target` names the volume and the applied position the op follows.
pub(crate) fn submit(
	state: &mut MaintainerState,
	op: MaintainerOp,
	target: Option<(VolumeLogKey, VolumeLogPosition)>,
	waiter: Waiter,
) -> LogToken {
	let log = &mut state.maintainer_log;
	let token = LogToken {
		seq: log.appended().volume_seq + 1,
		index: log.open.len() as u32,
	};
	log.open.push(OpSlot {
		op,
		target,
		applied: false,
		waiter: Some(waiter),
	});
	token
}

/// The op at `token`, while its entry is above cold.
pub(crate) fn op(state: &MaintainerState, token: LogToken) -> Option<MaintainerOp> {
	let log = &state.maintainer_log;
	let slot = match log.entry_index(token.seq) {
		Some(index) => log.entries[index].ops.get(token.index as usize),
		// Not built into an entry yet.
		None if token.seq == log.appended().volume_seq + 1 => log.open.get(token.index as usize),
		None => None,
	}?;
	Some(slot.op.clone())
}

/// Whether the log takes another op now. A full log refuses until passes trim it.
pub(crate) fn admits(state: &MaintainerState) -> bool {
	state.maintainer_log.serving() && state.maintainer_log.bytes() < MAX_MAINTAINER_LOG_BYTES
}

/// Builds the next entry from the waiting ops and advances every member.
pub(crate) fn drive(state: &mut MaintainerState, entropy: &Entropy, fx: &mut Effects) {
	if matches!(
		state.phase,
		MaintainerPhase::Fenced | MaintainerPhase::Stopped | MaintainerPhase::Draining
	) {
		return;
	}
	start(state, entropy, fx);
	if state.maintainer_log.phase == LogPhase::Idle {
		return;
	}
	// A commit can finish a recovery step, and a recovery step can build an entry, so the
	// steps repeat until a pass changes nothing; each phase advances at most once per pass.
	loop {
		let before = (
			state.maintainer_log.phase.clone(),
			state.maintainer_log.committed,
			state.maintainer_log.entries.len(),
		);
		commit(state, fx);
		recovery_step(state, fx);
		build_entry(state, fx);
		let workers = state
			.maintainer_log
			.members
			.keys()
			.cloned()
			.collect::<Vec<_>>();
		for worker in workers {
			advance_member(state, worker, fx);
		}
		let after = (
			&state.maintainer_log.phase,
			state.maintainer_log.committed,
			state.maintainer_log.entries.len(),
		);
		if (&before.0, before.1, before.2) == after {
			break;
		}
	}
	replay::drive(state, fx);
	arm_due(state, fx);
}

/// Moves the waiting ops into one new entry, at most one entry per drive. A member's append
/// carries every entry it lacks, so entries built while an append is in flight ride the next.
fn build_entry(state: &mut MaintainerState, fx: &mut Effects) {
	let log = &mut state.maintainer_log;
	if log.open.is_empty() || !matches!(log.phase, LogPhase::CommittingImport | LogPhase::Serving) {
		return;
	}
	let Some(client) = log.client.clone() else {
		return;
	};
	let ops = std::mem::take(&mut log.open);
	let version = state.config.write_versions.maintainer_log;
	let payload = match encode_entry(&ops, version) {
		Ok(payload) => payload,
		Err(error) => {
			tracing::error!(
				?error,
				version,
				"maintainer log entry does not encode at the grant's version; fencing"
			);
			super::fence(state, fx);
			return;
		}
	};
	let seq = log.appended().volume_seq + 1;
	let entry = log_entry(
		seq,
		client.client_epoch,
		&client,
		payload,
		version,
		fx.clocks().unix_ms,
	);
	log.entries.push_back(Entry {
		entry,
		ops,
		applied_at: None,
		since_boot_ms: fx.now_boot_ms(),
	});
}

/// Encodes an entry's ops at `version`, the grant's `depot.maintainer_log` version, which
/// `MaintainerState::new` checked this binary writes. Converting down to an older version
/// can still refuse a value, so the caller handles the error.
fn encode_entry(ops: &[OpSlot], version: u16) -> anyhow::Result<Vec<u8>> {
	let entry = MaintainerEntry {
		ops: ops.iter().map(|slot| slot.op.clone()).collect(),
	};
	rivet_data::versioned::DepotMaintainerEntryData::serialize_with_embedded_version(
		&entry, version,
	)
}

fn log_entry(
	seq: u64,
	epoch: u64,
	client: &ClientIdentity,
	payload: Vec<u8>,
	payload_version: u16,
	now_unix_ms: i64,
) -> VolumeLogEntry {
	// The same digest a volume client computes for a payload write.
	let payload_digest = depot_types::payload_codec::payload_digest(&payload);
	let mut request_id = client.client_incarnation;
	request_id[..8].copy_from_slice(&seq.to_be_bytes());
	VolumeLogEntry {
		written_at_unix_ms: now_unix_ms,
		volume_seq: seq,
		client_epoch: epoch,
		body: VolumeLogEntryBody::PayloadWrite(PayloadWrite {
			// `encode_entry` sealed the payload at this version.
			payload_family: rivet_data::families::VersionFamily::DEPOT_MAINTAINER_LOG.0,
			payload_version,
			client_seq: seq,
			request_id,
			payload_digest,
			codec: PayloadCodec::None,
			payload: payload.into(),
		}),
	}
}

/// Decodes the ops of a recovered entry. A claim entry holds none.
fn decode_entry(entry: &VolumeLogEntry) -> anyhow::Result<Vec<MaintainerOp>> {
	match &entry.body {
		VolumeLogEntryBody::ClaimEntry => Ok(Vec::new()),
		VolumeLogEntryBody::PayloadWrite(write) => {
			let payload = depot_types::payload_codec::decode(write)?;
			Ok(
				rivet_data::versioned::DepotMaintainerEntryData::deserialize_with_embedded_version(
					&payload,
				)?
				.ops,
			)
		}
	}
}

/// Sends a member the one request its replica needs next, if none is in flight.
fn advance_member(state: &mut MaintainerState, worker: WorkerId, fx: &mut Effects) {
	let Some(member) = state.maintainer_log.members.get(&worker) else {
		return;
	};
	// A replica that holds the log above its cold position but lags in cold is trimmed in
	// the member's trim slot, beside whatever else it is sent.
	let trims = member.trimming.is_none()
		&& !member.trim_waiting
		&& !member.absent
		&& matches!(
			state.maintainer_log.phase,
			LogPhase::Serving | LogPhase::CommittingImport
		) && member.state.as_ref().is_some_and(|replica| {
		replica.status == VolumeLogReplicaStatus::InSync
			&& replica.cold.volume_seq < state.maintainer_log.cold.volume_seq
			&& replica.appended.volume_seq >= state.maintainer_log.cold.volume_seq
	});
	if trims {
		trim_member(state, worker.clone(), fx);
	}
	let Some(member) = state.maintainer_log.members.get(&worker) else {
		return;
	};
	if member.in_flight.is_some() || member.waiting {
		return;
	}
	let Some(disk_incarnation) = state.disk_incarnation(&worker) else {
		return;
	};
	let log = &state.maintainer_log;
	let client = log.client.clone().expect("recovering log has a writer");
	let Some(replica) = member.state.clone().filter(|_| !member.absent) else {
		// Load the replica at the log's cold position. Loading an existing replica only
		// reports it, so a repairing status is safe to send to every member.
		let status = if log.fresh {
			VolumeLogReplicaStatus::InSync
		} else {
			VolumeLogReplicaStatus::Repairing
		};
		// Once the history is known, a replica created now, such as a replacement member's,
		// is created claimed by the writer. A separate claim could not fence it: the writer's
		// epoch already equals the epoch of the cold position the replica starts at.
		let history_known = matches!(
			log.phase,
			LogPhase::CommittingClaimEntry { .. }
				| LogPhase::ReadingParent(_)
				| LogPhase::CommittingImport
				| LogPhase::Serving
		);
		let claimed_client = if history_known {
			Some(client.clone())
		} else {
			None
		};
		let request = Request::LoadVolumeRequest(wire::LoadVolumeRequest {
			open_claim: None,
			learner_replica_set_version: None,
			disk_incarnation,
			claimed_client,
			claimed_holder: None,
			header: grant_header(state),
			cold: log.cold.clone(),
			status,
			initial_entries: Vec::new(),
			creation: None,
		});
		send(state, worker, LogRequest::Load, request, fx);
		return;
	};
	if replica.claimed_client.as_ref() != Some(&client) {
		let request = Request::ClaimRequest(wire::ClaimRequest {
			header: header(state),
			holder: None,
		});
		send(state, worker, LogRequest::Claim, request, fx);
		return;
	}
	// Until the history is known, claiming is all a member does.
	if !matches!(
		log.phase,
		LogPhase::CommittingClaimEntry { .. }
			| LogPhase::ReadingParent(_)
			| LogPhase::CommittingImport
			| LogPhase::Serving
	) {
		return;
	}
	// A replica whose tail ends below the log's cold position holds nothing the log can
	// match, so it moves up to cold first. One that lags only in cold catches up on entries
	// first and trims last, so a member that cannot trim still counts toward commits.
	if replica.appended.volume_seq < log.cold.volume_seq {
		if member.trimming.is_none() && !member.trim_waiting {
			trim_member(state, worker, fx);
		}
		return;
	}
	let appended = log.appended();
	let matches = replica.status == VolumeLogReplicaStatus::InSync
		&& log.position(replica.appended.volume_seq).as_ref() == Some(&replica.appended);
	let repairing_match = replica.status == VolumeLogReplicaStatus::Repairing
		&& log.position(replica.appended.volume_seq).as_ref() == Some(&replica.appended);
	if !matches && !repairing_match {
		// A divergent or unknown suffix: empty the replica down to its cold position, then
		// copy the log into it again. The truncate names the replica incarnation observed
		// here, so a replica recreated since is refused and observed again instead of emptied.
		let request = Request::TruncateRequest(wire::TruncateRequest {
			expected_replica_incarnation: Some(replica.replica_incarnation),
			header: header(state),
			expected_appended: replica.appended.clone(),
			keep_through_volume_seq: replica.cold.volume_seq,
			mark_repairing: true,
		});
		send(state, worker, LogRequest::Truncate, request, fx);
		return;
	}
	if replica.appended.volume_seq < appended.volume_seq {
		let Some(first) = state
			.maintainer_log
			.entry_index(replica.appended.volume_seq + 1)
		else {
			return;
		};
		let entries = state
			.maintainer_log
			.entries
			.iter()
			.skip(first)
			.take(MAX_APPEND_ENTRIES)
			.map(|entry| entry.entry.clone())
			.collect::<Vec<_>>();
		let through = entries.last().map_or(appended.volume_seq, |e| e.volume_seq);
		let request = Request::AppendRequest(wire::AppendRequest {
			header: header(state),
			previous: replica.appended.clone(),
			entries,
			committed_volume_seq: state.maintainer_log.committed,
		});
		send(state, worker, LogRequest::Append { through }, request, fx);
		return;
	}
	if repairing_match && replica.appended == appended {
		let request = Request::MarkInSyncRequest(wire::MarkInSyncRequest {
			header: header(state),
			replica_incarnation: replica.replica_incarnation,
			expected_appended: replica.appended.clone(),
		});
		send(state, worker, LogRequest::MarkInSync, request, fx);
	}
}

/// Trims a member's replica to the log's cold position.
fn trim_member(state: &mut MaintainerState, worker: WorkerId, fx: &mut Effects) {
	let cold = state.maintainer_log.cold.clone();
	let request = Request::TrimRequest(wire::TrimRequest {
		header: grant_header(state),
		cold: cold.clone(),
		claimed_client: None,
	});
	send(state, worker, LogRequest::Trim { cold }, request, fx);
}

/// Entries one append carries at most; a lagging member catches up over several.
const MAX_APPEND_ENTRIES: usize = 256;

/// Fixes the history once a claim quorum answered, reads it, and appends the claim entry.
fn recovery_step(state: &mut MaintainerState, fx: &mut Effects) {
	match state.maintainer_log.phase.clone() {
		LogPhase::Claiming => {
			let client = state.maintainer_log.client.clone();
			let claimed = |worker: &WorkerId| {
				state
					.maintainer_log
					.members
					.get(worker)
					.and_then(|member| member.state.as_ref())
					.is_some_and(|replica| {
						replica.status == VolumeLogReplicaStatus::InSync
							&& replica.claimed_client == client
					})
			};
			if !membership::quorum(&state.tablet.replica_set, claimed) {
				return;
			}
			// The longest log of the claim quorum, newest epoch first, is what a recovery
			// commits: log matching makes every committed entry part of it.
			let best = state
				.maintainer_log
				.members
				.iter()
				.filter(|(worker, _)| claimed(worker))
				.filter_map(|(worker, member)| {
					member
						.state
						.as_ref()
						.map(|replica| (worker.clone(), replica.appended.clone()))
				})
				.max_by_key(|(_, appended)| (appended.client_epoch, appended.volume_seq));
			let Some((source, through)) = best else {
				return;
			};
			if through.volume_seq < state.maintainer_log.cold.volume_seq {
				fence(state, fx);
				return;
			}
			// Only the first loads of a log no member held may create replicas in sync; a
			// replica created once the log has history must catch up first.
			state.maintainer_log.fresh = false;
			state.maintainer_log.phase = LogPhase::Reading { source, through };
			read_tail(state, fx);
		}
		LogPhase::Reading { .. } => read_tail(state, fx),
		LogPhase::CommittingClaimEntry { claim_entry } => {
			if state.maintainer_log.committed >= claim_entry {
				after_claim_entry(state, fx);
			}
		}
		LogPhase::ReadingParent(_) => read_parent(state, fx),
		LogPhase::CommittingImport => {
			let log = &state.maintainer_log;
			if log.open.is_empty() && log.committed >= log.appended().volume_seq {
				parent_import_committed(state);
				serve(state, fx);
			}
		}
		LogPhase::Idle | LogPhase::Serving => {}
	}
}

/// Reads the fixed history from its source page by page, then appends the claim entry.
fn read_tail(state: &mut MaintainerState, fx: &mut Effects) {
	let LogPhase::Reading { source, through } = state.maintainer_log.phase.clone() else {
		return;
	};
	let have = state.maintainer_log.appended();
	if have.volume_seq >= through.volume_seq {
		// The history is in memory: append the claim entry at the writer's epoch.
		let client = state.maintainer_log.client.clone().expect("writer");
		let seq = through.volume_seq + 1;
		let entry = VolumeLogEntry {
			written_at_unix_ms: fx.clocks().unix_ms,
			volume_seq: seq,
			client_epoch: client.client_epoch,
			body: VolumeLogEntryBody::ClaimEntry,
		};
		state.maintainer_log.entries.push_back(Entry {
			entry,
			ops: Vec::new(),
			applied_at: None,
			since_boot_ms: fx.now_boot_ms(),
		});
		state.maintainer_log.phase = LogPhase::CommittingClaimEntry { claim_entry: seq };
		return;
	}
	// A source that left the replica set or failed its last read is replaced by another
	// member holding the fixed history: one appended at `through`'s epoch at or past its
	// position, since a writer at one epoch appends a contiguous run and log matching makes
	// their logs equal through it, so the pages already read stay valid. A tail read stops
	// at `through`, so a longer member serves the same pages.
	let usable = |member: &Member| member.in_flight.is_none() && !member.waiting;
	let source_usable = state
		.maintainer_log
		.members
		.get(&source)
		.is_some_and(usable);
	let source_busy = state
		.maintainer_log
		.members
		.get(&source)
		.is_some_and(|member| member.in_flight.is_some());
	let source = if source_usable {
		source
	} else if source_busy {
		return;
	} else {
		let replacement = state
			.maintainer_log
			.members
			.iter()
			.filter(|(worker, _)| **worker != source)
			.find(|(_, member)| {
				usable(member)
					&& member.state.as_ref().is_some_and(|replica| {
						replica.appended.client_epoch == through.client_epoch
							&& replica.appended.volume_seq >= through.volume_seq
					})
			})
			.map(|(worker, _)| worker.clone());
		let Some(replacement) = replacement else {
			return;
		};
		state.maintainer_log.phase = LogPhase::Reading {
			source: replacement.clone(),
			through,
		};
		replacement
	};
	let request = Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
		volume_log_key: own_key(state).0,
		first_volume_seq: have.volume_seq + 1,
		max_bytes: state.config.tuning().max_cold_read_bytes,
		include_deleted: false,
	});
	send(state, source, LogRequest::ReadTail, request, fx);
}

/// The claim entry committed: every entry before it is the log's history. Queue its ops for
/// replay, then read a split parent's log or serve.
fn after_claim_entry(state: &mut MaintainerState, fx: &mut Effects) {
	let mut ops = Vec::new();
	// An entry this build cannot decode may hold store changes, so the maintainer stops
	// rather than treat it as empty.
	let undecodable = state
		.maintainer_log
		.entries
		.iter()
		.any(|entry| entry.ops.is_empty() && decode_entry(&entry.entry).is_err());
	if undecodable {
		tracing::error!("a maintainer log entry does not decode; fencing");
		fence(state, fx);
		return;
	}
	for entry in state.maintainer_log.entries.iter_mut() {
		if entry.ops.is_empty()
			&& let Ok(decoded) = decode_entry(&entry.entry)
		{
			entry.ops = decoded
				.into_iter()
				.map(|op| OpSlot {
					target: replay::trim_target(&op),
					op,
					applied: false,
					waiter: None,
				})
				.collect();
			// The claim entry's commit counted this entry applied while its ops were still
			// undecoded. Its ops now wait for replay, so a pass must not cover it yet.
			if !entry.ops.is_empty() {
				entry.applied_at = None;
			}
		}
		for (index, slot) in entry.ops.iter().enumerate() {
			ops.push((
				LogToken {
					seq: entry.entry.volume_seq,
					index: index as u32,
				},
				slot.op.clone(),
			));
		}
	}
	let frozen = state.tablet.split.is_some();
	let imported = parent_imported(state, &ops);
	for (token, op) in ops {
		replay::recovered(state, token, op, frozen);
	}
	if imported && state.tablet.parent_tablet_id.is_some() {
		super::parent_log::imported(state);
	}
	// A frozen child reads its parent's log too: its own split children read only this
	// child's log, so the parent's ops of this range must be in it before this child retires.
	match state.tablet.parent_tablet_id {
		Some(parent) if !imported => {
			// The parent's log key is placed at the parent's range start, which the split
			// records on both children: a right child's own range and listing cannot show
			// it, and the retired parent's manifest entry may be gone by its first recovery.
			let Some(parent_start) = state.tablet.parent_range_start else {
				tracing::error!(
					tablet_id = ?state.tablet.id,
					"split child has no recorded parent range start; its parent's log cannot be read"
				);
				fence(state, fx);
				return;
			};
			state.maintainer_log.phase = LogPhase::ReadingParent(ParentRead {
				key: log_key(parent, parent_start),
				answered: BTreeMap::new(),
				reading: None,
				entries: Vec::new(),
			});
			read_parent(state, fx);
		}
		Some(_) | None => serve(state, fx),
	}
}

/// Whether an earlier lifetime of this split child already logged its parent's ops again in
/// its own log, which then replays them in order with every later op. Reading the parent
/// again would replay its ops after the child's own, so a checkpoint the child deleted after
/// the split would come back. A child logs nothing before its log serves except its
/// parent's ops, all in one entry, and serves only once that entry committed, so any op in
/// its own history proves the import. Once that entry is trimmed, the tablet's own store
/// record proves it: a maintainer runs no pass before its log serves. A parent with no op in
/// this child's range leaves no proof, and reading it again imports nothing.
fn parent_imported(state: &MaintainerState, own_ops: &[(LogToken, MaintainerOp)]) -> bool {
	!own_ops.is_empty() || state.maintainer_log.own_record_stored
}

/// Recovery finished: the log takes new ops and the maintainer may leave `Opening`.
fn serve(state: &mut MaintainerState, fx: &mut Effects) {
	state.maintainer_log.phase = LogPhase::Serving;
	super::opening::log_recovered(state, fx);
}

/// Reads the split parent's log from a quorum of members, then pages it from the member
/// holding the longest log.
fn read_parent(state: &mut MaintainerState, fx: &mut Effects) {
	let LogPhase::ReadingParent(read) = state.maintainer_log.phase.clone() else {
		return;
	};
	let workers = membership::destinations(&state.tablet.replica_set);
	if read.reading.is_none() {
		let answered = |worker: &WorkerId| read.answered.contains_key(worker);
		if membership::quorum(&state.tablet.replica_set, answered) {
			let best = read
				.answered
				.iter()
				.filter_map(|(worker, replica)| {
					replica
						.as_ref()
						.map(|replica| (worker.clone(), replica.clone()))
				})
				.max_by_key(|(_, replica)| {
					(replica.appended.client_epoch, replica.appended.volume_seq)
				});
			let reading = best.map(|(worker, replica)| {
				(
					worker,
					replica.cold.volume_seq + 1,
					replica.appended.volume_seq,
				)
			});
			match reading {
				Some((_, next, through)) if next <= through => {
					if let LogPhase::ReadingParent(read) = &mut state.maintainer_log.phase {
						read.reading = reading;
					}
				}
				Some(_) => {
					finish_parent(state, Vec::new(), fx);
					return;
				}
				None => {
					// No member of a quorum holds the parent's log. A parent that opened
					// started its log on this replica set, so this is a moved set or a stale
					// listing: serve without the import, and never report one.
					tracing::warn!(
						tablet_id = ?state.tablet.id,
						"no member holds the split parent's maintainer log; its ops are not imported"
					);
					finish_parent_without_log(state, fx);
					return;
				}
			}
		} else {
			for worker in workers {
				if read.answered.contains_key(&worker)
					|| state
						.maintainer_log
						.members
						.get(&worker)
						.is_some_and(|member| member.in_flight.is_some() || member.waiting)
				{
					continue;
				}
				let request =
					Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
						volume_log_key: read.key.0.clone(),
						first_volume_seq: 0,
						max_bytes: 0,
						include_deleted: false,
					});
				send(state, worker, LogRequest::ReadParent, request, fx);
			}
			return;
		}
	}
	let LogPhase::ReadingParent(read) = state.maintainer_log.phase.clone() else {
		return;
	};
	let Some((source, next, through)) = read.reading.clone() else {
		return;
	};
	if next > through {
		finish_parent(state, read.entries, fx);
		return;
	}
	if state
		.maintainer_log
		.members
		.get(&source)
		.is_some_and(|member| member.in_flight.is_some() || member.waiting)
	{
		return;
	}
	let request = Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
		volume_log_key: read.key.0.clone(),
		first_volume_seq: next,
		max_bytes: state.config.tuning().max_cold_read_bytes,
		include_deleted: false,
	});
	send(state, source, LogRequest::ReadParent, request, fx);
}

/// Logs the parent's ops of this child's range again here for replay, and serves once they
/// committed.
fn finish_parent(state: &mut MaintainerState, entries: Vec<VolumeLogEntry>, fx: &mut Effects) {
	for entry in &entries {
		let Ok(ops) = decode_entry(entry) else {
			tracing::error!("a split parent's maintainer log entry does not decode; fencing");
			fence(state, fx);
			return;
		};
		for op in ops {
			replay::recovered_from_parent(state, op);
		}
	}
	state.maintainer_log.parent_read = true;
	state.maintainer_log.phase = LogPhase::CommittingImport;
}

/// Serves without the parent's ops, when no member holds its log. This lifetime does not
/// report the import; a later lifetime that finds an op of its own in this log treats the
/// import as done and reports it.
fn finish_parent_without_log(state: &mut MaintainerState, fx: &mut Effects) {
	serve(state, fx);
}

/// The import committed: the parent's ops this lifetime logged again are in this tablet's
/// own log, which is what a later recovery reads instead of the parent's. A lifetime that
/// never read the parent, such as a frozen child's, imported nothing.
fn parent_import_committed(state: &mut MaintainerState) {
	if state.tablet.parent_tablet_id.is_some() && state.maintainer_log.parent_read {
		super::parent_log::imported(state);
	}
}

/// Advances the committed position to the highest entry a quorum of in-sync members claimed
/// for this writer holds, then lets each newly committed op's waiter continue.
fn commit(state: &mut MaintainerState, fx: &mut Effects) {
	let Some(client) = state.maintainer_log.client.clone() else {
		return;
	};
	let mut committed = state.maintainer_log.committed;
	let appended = state.maintainer_log.appended().volume_seq;
	while committed < appended && holds_quorum(state, &client, committed + 1) {
		committed += 1;
	}
	let previous = state.maintainer_log.committed;
	if committed == previous {
		return;
	}
	state.maintainer_log.committed = committed;
	let mut released = Vec::new();
	for entry in state.maintainer_log.entries.iter_mut() {
		let seq = entry.entry.volume_seq;
		if seq <= previous || seq > committed {
			continue;
		}
		for (index, slot) in entry.ops.iter_mut().enumerate() {
			if let Some(waiter) = slot.waiter.take() {
				released.push((
					LogToken {
						seq,
						index: index as u32,
					},
					waiter,
				));
			}
		}
	}
	for (token, waiter) in released {
		match waiter {
			Waiter::Store(op_id) => super::logged_ops::committed(state, op_id, token, fx),
			Waiter::Replay => {}
			Waiter::Fork(key) => super::forks::logged(state, key, token, fx),
			Waiter::ForkFailed(key) => super::forks::fail_logged(state, key, token, fx),
		}
	}
	// The claim entry and other entries without ops count as applied once committed.
	let now = state.maintainer_log.pass_dispatches;
	for entry in state.maintainer_log.entries.iter_mut() {
		if entry.entry.volume_seq <= committed
			&& entry.applied_at.is_none()
			&& entry.ops.iter().all(|slot| slot.applied)
		{
			entry.applied_at = Some(now);
		}
	}
}

/// Whether a quorum of in-sync members claimed for `client` holds this lifetime's log
/// through `seq`.
fn holds_quorum(state: &MaintainerState, client: &ClientIdentity, seq: u64) -> bool {
	let log = &state.maintainer_log;
	membership::quorum(&state.tablet.replica_set, |worker| {
		log.members
			.get(worker)
			.and_then(|member| member.state.as_ref())
			.is_some_and(|replica| {
				replica.status == VolumeLogReplicaStatus::InSync
					&& replica.claimed_client.as_ref() == Some(client)
					&& replica.appended.volume_seq >= seq
					&& log.position(replica.appended.volume_seq).as_ref() == Some(&replica.appended)
			})
	})
}

/// Marks one op applied to the store. An entry whose ops are all applied waits for the
/// next pass to cover it.
pub(crate) fn applied(state: &mut MaintainerState, token: LogToken, fx: &mut Effects) {
	let now = state.maintainer_log.pass_dispatches;
	let log = &mut state.maintainer_log;
	let Some(index) = log.entry_index(token.seq) else {
		return;
	};
	let entry = &mut log.entries[index];
	if let Some(slot) = entry.ops.get_mut(token.index as usize) {
		slot.applied = true;
	}
	if entry.applied_at.is_none()
		&& entry.entry.volume_seq <= log.committed
		&& entry.ops.iter().all(|slot| slot.applied)
	{
		entry.applied_at = Some(now);
	}
	arm_due(state, fx);
}

/// A pass is being dispatched. Returns its dispatch count and the record it must write: the
/// log's new cold position or the tablet's version marks, when either changed. Marks rise in the same batch as the pass's rows, so no row of a higher version
/// is durable without its mark, and raising them costs no write of its own.
pub(crate) fn pass_dispatched(state: &mut MaintainerState) -> (u64, Option<RecordWrite>) {
	let log = &mut state.maintainer_log;
	log.pass_dispatches += 1;
	let dispatch = log.pass_dispatches;
	log.pass_wanted = false;
	if !log.serving() || state.tablet.split.is_some() {
		return (dispatch, None);
	}
	let cold = trim_point(state);
	let log = &mut state.maintainer_log;
	let Some(record) = log.record.clone() else {
		return (dispatch, None);
	};
	let advanced = cold.volume_seq > record.cold.volume_seq || record.tablet_id != state.tablet.id;
	let mut version_marks = record.version_marks.clone();
	let marks_raised =
		super::version_marks::raise_written(&mut version_marks, &state.config.write_versions);
	if !advanced && !marks_raised {
		return (dispatch, None);
	}
	if marks_raised && let Some(current) = &mut log.record {
		// The pass that carries the raised marks fences the maintainer if it fails, so the
		// in-memory record may lead the store's.
		current.version_marks = version_marks.clone();
	}
	let record = MaintainerLogRecord {
		tablet_id: state.tablet.id,
		cold: if advanced { cold.clone() } else { record.cold },
		version_marks,
	};
	if advanced {
		log.writing.insert(dispatch, cold);
	}
	let volume_key = own_key(state).0.volume_key;
	let version = state.config.write_versions.maintainer_log;
	(
		dispatch,
		Some(RecordWrite {
			volume_key,
			record,
			version,
		}),
	)
}

/// The highest position every entry through which is covered and whose volumes are cold past
/// their ops' positions. Replay in progress holds it, since its ops are not applied.
fn trim_point(state: &MaintainerState) -> VolumeLogPosition {
	let log = &state.maintainer_log;
	let mut point = log
		.writing
		.values()
		.last()
		.cloned()
		.unwrap_or_else(|| log.cold.clone());
	for entry in &log.entries {
		if entry.entry.volume_seq <= point.volume_seq {
			continue;
		}
		let applied = entry.ops.iter().all(|slot| slot.applied);
		let covered = applied
			&& entry
				.applied_at
				.is_some_and(|applied_at| log.passes_completed > applied_at);
		let volumes_cold = entry.ops.iter().all(|slot| match &slot.target {
			Some((key, after)) => replay::durable_cold(state, key) >= after.volume_seq,
			None => true,
		});
		if !covered || !volumes_cold || entry.entry.volume_seq > log.committed {
			break;
		}
		point = entry.position();
	}
	point
}

/// Records a store write that only an ordinary pass makes durable, and returns the dispatch
/// count a covering pass must exceed. The write rides the tablet's next tiering pass, at the
/// same age as the log's own entries; nothing asks for an upload of its own.
pub(crate) fn wrote_uncovered(state: &mut MaintainerState, fx: &mut Effects) -> u64 {
	let log = &mut state.maintainer_log;
	let through_dispatch = log.pass_dispatches;
	match &mut log.uncovered {
		Some(uncovered) => uncovered.through_dispatch = through_dispatch,
		None => {
			log.uncovered = Some(Uncovered {
				since_boot_ms: fx.now_boot_ms(),
				through_dispatch,
			});
		}
	}
	arm_due(state, fx);
	through_dispatch
}

/// Counts a pass that flushes the store without writing the log's record, such as a pass of
/// creation and deletion rows. It covers store writes made before it like any other pass.
pub(crate) fn store_pass_dispatched(state: &mut MaintainerState) -> u64 {
	state.maintainer_log.pass_dispatches += 1;
	state.maintainer_log.pass_dispatches
}

/// A pass dispatched as `dispatch` completed. Its record is in the store, so the log trims to
/// the cold position it wrote.
pub(crate) fn pass_completed(state: &mut MaintainerState, dispatch: u64, fx: &mut Effects) {
	let log = &mut state.maintainer_log;
	log.passes_completed = log.passes_completed.max(dispatch);
	if log
		.uncovered
		.as_ref()
		.is_some_and(|uncovered| log.passes_completed > uncovered.through_dispatch)
	{
		log.uncovered = None;
	}
	let Some(cold) = log.writing.remove(&dispatch) else {
		arm_due(state, fx);
		return;
	};
	// Older writes in flight are covered by this one.
	log.writing.retain(|older, _| *older > dispatch);
	if cold.volume_seq <= log.cold.volume_seq {
		arm_due(state, fx);
		return;
	}
	log.cold = cold.clone();
	if let Some(record) = &mut log.record {
		record.cold = cold.clone();
		record.tablet_id = state.tablet.id;
	}
	while log
		.entries
		.front()
		.is_some_and(|entry| entry.entry.volume_seq <= cold.volume_seq)
	{
		log.entries.pop_front();
	}
	// An unload waits until no entry names its volume; see `names_volume`.
	super::unload::mark_all(state);
	arm_due(state, fx);
}

/// Whether an entry above the log's cold position, or an op waiting for one, changes
/// `key`'s store rows. Until that entry is trimmed, the volume's replicas must stay: a
/// replay after a crash finds only volumes it can list, and would treat the op of a
/// forgotten volume as already in the store. The scan is bounded by the log's byte cap.
pub(crate) fn names_volume(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let log = &state.maintainer_log;
	log.entries
		.iter()
		.flat_map(|entry| entry.ops.iter())
		.chain(log.open.iter())
		.any(|slot| {
			slot.target
				.as_ref()
				.is_some_and(|(target, _)| target == key)
		})
}

/// A pass dispatched as `dispatch` failed. The maintainer fences, so nothing changes.
pub(crate) fn pass_failed(state: &mut MaintainerState, dispatch: u64) {
	state.maintainer_log.writing.remove(&dispatch);
}

/// Whether a pass should run for the log even when no volume asked for one.
pub(crate) fn wants_pass(state: &MaintainerState) -> bool {
	state.maintainer_log.pass_wanted
}

/// Asks for the passes the log owes. An applied entry rides an ordinary pass once the
/// oldest such entry reaches the tiering age, or at once when the log's bytes reach the
/// cold-write threshold; `MaintainerTimer::MaintainerLogDue` is armed for that age. An entry
/// a completed pass covered waits only for the pass that writes the cold position it may
/// trim to, which is asked for within the throttle window. Nothing is armed while a pass is
/// already wanted, so a due timer never fires again before its pass dispatches.
fn arm_due(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving || state.maintainer_log.pass_wanted {
		if state.maintainer_log.due_at.take().is_some() {
			fx.cancel_timer(MaintainerTimer::MaintainerLogDue);
		}
		return;
	}
	// A pass that writes a new cold position is owed once covered entries can be trimmed.
	let writing = state
		.maintainer_log
		.writing
		.values()
		.last()
		.map_or(state.maintainer_log.cold.volume_seq, |cold| cold.volume_seq);
	let covered = trim_point(state).volume_seq > writing;
	let log = &state.maintainer_log;
	let applied = log
		.entries
		.iter()
		.filter(|entry| entry.entry.volume_seq <= log.committed)
		.filter_map(|entry| entry.applied_at.map(|applied_at| (entry, applied_at)));
	let mut oldest_uncovered = log
		.uncovered
		.as_ref()
		.map(|uncovered| uncovered.since_boot_ms);
	for (entry, applied_at) in applied {
		if log.passes_completed > applied_at {
			continue;
		} else {
			oldest_uncovered = Some(
				oldest_uncovered
					.unwrap_or(entry.since_boot_ms)
					.min(entry.since_boot_ms),
			);
		}
	}
	// A full log asks for a prompt pass only when the pass can let it trim: it covers an
	// applied entry, or writes a cold position covered entries reached.
	let full = log.bytes() >= state.config.write_to_cold_above_bytes
		&& (covered || oldest_uncovered.is_some());
	if covered || full {
		if state.maintainer_log.due_at.take().is_some() {
			fx.cancel_timer(MaintainerTimer::MaintainerLogDue);
		}
		state.maintainer_log.pass_wanted = true;
		super::filesystem::request_log_pass(state, full, fx);
		return;
	}
	let wanted =
		oldest_uncovered.map(|since| since.saturating_add(state.config.write_to_cold_after_ms));
	if wanted != state.maintainer_log.due_at {
		match wanted {
			Some(at) => fx.arm_timer_at(MaintainerTimer::MaintainerLogDue, at),
			None => fx.cancel_timer(MaintainerTimer::MaintainerLogDue),
		}
		state.maintainer_log.due_at = wanted;
	}
}

/// Asks for the log's next throttled pass now instead of at the tiering age, so store writes
/// staged before it, such as deletions' rows, are covered within the throttle window. Learner
/// catch-up asks when deletions keep it from reporting.
pub(crate) fn request_pass(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if state.maintainer_log.due_at.take().is_some() {
		fx.cancel_timer(MaintainerTimer::MaintainerLogDue);
	}
	state.maintainer_log.pass_wanted = true;
	super::filesystem::request_log_pass(state, false, fx);
}

/// The log's oldest uncovered entry reached the tiering age: it rides the next throttled
/// pass, like a volume's due data.
pub(crate) fn due(state: &mut MaintainerState, fx: &mut Effects) {
	state.maintainer_log.due_at = None;
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	state.maintainer_log.pass_wanted = true;
	super::filesystem::request_log_pass(state, false, fx);
}

/// Handles one reply to a request of this module.
pub(crate) fn replica(
	state: &mut MaintainerState,
	worker: WorkerId,
	request: LogRequest,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	if let LogRequest::Trim { .. } = request {
		trimmed(state, worker, result, fx);
		return;
	}
	if let Some(member) = state.maintainer_log.members.get_mut(&worker) {
		member.in_flight = None;
	}
	let observed = match &result {
		Ok(Response::ReplicaStateResponse(reply)) => Some(reply.state.clone()),
		Ok(Response::ReadVolumeLogEntriesResponse(page)) => Some(page.state.clone()),
		Ok(Response::RejectResponse(reject)) => rejected_state(reject),
		Ok(_) | Err(_) => None,
	};
	let parent = matches!(request, LogRequest::ReadParent);
	if !parent && let Some(replica) = &observed {
		if let Some(member) = state.maintainer_log.members.get_mut(&worker) {
			member.state = Some(replica.clone());
			member.absent = false;
		}
	}
	match (&request, &result) {
		(LogRequest::ReadParent, _) => {
			if parent_read(state, worker.clone(), result) {
				retry_later(state, worker, fx);
			}
		}
		(_, Ok(Response::RejectResponse(RejectResponse::VolumeNotLoaded))) => {
			if let Some(member) = state.maintainer_log.members.get_mut(&worker) {
				member.state = None;
				member.absent = true;
			}
			// Load it again after the retry delay.
			retry_later(state, worker, fx);
		}
		(
			LogRequest::Claim
			| LogRequest::Append { .. }
			| LogRequest::Truncate
			| LogRequest::MarkInSync,
			Ok(Response::RejectResponse(
				RejectResponse::StaleClientEpoch(_) | RejectResponse::ClientConflict(_),
			)),
		) => {
			if state.maintainer_log.phase == LogPhase::Claiming {
				// Another writer of this grant claimed higher during recovery: claim above it.
				let floor = observed.as_ref().map_or(0, epoch_floor);
				if let Some(client) = &mut state.maintainer_log.client
					&& floor >= client.client_epoch
				{
					let Some(epoch) = floor.checked_add(1) else {
						fence(state, fx);
						return;
					};
					client.client_epoch = epoch;
				}
				retry_later(state, worker, fx);
			} else {
				// A newer writer took the log over; this lifetime no longer owns it.
				fence(state, fx);
			}
		}
		(LogRequest::ReadTail, Ok(Response::ReadVolumeLogEntriesResponse(page))) => {
			tail_page(state, page.entries.clone(), fx);
		}
		(_, Ok(Response::ReplicaStateResponse(_))) => {}
		(_, Ok(Response::RejectResponse(_)) | Err(_)) | (_, Ok(_)) => {
			retry_later(state, worker, fx);
		}
	}
}

/// A trim answered. While the member's other request is in flight, only the cold position
/// the trim reports is taken, since that request may have moved the rest of its state.
fn trimmed(
	state: &mut MaintainerState,
	worker: WorkerId,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	let Some(member) = state.maintainer_log.members.get_mut(&worker) else {
		return;
	};
	member.trimming = None;
	let sent_after = std::mem::take(&mut member.sent_after_trim);
	match result {
		// With nothing else sent since the trim, the reply is the member's latest state.
		Ok(Response::ReplicaStateResponse(reply)) if member.in_flight.is_none() && !sent_after => {
			member.state = Some(reply.state);
		}
		Ok(Response::ReplicaStateResponse(reply)) => {
			if let Some(replica) = &mut member.state
				&& reply.state.cold.volume_seq > replica.cold.volume_seq
			{
				replica.cold = reply.state.cold;
			}
		}
		Ok(_) | Err(_) => {
			member.trim_waiting = true;
			fx.arm_timer_after(
				MaintainerTimer::MaintainerLogRetry(worker),
				state.config.tuning().retry_ms,
			);
		}
	}
}

/// The replica state a rejection reports, when it carries one.
fn rejected_state(reject: &RejectResponse) -> Option<VolumeLogReplicaState> {
	match reject {
		RejectResponse::StaleClientEpoch(r) => Some(r.state.clone()),
		RejectResponse::ClientConflict(r) => Some(r.state.clone()),
		RejectResponse::NotClaimed(r) => Some(r.state.clone()),
		RejectResponse::VolumeSeqGap(r) => Some(r.state.clone()),
		RejectResponse::Diverged(r) => Some(r.state.clone()),
		RejectResponse::AppendedMismatch(r) => Some(r.state.clone()),
		RejectResponse::BelowCold(r) => Some(r.state.clone()),
		RejectResponse::VolumeLogReplicaIncarnationMismatch(r) => Some(r.state.clone()),
		RejectResponse::VolumeLogReplicaDeleted(r) => Some(r.state.clone()),
		RejectResponse::StaleTabletReplicaSet(_)
		| RejectResponse::StaleTabletGrantEpoch(_)
		| RejectResponse::VolumeNotLoaded
		| RejectResponse::Overloaded(_)
		| RejectResponse::NotServing
		| RejectResponse::RequestTooLarge(_)
		| RejectResponse::TooManyLoadedVolumes(_)
		| RejectResponse::DiskIncarnationMismatch(_)
		| RejectResponse::DiskFull(_)
		| RejectResponse::UnsupportedPayloadVersion(_) => None,
	}
}

/// Adds a page of the fixed history. The page must continue the log exactly.
fn tail_page(state: &mut MaintainerState, entries: Vec<VolumeLogEntry>, fx: &mut Effects) {
	let LogPhase::Reading { through, .. } = state.maintainer_log.phase.clone() else {
		return;
	};
	let mut previous = state.maintainer_log.appended();
	for entry in entries {
		if entry.volume_seq > through.volume_seq {
			break;
		}
		if previous.volume_seq.checked_add(1) != Some(entry.volume_seq)
			|| entry.client_epoch < previous.client_epoch
		{
			fence(state, fx);
			return;
		}
		previous = VolumeLogPosition {
			volume_seq: entry.volume_seq,
			client_epoch: entry.client_epoch,
		};
		state.maintainer_log.entries.push_back(Entry {
			entry,
			ops: Vec::new(),
			applied_at: None,
			since_boot_ms: fx.now_boot_ms(),
		});
	}
}

/// Records a member's answer to the parent read, or a page of the parent's log.
/// Records one reply of the split parent's log read. Returns whether the reply failed, so
/// the member waits for its retry delay before it is asked again.
fn parent_read(
	state: &mut MaintainerState,
	worker: WorkerId,
	result: Result<Response, String>,
) -> bool {
	let LogPhase::ReadingParent(read) = &mut state.maintainer_log.phase else {
		return false;
	};
	match (&read.reading, result) {
		(None, Ok(Response::RejectResponse(RejectResponse::VolumeNotLoaded))) => {
			read.answered.insert(worker, None);
		}
		(None, Ok(Response::RejectResponse(RejectResponse::BelowCold(reply)))) => {
			read.answered.insert(worker, Some(reply.state));
		}
		(None, Ok(Response::ReadVolumeLogEntriesResponse(page))) => {
			read.answered.insert(worker, Some(page.state));
		}
		(Some((source, next, through)), Ok(Response::ReadVolumeLogEntriesResponse(page)))
			if *source == worker =>
		{
			let (source, mut next, through) = (source.clone(), *next, *through);
			for entry in page.entries {
				if entry.volume_seq != next || entry.volume_seq > through {
					break;
				}
				next += 1;
				read.entries.push(entry);
			}
			read.reading = Some((source, next, through));
		}
		(Some((source, _, _)), _) if *source == worker => {
			// The member holding the longest log failed a page. Ask a quorum again, so the
			// read moves to another member holding the longest log if one does; the pages
			// read so far are dropped, since that member's log is chosen afresh.
			read.answered.remove(&worker);
			read.reading = None;
			read.entries.clear();
			return true;
		}
		(_, _) => {
			// A failed probe is asked again after the retry delay.
			return true;
		}
	}
	false
}

fn retry_later(state: &mut MaintainerState, worker: WorkerId, fx: &mut Effects) {
	if let Some(member) = state.maintainer_log.members.get_mut(&worker) {
		member.waiting = true;
	}
	fx.arm_timer_after(
		MaintainerTimer::MaintainerLogRetry(worker),
		state.config.tuning().retry_ms,
	);
}

/// The retry timer of a member that failed or refused.
pub(crate) fn retry(state: &mut MaintainerState, worker: WorkerId) {
	if let Some(member) = state.maintainer_log.members.get_mut(&worker) {
		member.waiting = false;
		member.trim_waiting = false;
	}
}

/// The replica set changed: new destinations get the log, removed ones stop counting.
pub(crate) fn assignment_updated(state: &mut MaintainerState) {
	if state.maintainer_log.phase == LogPhase::Idle {
		return;
	}
	let destinations = membership::destinations(&state.tablet.replica_set)
		.into_iter()
		.collect::<BTreeSet<_>>();
	state.maintainer_log.members.retain(|worker, member| {
		destinations.contains(worker) || member.in_flight.is_some() || member.trimming.is_some()
	});
	for worker in destinations {
		state.maintainer_log.members.entry(worker).or_default();
	}
}

/// Whether the replica set being moved to holds the committed log in sync, which a catch-up
/// report requires before the set may change. As for volumes, a majority of `next` must hold
/// it, and every learner must be one of them, even one that cannot be reached; the
/// coordinator's Learning deadline aborts a change whose learner never catches up. A leaving
/// member is never required, so replacing an unreachable member does not wait for it.
pub(crate) fn covers_destinations(state: &MaintainerState) -> bool {
	let log = &state.maintainer_log;
	let Some(client) = &log.client else {
		return false;
	};
	let holds_log = |worker: &WorkerId| {
		log.members
			.get(worker)
			.and_then(|member| member.state.as_ref())
			.is_some_and(|replica| {
				replica.status == VolumeLogReplicaStatus::InSync
					&& replica.claimed_client.as_ref() == Some(client)
					&& replica.appended.volume_seq >= log.committed
					&& log.position(replica.appended.volume_seq).as_ref() == Some(&replica.appended)
			})
	};
	let set = &state.tablet.replica_set;
	let incoming = membership::incoming(set);
	let majority = incoming.iter().filter(|worker| holds_log(worker)).count() > incoming.len() / 2;
	let learners_hold_log = match &set.phase {
		TabletPhase::Learning(_) => incoming
			.iter()
			.filter(|worker| !set.members.contains(worker.as_bytes()))
			.all(|worker| holds_log(worker)),
		TabletPhase::Stable | TabletPhase::Joint(_) => true,
	};
	majority && learners_hold_log
}

/// Debug checks of the log's own invariants.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	let log = &state.maintainer_log;
	let mut previous = log.cold.clone();
	for entry in &log.entries {
		anyhow::ensure!(
			previous.volume_seq + 1 == entry.entry.volume_seq
				&& entry.entry.client_epoch >= previous.client_epoch,
			"maintainer log entries are not consecutive above cold"
		);
		previous = entry.position();
	}
	anyhow::ensure!(
		log.committed <= previous.volume_seq.max(log.cold.volume_seq),
		"maintainer log committed past its tail"
	);
	Ok(())
}

/// The store key of the record row of the maintainer log volume `volume_key`.
pub fn record_key(
	volume_key: &depot_types::VolumeKey,
) -> depot_fs_core::keys::slatedb::MaintainerLogRecordKey {
	depot_fs_core::keys::slatedb::MaintainerLogRecordKey::new(
		depot_types::partition_key_hash(volume_key),
		volume_key.name.clone().map(Into::into),
	)
}


