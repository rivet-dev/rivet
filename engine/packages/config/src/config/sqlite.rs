use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Name of the UniversalDB throttle the depot compaction, cold, and reclaim paths charge. The read
/// and write axes of this one throttle are what `compaction_read_bytes_per_second` and
/// `compaction_write_bytes_per_second` configure.
pub const DEPOT_COMPACTION_THROTTLE: &str = "depot_compaction";

/// Name of the UniversalDB throttle the actor-facing depot paths charge: `get_pages`, `commit`, and
/// each staged commit segment.
///
/// Deliberately a different name from `DEPOT_COMPACTION_THROTTLE` rather than a shared budget.
/// Compaction staging already starved install once by sharing one, and putting actor commits on that
/// same budget would reproduce it with the actor as the starver.
pub const DEPOT_ACTOR_THROTTLE: &str = "depot_actor";

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sqlite {
	/// Maximum bytes of FoundationDB storage a single SQLite database may occupy, counting its
	/// live shards, its unfolded delta history, and its page index. A commit whose projected usage
	/// exceeds this is refused with `depot.quota_exceeded` rather than partially applied.
	///
	/// Defaults to 10 GiB. Burst mode raises the effective cap for a branch whose cold drain has
	/// fallen behind, so peak usage can exceed this by that multiplier before a commit is refused.
	/// Raising it lets existing databases keep growing immediately; lowering it does not delete
	/// anything, it only refuses further growth on databases already above the new value.
	#[serde(default)]
	pub max_storage_bytes: Option<u64>,
	#[serde(default)]
	pub unstable_disable_commit_size_cap: Option<bool>,
	/// UNSTABLE: disables SQLite hot compaction.
	#[serde(default)]
	pub unstable_disable_compaction: Option<bool>,
	/// Cluster-wide budget, in bytes per second, for FoundationDB writes issued by depot
	/// compaction (hot compaction and reclaim). Bounds how fast compaction drains a backlog so it
	/// cannot saturate FDB's write queue. Defaults to 16 MiB/s, a conservative fraction of the
	/// 500 MiB/s sustained disk throughput recommended for FDB in the self-hosting docs, leaving
	/// headroom for live traffic and FDB write amplification.
	#[serde(default)]
	pub compaction_write_bytes_per_second: Option<u64>,
	/// Cluster-wide budget, in bytes per second, for FoundationDB reads issued by the heavy depot
	/// compaction read transactions (hot stage and hot install). Bounds how fast compaction reads a
	/// backlog's DELTA history so a deep, overwrite-heavy chain cannot pin a handful of storage
	/// processes on reads. Defaults to 64 MiB/s: reads carry no FDB replication amplification so
	/// this can sit above the write budget, but the read-amplification pathology (a throttled stage
	/// re-reading the same slice) makes an explicit cap worthwhile. Shares the same conflict-free
	/// windowed-counter design as the write throttle.
	#[serde(default)]
	pub compaction_read_bytes_per_second: Option<u64>,
	/// Cluster-wide budget, in bytes per second, for FoundationDB writes issued by actor commits,
	/// including each segment of a staged commit.
	///
	/// Unset means unthrottled, which is the behaviour before this existed. A staged commit lets one
	/// actor push an unbounded byte volume through `commit`, so this is the bound for that; it is
	/// opt-in because a byte-rate cap on the actor write path stalls user-visible commits, and that
	/// is not a default worth taking until an operator asks for it.
	#[serde(default)]
	pub actor_write_bytes_per_second: Option<u64>,
	/// Cluster-wide budget, in bytes per second, for FoundationDB reads issued by actor `get_pages`.
	///
	/// Unset means unthrottled. Reads matter here because a large staged commit makes the pages it
	/// staged take the slow read path (their PIDX owner sits above head, so a reader walks history
	/// for them), which means read volume spikes exactly when write volume peaks.
	#[serde(default)]
	pub actor_read_bytes_per_second: Option<u64>,
	/// Whether hot compaction folds shard images straight into the live `SHARD` tier instead of
	/// staging them and having the install copy them across byte for byte. Halves hot compaction's
	/// FDB write volume, because the image stops being written twice.
	///
	/// Read per slice, so flipping it takes effect on the next slice without restarting anything.
	/// Nothing about the mode is persisted: the install discovers where an image landed by looking
	/// for it, so a drain that spans a flip installs correctly either way and a mixed job needs no
	/// special handling.
	///
	/// No persisted format changes, so there is no version floor to clear before turning this on and
	/// nothing on disk to downgrade after turning it off. Rolling back is flipping it off; new slices
	/// stage again from that point.
	///
	/// A compaction job abandoned mid-drain leaves shard images behind. Normally its successor
	/// reproduces the same boundaries and overwrites them, so they do not accumulate. A forced drain
	/// takes the live head by design, so an abandoned forced drain can strand images that no later
	/// drain revisits; those are inert for reads but nothing reclaims them. Defaults to off.
	#[serde(default)]
	pub compaction_hot_fold_direct_to_shard: Option<bool>,
	/// Percentage (0-100) of database branches admitted to run compaction, keyed by a stable hash of
	/// the branch id. A branch whose hash falls outside the admitted fraction skips starting any
	/// compaction job (hot, cold, reclaim) while still tracking its backlog. This is a gradual
	/// rollout / load-shedding knob: it bounds how many databases compact at once so enabling
	/// compaction over a large uncompacted herd cannot saturate FDB all at once. Re-evaluated on
	/// every manager refresh, so raising it lets a previously skipped branch compact on its next
	/// check without restarting anything. Defaults to 100 (every branch admitted).
	#[serde(default)]
	pub compaction_admission_percent: Option<f64>,
	/// Multiplier hot staging applies to the compaction byte budget before the throttle's admission
	/// ramp is evaluated. Every compaction lane charges one shared counter, so a multiplier below 1 is
	/// how much of the total compaction byte rate staging is allowed to occupy before it stops
	/// admitting. The lanes that advance the pipeline (install, cold publish, cold staging) and the
	/// lane that deletes (reclaim) measure the same counter against larger budgets, so they keep
	/// admitting through the region where staging is already denied.
	///
	/// This bounds a duplicate. Staging writes a second copy of every page it folds and cannot release
	/// either copy until install consumes it, so a cluster where staging wins the shared budget grows
	/// its FoundationDB footprint without bound while install and reclaim starve. Defaults to 0.3.
	/// Lower it to drain a backlog harder, raise it toward 1.0 to let staging and install compete as
	/// peers.
	///
	/// Applies only while `compaction_hot_fold_direct_to_shard` is off. A direct fold writes the
	/// authoritative image once, so there is no duplicate to bound and the slice measures against
	/// `compaction_install_throttle_budget_multiplier` instead.
	#[serde(default)]
	pub compaction_stage_throttle_budget_multiplier: Option<f64>,
	/// Utilization of the staging lane's budget below which every staging charge is admitted. Between
	/// this mark and that budget the admit probability ramps linearly to zero. Defaults to 0.5. Applies
	/// on the same terms as `compaction_stage_throttle_budget_multiplier`.
	#[serde(default)]
	pub compaction_stage_throttle_admit_soft_util: Option<f64>,
	/// How long, in milliseconds, a throttled staging slice backs off before its drain retries.
	///
	/// Deliberately far longer than the other lanes. Admission is probabilistic, so a denied slice that
	/// retries within the same window keeps rolling dice against the estimate install and reclaim are
	/// trying to work under. Backing staging off across several windows removes that retry pressure
	/// rather than merely losing each roll. Defaults to 30 seconds.
	///
	/// Applies only while `compaction_hot_fold_direct_to_shard` is off, for the same reason the staging
	/// multiplier does: a direct fold is not competing with install over a duplicate it created, so
	/// parking it that long would only stall the fold.
	#[serde(default)]
	pub compaction_stage_throttle_backoff_ms: Option<i64>,
	/// Multiplier the lanes that advance the pipeline without duplicating apply to the compaction byte
	/// budget before the throttle's admission ramp is evaluated: hot install, cold publish, cold
	/// staging, and hot folds that write straight to the shard tier. Install folds staged shards into
	/// the manifest and releases the staging area, and cold publish is what lets the cold watermark
	/// advance so reclaim may delete. Defaults to 1.0, so the configured byte rate stays an honest cap
	/// on everything except reclaim.
	///
	/// Cold staging measures against this rather than the staging budget because its heavy output is
	/// object-storage bytes, so it leaves no FoundationDB duplicate to bound.
	#[serde(default)]
	pub compaction_install_throttle_budget_multiplier: Option<f64>,
	/// Utilization of the advancing lanes' budget below which every such charge is admitted. Defaults
	/// to 0.5.
	#[serde(default)]
	pub compaction_install_throttle_admit_soft_util: Option<f64>,
	/// Multiplier the reclaim lane applies to the compaction byte budget before the throttle's
	/// admission ramp is evaluated. Above 1.0 on purpose: reclaim deletes rather than produces, so it
	/// must keep admitting through the region where every other lane has already backed off, and its
	/// own charges land on the same counter so the arrangement stays self-regulating. This is also the
	/// bound on how far peak compaction pressure can exceed the configured byte rate. Defaults to 1.5.
	#[serde(default)]
	pub compaction_reclaim_throttle_budget_multiplier: Option<f64>,
	/// Utilization of the reclaim lane's budget below which every reclaim charge is admitted. Higher
	/// than the other lanes so reclaim stays fully admitted across the whole band where they ramp down,
	/// instead of merely ramping down more slowly. Defaults to 0.9.
	#[serde(default)]
	pub compaction_reclaim_throttle_admit_soft_util: Option<f64>,
	/// How long, in milliseconds, a depot database branch manager that received no work waits
	/// before checking again.
	///
	/// Reads never signal the manager, so without a poll a branch that stops being written parks on
	/// a deadline-less listen and never reclaims again: its cold-backed hot rows stay resident in
	/// FoundationDB forever, and the one-shot stale-PIDX repair never gets dispatched. One reclaim
	/// job drains a branch's whole backlog, so this only bounds how long a newly idle branch keeps
	/// duplicate hot copies. Defaults to 12 hours. Each wake is a bounded-budget FoundationDB
	/// snapshot, so lower it if reclaim latency on idle branches matters more than the cluster-wide
	/// refresh cost (at 1M branches, 12 hours is ~23 refreshes/sec).
	#[serde(default)]
	pub manager_idle_poll_interval_ms: Option<i64>,
	/// How long, in milliseconds, a depot database branch manager waits before its next
	/// reclaim/GC check after arming one.
	///
	/// This is the latency floor on freeing hot rows that compaction has already superseded: the
	/// footprint a branch carries stays at its peak for up to this long after the work that made it
	/// reclaimable finished. Every wake costs a bounded-budget FoundationDB snapshot per branch, so
	/// lower it only where reclaim latency matters more than that per-branch refresh cost.
	/// Defaults to 10 minutes.
	#[serde(default)]
	pub manager_reclaim_interval_ms: Option<i64>,
	/// The largest span of txids one hot compaction drain folds before its install advances
	/// `hot_watermark_txid`.
	///
	/// The watermark is what makes a delta reclaimable, and a drain advances it exactly once, at
	/// the end. So this is also the granularity of reclaim eligibility: a branch whose whole
	/// history fits in one span holds every delta resident until the entire fold lands, which sets
	/// the peak footprint at live shards plus the full delta history. Lowering it folds in smaller
	/// chunks so reclaim interleaves and the peak comes down, at the cost of re-folding a page once
	/// per chunk that touches it and of running proportionally more jobs. Each drain rounds its own
	/// span down to a multiple of the drain head grain; this cap is not itself rounded.
	///
	/// Defaults to 512, which keeps a typical branch draining over several jobs rather than one, so
	/// reclaim gets more than a single eligibility window per branch.
	#[serde(default)]
	pub compaction_max_hot_drain_span_txids: Option<u64>,
	/// Point-in-time recovery. Absent disables PITR entirely: hot compaction selects no interval
	/// coverage txids, writes no `PITR_INTERVAL` rows, and ignores any stored bucket or database
	/// policy override. Present enables it cluster-wide with the settings below.
	///
	/// PITR is off by default because each retained coverage position pins a complete shard image
	/// per touched shard, so the retained shard versions scale with `retention_ms / interval_ms`.
	/// Turning it on is a deliberate trade of FoundationDB footprint for restorable positions.
	#[serde(default)]
	pub pitr: Option<SqlitePitr>,
}

impl Sqlite {
	pub fn max_storage_bytes(&self) -> i64 {
		let bytes = self.max_storage_bytes.unwrap_or(10 * 1024 * 1024 * 1024);
		i64::try_from(bytes).unwrap_or(i64::MAX)
	}

	pub fn unstable_disable_commit_size_cap(&self) -> bool {
		self.unstable_disable_commit_size_cap.unwrap_or_default()
	}

	pub fn unstable_disable_compaction(&self) -> bool {
		// TODO: Re-enable compaction by default after thorough testing is complete.
		self.unstable_disable_compaction.unwrap_or(true)
	}

	pub fn compaction_write_bytes_per_second(&self) -> u64 {
		self.compaction_write_bytes_per_second
			.unwrap_or(16 * 1024 * 1024)
	}

	pub fn compaction_read_bytes_per_second(&self) -> u64 {
		self.compaction_read_bytes_per_second
			.unwrap_or(64 * 1024 * 1024)
	}

	pub fn compaction_hot_fold_direct_to_shard(&self) -> bool {
		self.compaction_hot_fold_direct_to_shard.unwrap_or(false)
	}

	/// Multiplier the staging lanes measure the shared compaction budget against. Below 1 so staging
	/// stops admitting well before the lanes that consume its output do.
	pub fn compaction_stage_throttle_budget_multiplier(&self) -> f64 {
		self.compaction_stage_throttle_budget_multiplier
			.unwrap_or(0.3)
	}

	pub fn compaction_stage_throttle_admit_soft_util(&self) -> f64 {
		self.compaction_stage_throttle_admit_soft_util
			.unwrap_or(0.5)
	}

	pub fn compaction_stage_throttle_backoff_ms(&self) -> i64 {
		self.compaction_stage_throttle_backoff_ms
			.unwrap_or(30 * 1000)
	}

	/// Multiplier the install and cold publish lanes measure the shared compaction budget against.
	pub fn compaction_install_throttle_budget_multiplier(&self) -> f64 {
		self.compaction_install_throttle_budget_multiplier
			.unwrap_or(1.0)
	}

	pub fn compaction_install_throttle_admit_soft_util(&self) -> f64 {
		self.compaction_install_throttle_admit_soft_util
			.unwrap_or(0.5)
	}

	/// Multiplier the reclaim lane measures the shared compaction budget against. Above 1 so deletion
	/// keeps admitting where every producing lane has already backed off.
	pub fn compaction_reclaim_throttle_budget_multiplier(&self) -> f64 {
		self.compaction_reclaim_throttle_budget_multiplier
			.unwrap_or(1.5)
	}

	pub fn compaction_reclaim_throttle_admit_soft_util(&self) -> f64 {
		self.compaction_reclaim_throttle_admit_soft_util
			.unwrap_or(0.9)
	}

	pub fn manager_idle_poll_interval_ms(&self) -> i64 {
		self.manager_idle_poll_interval_ms
			.unwrap_or(12 * 60 * 60 * 1000)
	}

	pub fn manager_reclaim_interval_ms(&self) -> i64 {
		self.manager_reclaim_interval_ms.unwrap_or(10 * 60 * 1000)
	}

	pub fn compaction_max_hot_drain_span_txids(&self) -> u64 {
		self.compaction_max_hot_drain_span_txids.unwrap_or(512)
	}

	/// PITR settings, or `None` when PITR is disabled.
	pub fn pitr(&self) -> Option<&SqlitePitr> {
		self.pitr.as_ref()
	}

	pub fn validate(&self) -> Result<()> {
		if self.manager_idle_poll_interval_ms() <= 0 {
			bail!("sqlite.manager_idle_poll_interval_ms must be greater than 0");
		}

		if self.manager_reclaim_interval_ms() <= 0 {
			bail!("sqlite.manager_reclaim_interval_ms must be greater than 0");
		}

		// The floor mirrors depot's `COMPACTION_DELTA_THRESHOLD`, the lag at which a branch first
		// becomes eligible to compact. A span under it would cap a drain below the amount of lag
		// that triggered the job, so the planner would emit a job that cannot clear the trigger and
		// would re-plan it every cycle. depot cannot be imported here (it depends on this crate),
		// so the value is restated rather than shared.
		const MIN_HOT_DRAIN_SPAN_TXIDS: u64 = 128;

		let span = self.compaction_max_hot_drain_span_txids();
		if span < MIN_HOT_DRAIN_SPAN_TXIDS {
			bail!(
				"sqlite.compaction_max_hot_drain_span_txids must be at least {MIN_HOT_DRAIN_SPAN_TXIDS}, got {span}"
			);
		}

		validate_percent(
			"sqlite.compaction_admission_percent",
			self.compaction_admission_percent,
		)?;

		// A zero budget stalls compaction outright rather than meaning "unlimited". Throttle hard
		// with a small value instead.
		if self.max_storage_bytes() <= 0 {
			bail!("sqlite.max_storage_bytes must be greater than 0");
		}

		if self.compaction_write_bytes_per_second() == 0 {
			bail!("sqlite.compaction_write_bytes_per_second must be greater than 0");
		}

		if self.compaction_read_bytes_per_second() == 0 {
			bail!("sqlite.compaction_read_bytes_per_second must be greater than 0");
		}

		validate_budget_multiplier(
			"sqlite.compaction_stage_throttle_budget_multiplier",
			self.compaction_stage_throttle_budget_multiplier(),
		)?;
		validate_budget_multiplier(
			"sqlite.compaction_install_throttle_budget_multiplier",
			self.compaction_install_throttle_budget_multiplier(),
		)?;
		validate_budget_multiplier(
			"sqlite.compaction_reclaim_throttle_budget_multiplier",
			self.compaction_reclaim_throttle_budget_multiplier(),
		)?;
		validate_admit_soft_util(
			"sqlite.compaction_stage_throttle_admit_soft_util",
			self.compaction_stage_throttle_admit_soft_util(),
		)?;
		validate_admit_soft_util(
			"sqlite.compaction_install_throttle_admit_soft_util",
			self.compaction_install_throttle_admit_soft_util(),
		)?;
		validate_admit_soft_util(
			"sqlite.compaction_reclaim_throttle_admit_soft_util",
			self.compaction_reclaim_throttle_admit_soft_util(),
		)?;

		if self.compaction_stage_throttle_backoff_ms() <= 0 {
			bail!("sqlite.compaction_stage_throttle_backoff_ms must be greater than 0");
		}

		if let Some(pitr) = self.pitr() {
			pitr.validate()?;
		}

		Ok(())
	}

	/// Fraction (0.0-1.0) of database branches admitted to run compaction, derived from the
	/// configured percent and clamped to the valid range. Defaults to fully admitted.
	///
	/// Prefer `Config::compaction_admission_fraction`, which lets a runtime override take
	/// precedence over the loaded value.
	pub fn compaction_admission_fraction(&self) -> f64 {
		percent_to_fraction(self.compaction_admission_percent.unwrap_or(100.0))
	}
}

/// Converts a percentage into a fraction. Clamped as a backstop; the range is enforced by
/// `Sqlite::validate`.
fn percent_to_fraction(percent: f64) -> f64 {
	percent.clamp(0.0, 100.0) / 100.0
}

/// A throttle class multiplier scales the budget an admission ramp is evaluated against. Zero would
/// make the ramp collapse to "deny everything", which stalls the lane rather than throttling it.
fn validate_budget_multiplier(name: &str, multiplier: f64) -> Result<()> {
	if !multiplier.is_finite() {
		bail!("{name} must be a finite number");
	}

	if multiplier <= 0.0 {
		bail!("{name} must be greater than 0, got {multiplier}");
	}

	Ok(())
}

/// The soft mark is a utilization of that class's own budget, so it is a fraction.
fn validate_admit_soft_util(name: &str, util: f64) -> Result<()> {
	if !util.is_finite() {
		bail!("{name} must be a finite number");
	}

	if !(0.0..=1.0).contains(&util) {
		bail!("{name} must be between 0 and 1, got {util}");
	}

	Ok(())
}

fn validate_percent(name: &str, percent: Option<f64>) -> Result<()> {
	let Some(percent) = percent else {
		return Ok(());
	};

	if !percent.is_finite() {
		bail!("{name} must be a finite number");
	}

	if !(0.0..=100.0).contains(&percent) {
		bail!("{name} must be between 0 and 100, got {percent}");
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SqlitePitr {
	/// Spacing, in milliseconds, between the PITR coverage positions hot compaction retains for a
	/// database that has no bucket or database policy override. Each position holds a complete
	/// image of every shard it covers until it expires, so halving this roughly doubles the
	/// retained shard versions. Defaults to 5 minutes.
	#[serde(default)]
	pub interval_ms: Option<i64>,
	/// How long, in milliseconds, PITR coverage is retained for a database that has no bucket or
	/// database policy override. History older than this window is reclaimable. Defaults to 7 days.
	#[serde(default)]
	pub retention_ms: Option<i64>,
}

impl SqlitePitr {
	pub fn interval_ms(&self) -> i64 {
		self.interval_ms.unwrap_or(5 * 60 * 1000)
	}

	pub fn retention_ms(&self) -> i64 {
		self.retention_ms.unwrap_or(7 * 24 * 60 * 60 * 1000)
	}

	pub fn validate(&self) -> Result<()> {
		let interval_ms = self.interval_ms();
		let retention_ms = self.retention_ms();

		if interval_ms <= 0 {
			bail!("sqlite.pitr.interval_ms must be greater than 0");
		}

		if retention_ms <= 0 {
			bail!("sqlite.pitr.retention_ms must be greater than 0");
		}

		if interval_ms > retention_ms {
			bail!(
				"sqlite.pitr.interval_ms ({interval_ms}) must be less than or equal to \
				sqlite.pitr.retention_ms ({retention_ms})"
			);
		}

		Ok(())
	}
}
