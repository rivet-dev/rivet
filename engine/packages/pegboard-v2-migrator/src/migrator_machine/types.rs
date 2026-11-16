//! Values shared by the migrator state machine's state, events and effects.
use rivet_util::Id;
use serde::{Deserialize, Serialize};

pub use rivet_data::generated::pegboard_v2_migrator_v2 as wire;

/// Correlates one effect with its response within one tenure. Never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer {
	/// Retries the state load after a failure.
	Load,
	/// Ends the backoff after a failed or refused state write, which dispatch then sends again.
	Persist,
	/// Retries a listing after a failure, or admits the next page once pacing allows.
	Listing,
	/// Starts the next pass. A documented last-resort poll: V2 owns the state and offers no
	/// notification, so passes rerun on an interval of minutes.
	Pass,
	/// The entry's wait ended: the confirm delay, or a retry backoff.
	Entry(Id),
	/// The effect `OpId` did not answer in time.
	Io(OpId),
}

/// Correlates an operator's request to migrate one actor with its reply. The service that received
/// the request assigns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

/// The migration settings one decision uses, taken from dynamic config by the executor.
#[derive(Clone, Debug, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct Settings {
	/// False when `migrate_from_v2` is not configured; the migrator then lists nothing.
	pub enabled: bool,
	/// `migrate_ratio` (**a**) in parts per million, so the state stays hashable.
	pub migrate_ratio_ppm: u32,
	pub migrate_seed: u64,
	pub max_in_flight: u32,
	pub max_actors_per_sec: u32,
	pub pass_interval_ms: u64,
	pub min_sleep_ms: u64,
	pub wake_horizon_ms: u64,
	pub confirm_delay_ms: u64,
	/// Candidates whose reported size exceeds this are skipped as `TooLarge`.
	pub max_copy_bytes: u64,
	/// Copy bytes admitted per second; see `State::copy_budget_unix_ms`.
	pub max_copy_bytes_per_sec: u64,
	/// V2 datacenters to list, ascending.
	pub datacenter_labels: Vec<u16>,
	/// The namespace the actors land in and this datacenter's label, which keyed ids derive
	/// from.
	pub target_namespace: String,
	pub v3_datacenter_label: u16,
	/// The target namespace's volume group, or `None` while it has none. An admission places its
	/// entry here and derives a keyed actor's id for that placement, as a fresh keyed create in
	/// the namespace would: the group's id when grouped, the legacy namespace-hashed id at the
	/// unprefixed placement otherwise. The entry keeps both for its life. A namespace gains its
	/// group once and never loses it, so a group, once seen, is never replaced by `None`.
	pub target_group: Option<[u8; 16]>,
}

impl Settings {
	pub fn disabled() -> Self {
		Self {
			enabled: false,
			migrate_ratio_ppm: 0,
			migrate_seed: 0,
			max_in_flight: 1,
			max_actors_per_sec: 1,
			pass_interval_ms: 60_000,
			min_sleep_ms: 0,
			wake_horizon_ms: 0,
			confirm_delay_ms: 20_000,
			max_copy_bytes: u64::MAX,
			max_copy_bytes_per_sec: u64::MAX,
			datacenter_labels: Vec::new(),
			target_namespace: String::new(),
			v3_datacenter_label: 0,
			target_group: None,
		}
	}
}

/// The facts of one V2 candidate the selection reads, from V2's `Candidate`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CandidateFacts {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub pool_name: String,
	pub create_ts: i64,
	pub version: u8,
	pub generation: u32,
	pub sleep_ts: Option<i64>,
	pub allocated: bool,
	pub destroy_ts: Option<i64>,
	pub wake_deadline_ts: Option<i64>,
	pub hibernating: bool,
	pub sqlite_v1: bool,
	pub parked: bool,
	pub db_size_pages: u32,
	pub kv_bytes_estimate: u64,
}

/// Why a listed candidate was not admitted, or why an admitted entry ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reason {
	None,
	NotSelected,
	AlreadyMigrated,
	Destroyed,
	RecentlyAwake,
	WakeSoon,
	Hibernating,
	SqliteV1,
	ActorV1,
	TooLarge,
	KeyUntranslatable,
	InFlight,
	DuplicateKey,
	FenceMoved,
	Woke,
	WokeAfterCut,
	Ineligible,
	CopyFailed,
	KeyReservationFailed,
	Failover,
	ActivateRefused,
	CutoverMismatch,
	HoldVanished,
}

impl Reason {
	pub fn as_str(self) -> &'static str {
		match self {
			Reason::None => "none",
			Reason::NotSelected => "not_selected",
			Reason::AlreadyMigrated => "already_migrated",
			Reason::Destroyed => "destroyed",
			Reason::RecentlyAwake => "recently_awake",
			Reason::WakeSoon => "wake_soon",
			Reason::Hibernating => "hibernating",
			Reason::SqliteV1 => "sqlite_v1",
			Reason::ActorV1 => "actor_v1",
			Reason::TooLarge => "too_large",
			Reason::KeyUntranslatable => "key_untranslatable",
			Reason::InFlight => "in_flight",
			Reason::DuplicateKey => "duplicate_key",
			Reason::FenceMoved => "fence_moved",
			Reason::Woke => "woke",
			Reason::WokeAfterCut => "woke_after_cut",
			Reason::Ineligible => "ineligible",
			Reason::CopyFailed => "copy_failed",
			Reason::KeyReservationFailed => "key_reservation_failed",
			Reason::Failover => "failover",
			Reason::ActivateRefused => "activate_refused",
			Reason::CutoverMismatch => "cutover_mismatch",
			Reason::HoldVanished => "hold_vanished",
		}
	}
}

/// How one actor's migration ended, for `migration_actor_result_total`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
	Confirmed,
	Woke,
	Destroyed,
	Skipped,
	Failed,
	/// V2 parked the actor but V3 cannot activate it. The entry is dropped so the slot frees, and
	/// an operator resolves the actor by hand.
	NeedsRecovery,
}

impl Outcome {
	pub fn as_str(self) -> &'static str {
		match self {
			Outcome::Confirmed => "confirmed",
			Outcome::Woke => "woke",
			Outcome::Destroyed => "destroyed",
			Outcome::Skipped => "skipped",
			Outcome::Failed => "failed",
			Outcome::NeedsRecovery => "needs_recovery",
		}
	}
}

/// What a read of the held V3 actor says about its key reservation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReservationResult {
	/// The key is reserved, or the actor has none to reserve.
	Reserved,
	/// The actor state machine is still reserving the key.
	Pending,
	/// Another datacenter owns the key.
	OwnedElsewhere,
	/// The reservation ran out of attempts and will never settle.
	Exhausted,
	/// The actor is no longer held by this migration.
	NotHeld,
	Failed(String),
}

/// What the copy executor reports.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CopyResult {
	Copied {
		kv_bytes: u64,
		pages: u32,
	},
	/// The V3 state of the target id belongs to an actor outside this migration.
	NativeActor,
	/// The entry was placed at the legacy unprefixed placement, but the target namespace has the
	/// volume group `group` now. Checked before the begin, so nothing was written.
	PlacementMoved {
		group: [u8; 16],
	},
	FenceMoved,
	Destroyed,
	Ineligible,
	Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CutoverResult {
	Cut,
	Woke,
	Destroyed,
	Ineligible,
	Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfirmResult {
	Confirmed,
	Woke,
	Destroyed,
	/// V2 already parked the actor under a different `cutover_ts`.
	CutoverMismatch,
	Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum V3Result {
	Applied,
	Refused,
	Failed(String),
}

/// How an activation's read-back left the V3 actor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivateResult {
	/// The actor is activated, and at the legacy placement its legacy index entry is written.
	Applied,
	/// The actor is activated at the legacy placement, and its legacy index entry is not written
	/// yet. The actor state machine retries that write itself; the entry reads it again.
	IndexPending,
	/// Another migration holds the actor.
	Refused,
	/// The actor has no state at the entry's placement after the activation was sent.
	Vanished(String),
	Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteResult {
	Written,
	Fenced,
	/// The storage host refused the write at admission, so nothing ran. It asks the writer to
	/// wait `retry_after_ms` before sending it again.
	Refused {
		retry_after_ms: u64,
		error: String,
	},
	Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadResult {
	Loaded {
		pass: Option<wire::MigrateFromV2Pass>,
		in_flight: Vec<wire::MigrateFromV2InFlight>,
	},
	Fenced,
	Failed(String),
}

/// What a V2 read of one actor's candidate facts returned.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateResult {
	Found(CandidateFacts),
	/// V2 holds no such actor in the migrated namespace.
	NotFound,
	Failed(String),
}

/// The answer to an operator's request to migrate one actor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrateActorResult {
	/// The actor is in flight and its entry is stored, whether this request admitted it or it
	/// already was.
	Admitted {
		v3_actor_id: Id,
		migration_id: u64,
	},
	/// The actor fails a selection rule other than the share.
	Ineligible(Reason),
	NotFound,
	/// The migrator cannot take the actor now. Nothing was admitted.
	Unavailable(Unavailable),
	/// Reading the actor from V2 failed. Nothing was admitted.
	Failed(String),
}

/// Why the migrator cannot take a requested actor now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unavailable {
	/// The tenure has not loaded the migration state yet.
	Loading,
	/// `migrate_from_v2` lists no V2 API for the actor's datacenter.
	DatacenterNotConfigured,
	/// Operator requests already hold every slot of their own allowance (`MAX_REQUESTED_IN_FLIGHT`).
	AtCapacity,
	/// Another operator request is being answered.
	RequestPending,
}

impl Unavailable {
	pub fn as_str(self) -> &'static str {
		match self {
			Unavailable::Loading => "loading",
			Unavailable::DatacenterNotConfigured => "datacenter_not_configured",
			Unavailable::AtCapacity => "at_capacity",
			Unavailable::RequestPending => "request_pending",
		}
	}
}

/// One candidate of a listed page, with the cursor that resumes the listing right after it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListedCandidate {
	pub facts: CandidateFacts,
	pub cursor: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListResult {
	Page {
		candidates: Vec<ListedCandidate>,
		next: Option<Vec<u8>>,
	},
	Failed(String),
}
