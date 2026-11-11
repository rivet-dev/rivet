//! Legacy: the backfill that moves a namespace's actors from before volume groups into its
//! group. Actors created before namespaces had groups live at unprefixed keys, and keyed ones
//! have an id that hashed their namespace; new behavior keys every volume under the namespace's
//! group, and a keyed actor's id hashes its name and key alone. Taken only by the namespace state
//! machine's `MovePage` effect, for a namespace moved from its legacy catalog row whose backfill
//! is not done. Removed in 3.1 with the backfill.
//!
//! The walk goes through the namespace's legacy index buckets: actor names in order, each name's
//! buckets, each bucket's live entries. A move of one actor claims its old database volume,
//! which fences the process holding it; creates the actor at its new id in the group, with its
//! database a Depot fork of the old one, its old creation time and its scheduled alarm, held in
//! `Initializing`; checks that the claim still holds once the fork published, and starts over if
//! not; sleeps the old actor if it runs, so its runtime stops through its sleep rather than its
//! destroy; destroys the old actor through its normal destroy path; waits until that destroy
//! ended the old actor's epoch, so the copy never runs beside the old generation; then confirms
//! the move, which raises the copy's epoch above the old actor's and lets it start, and deletes
//! the old actor's index entry, which the move's destroy keeps so a page resumed after a crash
//! before the confirm still lists the actor and confirms its copy. A write that lands on the old
//! volume after the check is lost, which is the accepted loss of the cutover.
//!
//! Two pages can run at once: one still running in a process that lost the namespace's volume,
//! and the one sent after it. Each page carries a mover token the namespace raises with every
//! send. A move marks the old actor with its token first, and every later write it makes is
//! checked at the actor it changes: the old actor's destroy only while it holds that mark, a
//! copy's confirm only by the copy's own mover, and an abandon of a held copy only for its
//! incarnation by its own mover or a later one. So of two overlapping moves of one actor, at
//! most one destroys the old actor, and only the copy whose claim check passed is confirmed.
//!
//! Actor names come from the namespace's envoy records and from the names a legacy fallback read
//! recorded on the namespace's volume, since no listing of actor names or of volumes exists:
//! every envoy that ever registered in a pool of the namespace keeps a record naming the actor
//! names it last served, and a request that found an actor through the fallback recorded its
//! name. Only pools that are still configured are read, because the pool list comes from the
//! namespace's pool configs.
//!
//! So an actor is never moved when no envoy record of a configured pool names it and no request
//! read it through the fallback: for example one that only ran on a pool deleted since, or one
//! created under a name no envoy ever served. This gap is intended, since nothing lists the old
//! volumes and closing it would need an operator scan. Once the backfill is done, the fallback
//! reads stop and such an actor answers not found; its data stays at its unprefixed keys, and
//! after a later release removes this backfill and its legacy reads, it cannot be recovered.
//!
//! A moved actor is reachable only through its namespace. The gateway resolves a direct actor-id
//! request that names no namespace and uses the admin token, an ACL token or no authentication
//! in `default`. RivetKit 3.0.x clients send `getForId()` and `create()`-handle traffic this way,
//! so after the upgrade and this backfill such a request answers `actor.not_found` for an actor
//! outside `default`. In a fork of `default`, which keeps `default`'s actor ids, it reaches
//! `default`'s actor. This is accepted and intended: current clients always send their namespace
//! in the `x-rivet-namespace` header or the `rvt-namespace` query parameter, a namespace-scoped
//! JWT resolves in its own namespace, and the engine adds no lookup from actor id to namespace.
//!
//! The end of the walk lists the names again and probes every bucket. A live actor left in one,
//! under a name that appeared mid-walk or one the walk passed, or a moved actor whose entry is
//! still there because its copy was not confirmed yet, restarts the walk there. Actors a request
//! is still destroying are waited for, since their destroy writes the bucket. Only then are the
//! buckets deleted, each probed once more right before its delete. Probing and deleting are
//! bounded by the page's deadline and resume at the bucket the last page reached, unless more
//! names are listed than when it was reached, which probes over from the first name.

use crate::{
	catalog::LegacyBackfillNameList,
	commands::{ActorCreate, ActorEvent, ActorRead, LegacyActorAlarmRead},
	index::{IndexDeleteActor, IndexListActors},
	keys::partition::{
		ActorVolume, LegacyActorDatabaseVolume, LegacyActorIndexVolume, LegacyActorVolume,
		NamespaceVolume,
	},
	wire,
};
use anyhow::{Context, Result, bail};
use rivet_data::generated::pegboard_namespace_v1 as ns;
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{VolumeKey, codec::GroupId};
use rivet_metrics::GaugeGuardExt;
use rivet_util::Id;
use std::{collections::BTreeSet, ops::Bound, time::Duration};

/// How often a move reads its new actor while that actor's initialization runs, and how often
/// the end of the walk re-lists a bucket whose moved actors are still being destroyed. The waits
/// have no notification to arm, the same as a fork request's wait for its copy.
const MOVE_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// The longest wait between two claims of an old database this process holds.
const CLAIM_RETRY_MAX_INTERVAL: Duration = Duration::from_secs(5);

/// The time a move keeps for its own steps after its old actor stopped: the mark and reads
/// before the sleep, then the claim, the fork, the old actor's destroy and the copy's confirm.
const MOVE_AFTER_STOP_MS: u64 = 15_000;

/// Legacy: the longest an old actor's stop runs once a move put it to sleep. The actor state
/// machine ends an unanswered stop as lost once the longer of the stop and node lost thresholds,
/// plus its margin, has passed. Removed in 3.1 with the backfill.
pub(crate) fn actor_stop_deadline_ms(pegboard: &rivet_config::config::Pegboard) -> u64 {
	pegboard
		.actor_stop_threshold()
		.max(pegboard.envoy_lost_threshold())
		.saturating_add(crate::actor_machine::STOP_LOST_MARGIN_MS)
}

/// Legacy: the page timeout the backfill runs under: `pegboard.backfill.page_timeout_ms`, raised
/// when shorter so that the soft deadline at half a page leaves a page's first move a whole
/// [`actor_stop_deadline_ms`] plus [`MOVE_AFTER_STOP_MS`]. A healthy actor's stop then never
/// outlasts the move that started it, however long it takes. The page deadlines, the moving
/// mark's lifetime and the namespace machine's page retry all use it. A shorter configured
/// timeout is raised rather than refused at load, so a deployment that raised its lost threshold
/// still starts. The defaults need no raise. Removed in 3.1 with the backfill.
pub(crate) fn page_timeout_ms(pegboard: &rivet_config::config::Pegboard) -> u64 {
	let first_move_ms = actor_stop_deadline_ms(pegboard).saturating_add(MOVE_AFTER_STOP_MS);
	pegboard
		.backfill
		.page_timeout_ms()
		.max(first_move_ms.saturating_mul(2))
}

/// The recorded names one list request pages.
const NAME_LIST_LIMIT: u32 = 1_000;

/// What the namespace machine's `MovePage` asks one page to do.
pub(crate) struct PageRequest {
	pub group: GroupId,
	pub index_bucket_count: u64,
	pub cursor: Option<ns::NamespaceBackfillPosition>,
	pub attempt: u32,
	/// The actors the backfill gave up on, which the walk passes and the end of the walk leaves.
	pub skipped: Vec<ns::NamespaceBackfillSkipped>,
	/// Whether a failure at the page's first actor gives up on that actor instead of failing the
	/// page.
	pub give_up: bool,
	/// The page's mover token, which every write it makes to an actor names.
	pub mover: u64,
}

/// Legacy: the context of a page's failure at its first actor, without a give-up: the actor the
/// walk waits on, by the name it is listed under and its id. The executor reports it to the
/// namespace's machine. Removed in 3.1 with the backfill.
#[derive(Debug)]
pub(crate) struct LegacyPageFailedAt {
	pub name: String,
	pub actor_id: Vec<u8>,
}

impl std::fmt::Display for LegacyPageFailedAt {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"a backfill page failed at its first actor, listed under {}",
			self.name
		)
	}
}

/// Legacy: a later move of a page did not start on an old actor that must stop first, because
/// less than its stop deadline was left before the page's soft deadline. Nothing was marked or
/// written, and the next page starts with the actor. Removed in 3.1 with the backfill.
#[derive(Debug)]
struct NoTimeToStop;

impl std::fmt::Display for NoTimeToStop {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"a backfill page has too little time left to stop a legacy actor; the next page moves it"
		)
	}
}

impl std::error::Error for NoTimeToStop {}

/// Legacy: a move's old actor was still starting, running or stopping at the page's deadline.
/// The move keeps its mark, which refuses wakes, so once the actor stops it stays asleep and the
/// next page resumes the same stop rather than putting a woken actor to sleep again. Removed in
/// 3.1 with the backfill.
#[derive(Debug)]
struct StillStopping;

impl std::fmt::Display for StillStopping {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"a moved legacy actor was still awake or stopping at the end of the backfill page; the next page resumes its stop"
		)
	}
}

impl std::error::Error for StillStopping {}

/// Legacy: a moved copy accepted its abandon but was not destroyed by the page's deadline,
/// usually because its database fork had not ended, which the copy's destroy waits on. The move
/// keeps the old actor's mark, so the old actor stays asleep rather than serving wakes while the
/// copy ends, and the next page resumes the abandon. Removed in 3.1 with the backfill.
#[derive(Debug)]
struct AbandonPending;

impl std::fmt::Display for AbandonPending {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"an abandoned moved copy was not destroyed within the backfill page; the next page resumes its abandon"
		)
	}
}

impl std::error::Error for AbandonPending {}

/// What one page did: the actors it moved, the actor it gave up on, and where the walk goes next.
pub(crate) struct PageOutcome {
	pub moved: u32,
	pub skipped: Option<ns::NamespaceBackfillSkipped>,
	pub next: ns::NamespaceBackfillNext,
}

/// What a move of one legacy actor did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MoveResult {
	/// The actor now lives at its new id in the group and its old copy is being destroyed.
	Moved,
	/// There was nothing to move: the old actor is gone, destroyed or belongs to no namespace.
	Skipped,
	/// A live grouped actor already held the new id, so the old copy was destroyed instead.
	Duplicate,
	/// A request destroyed the actor during its move, through its held copy: the old actor was
	/// destroyed too.
	Destroyed,
}

/// The deadlines of one page, by the boot clock.
#[derive(Clone, Copy)]
struct PageDeadlines {
	/// No new move starts after this, so the page reports what it did in time.
	soft_boot_ms: u64,
	/// A move still running at this fails, so a stuck actor cannot hold the page past its retry.
	hard_boot_ms: u64,
	/// A give-up's wait for an abandoned copy ends at this, before the page's own timeout.
	give_up_boot_ms: u64,
}

/// Legacy: runs one backfill page of `namespace` from `page.cursor`: moves the actors it finds,
/// one after another, until it handled `pegboard.backfill.page_size` of them or half the page
/// timeout passed, and answers where the next page starts. At the end of the walk it checks that
/// nothing was left behind and deletes the namespace's legacy index buckets, answering
/// `Finished`; that end is bounded by the same deadline and resumes where the last page stopped. A failure at the page's first actor is an error, which the page's `BackfillRetry`
/// re-drives, unless `page.give_up` is set: then the actor is reported skipped and the next page
/// starts after it. A later failure ends the page early at the failing actor, so the next page
/// starts with it. Removed in 3.1 with the backfill.
///
/// `still_owner` answers whether this process still holds the namespace's volume; the page stops
/// before its next move once it does not. The actors fence a page that kept running anyway by
/// its mover token.
pub(crate) async fn legacy_move_page(
	ctx: &EngineContext,
	namespace: &str,
	page: PageRequest,
	still_owner: &(dyn Fn() -> bool + Send + Sync),
) -> Result<PageOutcome> {
	let pegboard = ctx.config().pegboard();
	let page_size = pegboard.backfill.page_size();
	let page_timeout_ms = page_timeout_ms(pegboard);
	let stop_deadline_ms = actor_stop_deadline_ms(pegboard);
	let clock = ctx.env().clock();
	let now_boot_ms = clock.observe()?.boot_ms;
	let deadlines = PageDeadlines {
		soft_boot_ms: now_boot_ms.saturating_add(page_timeout_ms / 2),
		hard_boot_ms: now_boot_ms.saturating_add(page_timeout_ms / 4 * 3),
		give_up_boot_ms: now_boot_ms.saturating_add(page_timeout_ms / 8 * 7),
	};
	let PageRequest {
		group,
		index_bucket_count,
		cursor,
		skipped,
		give_up,
		mover,
		..
	} = page;

	let finishing = Finishing {
		index_bucket_count,
		skipped: &skipped,
		deadlines,
		still_owner,
	};

	// Names are listed only where the walk needs the next one, not on every page.
	let mut position = match cursor {
		Some(ns::NamespaceBackfillPosition::NamespaceBackfillCursor(cursor)) => cursor,
		Some(ns::NamespaceBackfillPosition::NamespaceBackfillProbing(at)) => {
			let at = FinishAt::Probing {
				name: at.name.to_string(),
				bucket: at.bucket,
				names_hash: at.names_hash,
				after: at.after,
			};
			return finish(ctx, namespace, &finishing, 0, at).await;
		}
		Some(ns::NamespaceBackfillPosition::NamespaceBackfillDeleting(at)) => {
			let at = FinishAt::Deleting {
				name: at.name.to_string(),
				bucket: at.bucket,
				names_hash: at.names_hash,
				after: at.after,
			};
			return finish(ctx, namespace, &finishing, 0, at).await;
		}
		None => {
			let names = legacy_actor_names(ctx, namespace).await?;
			match names.first() {
				Some(name) => first_position(name),
				None => return finish(ctx, namespace, &finishing, 0, FinishAt::Start).await,
			}
		}
	};

	let mut moved = 0u32;
	let mut handled = 0u32;
	loop {
		if handled >= page_size || clock.observe()?.boot_ms >= deadlines.soft_boot_ms {
			return Ok(next_page(moved, position));
		}

		let listed = list_legacy_bucket(ctx, namespace, &position, page_size - handled).await?;
		for entry in listed.actors {
			if clock.observe()?.boot_ms >= deadlines.soft_boot_ms {
				return Ok(next_page(moved, position));
			}
			let passed = ns::NamespaceBackfillAfter {
				create_unix_ms: entry.create_unix_ms,
				actor_id: entry.actor_id.clone(),
			};
			if is_skipped(&skipped, &position, &passed) {
				handled = handled.saturating_add(1);
				position.after = Some(passed);
				continue;
			}
			if !still_owner() {
				bail!("this process lost the namespace's volume during a backfill page");
			}
			let actor_id = Id::from_slice(&entry.actor_id)?;
			// A page's first move always starts. A later one starts on an actor that must stop
			// first only while a whole stop deadline is left before the soft deadline, so its
			// sleep does not run out with the page; otherwise the next page starts with it.
			let stop_by_boot_ms = match handled {
				0 => u64::MAX,
				_ => deadlines.soft_boot_ms.saturating_sub(stop_deadline_ms),
			};
			let result = bounded_move(
				ctx,
				namespace,
				group,
				actor_id,
				index_bucket_count,
				mover,
				deadlines,
				stop_by_boot_ms,
			)
			.await;
			match result {
				Ok(MoveResult::Moved) => moved = moved.saturating_add(1),
				Ok(MoveResult::Skipped | MoveResult::Duplicate | MoveResult::Destroyed) => {}
				Err(error) if error.downcast_ref::<NoTimeToStop>().is_some() => {
					return Ok(next_page(moved, position));
				}
				Err(error) if handled == 0 && give_up => {
					// The give-up decides from the actors as they are now: a move whose destroy
					// committed is confirmed rather than given up on.
					tracing::debug!(
						?error,
						namespace,
						%actor_id,
						"a backfill move failed at its last allowed attempt; giving up on it"
					);
					let gave_up = give_up_actor(
						ctx,
						namespace,
						group,
						actor_id,
						index_bucket_count,
						mover,
						deadlines.give_up_boot_ms,
					)
					.await
					.context("give up on a legacy actor after its last allowed attempt")?;
					let skipped_key = match gave_up {
						GiveUp::Resolved(result) => {
							tracing::debug!(
								?result,
								namespace,
								%actor_id,
								"a backfill move failed at its last allowed attempt, but the actor's move was already decided; it is not skipped"
							);
							if result == MoveResult::Moved {
								moved = moved.saturating_add(1);
							}
							position.after = Some(passed);
							return Ok(next_page(moved, position));
						}
						GiveUp::Skipped { key } => {
							tracing::error!(
								?error,
								namespace,
								%actor_id,
								"a backfill move failed at its last allowed attempt; the actor is skipped"
							);
							key
						}
					};
					let gave_up = ns::NamespaceBackfillSkipped {
						name: position.name.clone(),
						bucket: position.bucket,
						create_unix_ms: passed.create_unix_ms,
						actor_id: passed.actor_id.clone(),
						key: skipped_key.map(Into::into),
					};
					position.after = Some(passed);
					return Ok(PageOutcome {
						moved,
						skipped: Some(gave_up),
						next: ns::NamespaceBackfillNext::NamespaceBackfillNextPage(
							ns::NamespaceBackfillNextPage {
								cursor: ns::NamespaceBackfillPosition::NamespaceBackfillCursor(
									position,
								),
							},
						),
					});
				}
				Err(error) if handled == 0 => {
					return Err(error.context(LegacyPageFailedAt {
						name: position.name.to_string(),
						actor_id: passed.actor_id.to_vec(),
					}));
				}
				Err(error) => {
					tracing::debug!(
						?error,
						namespace,
						"a backfill move failed after others in its page; the next page starts with it"
					);
					return Ok(next_page(moved, position));
				}
			}
			handled = handled.saturating_add(1);
			position.after = Some(passed);
		}
		if listed.has_more {
			continue;
		}

		// This bucket is walked: on to the name's next bucket, then to the next name.
		if position.bucket.saturating_add(1) < index_bucket_count {
			position = ns::NamespaceBackfillCursor {
				name: position.name,
				bucket: position.bucket + 1,
				after: None,
			};
			continue;
		}
		let names = legacy_actor_names(ctx, namespace).await?;
		let current = position.name.to_string();
		let next_name = names
			.range::<String, _>((Bound::Excluded(&current), Bound::Unbounded))
			.next();
		match next_name {
			Some(name) => position = first_position(name),
			None => return finish(ctx, namespace, &finishing, moved, FinishAt::Start).await,
		}
	}
}

/// Moves one actor, failing when the move is still running at the page's hard deadline: a
/// resumed page redoes or confirms whatever it left, so stopping it is safe. An actor that must
/// stop first is not moved past `stop_by_boot_ms`; see [`has_time_to_stop`].
#[allow(clippy::too_many_arguments, reason = "one move's inputs")]
async fn bounded_move(
	ctx: &EngineContext,
	namespace: &str,
	group: GroupId,
	actor_id: Id,
	index_bucket_count: u64,
	mover: u64,
	deadlines: PageDeadlines,
	stop_by_boot_ms: u64,
) -> Result<MoveResult> {
	let clock = ctx.env().clock();
	let moved = rivet_env::timeout_at(
		clock.as_ref(),
		deadlines.hard_boot_ms,
		legacy_move_actor(
			ctx,
			namespace,
			group,
			actor_id,
			index_bucket_count,
			mover,
			deadlines.soft_boot_ms,
			stop_by_boot_ms,
		),
	)
	.await;
	match moved {
		Ok(result) => result,
		Err(error) => Err(anyhow::Error::from(error).context("a backfill move ran past its page")),
	}
}

/// Whether the entry `after` of the bucket at `position` is an actor the backfill gave up on.
fn is_skipped(
	skipped: &[ns::NamespaceBackfillSkipped],
	position: &ns::NamespaceBackfillCursor,
	after: &ns::NamespaceBackfillAfter,
) -> bool {
	skipped.iter().any(|skipped| {
		skipped.name == position.name
			&& skipped.bucket == position.bucket
			&& skipped.create_unix_ms == after.create_unix_ms
			&& skipped.actor_id == after.actor_id
	})
}

/// The start of the walk through the buckets of `name`.
fn first_position(name: &str) -> ns::NamespaceBackfillCursor {
	ns::NamespaceBackfillCursor {
		name: name.to_owned().into(),
		bucket: 0,
		after: None,
	}
}

/// The next page, in the walk at `cursor`.
fn next_page(moved: u32, cursor: ns::NamespaceBackfillCursor) -> PageOutcome {
	next_at(
		moved,
		ns::NamespaceBackfillPosition::NamespaceBackfillCursor(cursor),
	)
}

fn next_at(moved: u32, position: ns::NamespaceBackfillPosition) -> PageOutcome {
	PageOutcome {
		moved,
		skipped: None,
		next: ns::NamespaceBackfillNext::NamespaceBackfillNextPage(ns::NamespaceBackfillNextPage {
			cursor: position,
		}),
	}
}

/// What the end of the walk found in one legacy bucket, apart from the actors given up on.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BucketProbe {
	/// Nothing: every actor listed there was moved, its copy confirmed and its entry deleted.
	Empty,
	/// An actor a request destroyed is still being destroyed, and its destroy still writes the
	/// bucket; it is the next entry after `after`.
	Destroying {
		after: Option<ns::NamespaceBackfillAfter>,
	},
	/// A live actor the walk left behind, or a moved one whose copy may still wait for its
	/// confirm.
	Live,
	/// The page's deadline came before the probe reached the bucket's end; every entry up to
	/// `after` was probed.
	Deadline {
		after: Option<ns::NamespaceBackfillAfter>,
	},
}

/// What the end of the walk needs from its page.
struct Finishing<'a> {
	index_bucket_count: u64,
	skipped: &'a [ns::NamespaceBackfillSkipped],
	deadlines: PageDeadlines,
	still_owner: &'a (dyn Fn() -> bool + Send + Sync),
}

/// Where the end of the walk resumes.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FinishAt {
	/// The first bucket of the first name, probing.
	Start,
	/// Probing the bucket of this name from after its entry `after`, or from its start, reached
	/// when the names listed hashed to `names_hash` (see [`names_hash`]).
	Probing {
		name: String,
		bucket: u64,
		names_hash: u64,
		after: Option<ns::NamespaceBackfillAfter>,
	},
	/// Deleting from the bucket of this name, every bucket having probed empty, its last probe
	/// resuming from after `after`, reached when the names listed hashed to `names_hash`.
	Deleting {
		name: String,
		bucket: u64,
		names_hash: u64,
		after: Option<ns::NamespaceBackfillAfter>,
	},
}

/// What a page of the end of the walk does first, given where it resumes and the hash of the
/// names it listed: probe or delete, from a `(name, bucket)` or from the start. A resumed
/// position reached with any other set of names starts probing over, since a name recorded since
/// may sort before it and its buckets would otherwise be neither probed nor kept, even when
/// another name dropped out meanwhile and the count stayed the same.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FinishResume {
	pub deleting: bool,
	pub from: Option<(String, u64)>,
	/// Where the probe of the bucket `from` resumes inside it; `None` from its start.
	pub after: Option<ns::NamespaceBackfillAfter>,
}

/// A stable 64-bit hash of the sorted name set (FNV-1a over each name's length and bytes), which
/// a resumed end of the walk persists to tell whether its listing changed. It must not change
/// between builds, so it is spelled out rather than taken from the standard library's hasher.
/// It is not collision-resistant against crafted input: actor names are the tenant's own, so a
/// tenant could choose a name set with an equal hash, and a collision only strands that tenant's
/// own newly recorded name until the next listing change restarts the probe.
pub(crate) fn names_hash(names: &BTreeSet<String>) -> u64 {
	const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
	const PRIME: u64 = 0x0100_0000_01b3;

	let mut hash = OFFSET_BASIS;
	for name in names {
		let length = (name.len() as u64).to_le_bytes();
		for byte in length.iter().chain(name.as_bytes()) {
			hash ^= u64::from(*byte);
			hash = hash.wrapping_mul(PRIME);
		}
	}
	hash
}

pub(crate) fn finish_resume(at: FinishAt, listed_hash: u64) -> FinishResume {
	match at {
		FinishAt::Start => FinishResume {
			deleting: false,
			from: None,
			after: None,
		},
		FinishAt::Probing { names_hash, .. } | FinishAt::Deleting { names_hash, .. }
			if listed_hash != names_hash =>
		{
			FinishResume {
				deleting: false,
				from: None,
				after: None,
			}
		}
		FinishAt::Probing {
			name,
			bucket,
			after,
			..
		} => FinishResume {
			deleting: false,
			from: Some((name, bucket)),
			after,
		},
		FinishAt::Deleting {
			name,
			bucket,
			after,
			..
		} => FinishResume {
			deleting: true,
			from: Some((name, bucket)),
			after,
		},
	}
}

/// What the end of the walk's wait on one bucket came to.
enum BucketSettled {
	/// The bucket holds nothing but actors given up on.
	Empty,
	/// A live actor the walk left behind is there.
	Live,
	/// The page's deadline came first, with the probe past every entry up to `after`.
	Deadline {
		after: Option<ns::NamespaceBackfillAfter>,
	},
}

/// Probes the bucket `bucket` of `name` from after its entry `after`, waiting while an actor
/// there is still being destroyed by a request, until it settles or the page's deadline passes.
async fn settle_bucket(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	bucket: u64,
	mut after: Option<ns::NamespaceBackfillAfter>,
	finishing: &Finishing<'_>,
) -> Result<BucketSettled> {
	let clock = ctx.env().clock();
	let entries = DepotBucket {
		ctx,
		namespace,
		name,
		bucket,
		soft_boot_ms: finishing.deadlines.soft_boot_ms,
	};
	loop {
		let probe =
			probe_bucket(&entries, namespace, name, bucket, finishing.skipped, after).await?;
		match probe {
			BucketProbe::Empty => return Ok(BucketSettled::Empty),
			BucketProbe::Live => return Ok(BucketSettled::Live),
			BucketProbe::Deadline { after } => return Ok(BucketSettled::Deadline { after }),
			BucketProbe::Destroying { after: before } => {
				after = before;
				rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
			}
		}
	}
}

/// The walk restarted at the bucket `bucket` of `name`, where a live actor was left behind.
fn walk_at(moved: u32, name: &str, bucket: u64) -> PageOutcome {
	next_page(
		moved,
		ns::NamespaceBackfillCursor {
			name: name.to_owned().into(),
			bucket,
			after: None,
		},
	)
}

/// Ends the walk, bounded by the page's deadline and resumable at the `(name, bucket)` it reached.
/// The names are listed once for the page, after a deadline check, since the listing reads every
/// envoy record of the namespace. Every bucket from `at` on is probed: a live actor left behind
/// restarts the walk at its bucket, and a bucket whose moved actors are still being destroyed is
/// waited for. Once every bucket holds nothing but the actors given up on, the buckets are
/// deleted in the same order, except those holding such an actor, each probed once more right
/// before its delete, and `Finished` is answered. The deadline is checked before each probe and
/// each delete; when it passes, the next page resumes at the bucket it had reached. A resumed
/// position records a hash of the names listed when it was reached, and a page whose listing
/// hashes differently probes over from the first name, so a name recorded behind the position
/// is probed rather than skipped or deleted unprobed. Deleting is idempotent, so a crash
/// re-drives the same position.
async fn finish(
	ctx: &EngineContext,
	namespace: &str,
	finishing: &Finishing<'_>,
	moved: u32,
	at: FinishAt,
) -> Result<PageOutcome> {
	let clock = ctx.env().clock();
	let soft_boot_ms = finishing.deadlines.soft_boot_ms;

	// An empty name sorts before every name, so this position resumes the end of the walk from
	// its start whatever the next listing hashes to.
	if clock.observe()?.boot_ms >= soft_boot_ms {
		let resume = match at {
			FinishAt::Start => probing_at(moved, "", 0, names_hash(&BTreeSet::new()), None),
			FinishAt::Probing {
				name,
				bucket,
				names_hash,
				after,
			} => probing_at(moved, &name, bucket, names_hash, after),
			FinishAt::Deleting {
				name,
				bucket,
				names_hash,
				after,
			} => deleting_at(moved, &name, bucket, names_hash, after),
		};
		return Ok(resume);
	}
	let names = legacy_actor_names(ctx, namespace).await?;
	let listed_hash = names_hash(&names);
	let FinishResume {
		deleting,
		from,
		after,
	} = finish_resume(at, listed_hash);
	// The resumed position inside a bucket applies to the first bucket the resumed phase
	// probes, which is the bucket it was reached in.
	let (mut probe_after, mut delete_after) = match deleting {
		true => (None, after),
		false => (after, None),
	};

	if !deleting {
		for (name, bucket) in bucket_positions(&names, finishing.index_bucket_count, from.as_ref())
		{
			let start = probe_after.take();
			match settle_bucket(ctx, namespace, name, bucket, start, finishing).await? {
				BucketSettled::Empty => {}
				BucketSettled::Live => return Ok(walk_at(moved, name, bucket)),
				BucketSettled::Deadline { after } => {
					return Ok(probing_at(moved, name, bucket, listed_hash, after));
				}
			}
		}
	}

	let delete_from = match deleting {
		true => from,
		false => None,
	};
	let depot = ctx.depot();
	for (name, bucket) in
		bucket_positions(&names, finishing.index_bucket_count, delete_from.as_ref())
	{
		let holds_skipped = finishing
			.skipped
			.iter()
			.any(|skipped| skipped.name == name && skipped.bucket == bucket);
		let start = delete_after.take();
		if holds_skipped {
			continue;
		}
		if clock.observe()?.boot_ms >= soft_boot_ms {
			return Ok(deleting_at(moved, name, bucket, listed_hash, start));
		}
		if !(finishing.still_owner)() {
			bail!("this process lost the namespace's volume during a backfill page");
		}
		// One more probe right before the delete: an actor recorded since the probing pass
		// restarts the walk instead of losing its bucket.
		match settle_bucket(ctx, namespace, name, bucket, start, finishing).await? {
			BucketSettled::Empty => {}
			BucketSettled::Live => return Ok(walk_at(moved, name, bucket)),
			BucketSettled::Deadline { after } => {
				return Ok(deleting_at(moved, name, bucket, listed_hash, after));
			}
		}
		let volume =
			LegacyActorIndexVolume::new(namespace.to_owned(), name.to_owned(), bucket).volume_key();
		depot
			.delete_current(volume)
			.await
			.context("delete a legacy actor index bucket")?;
	}
	Ok(PageOutcome {
		moved,
		skipped: None,
		next: ns::NamespaceBackfillNext::NamespaceBackfillFinished,
	})
}

/// The next page, probing the bucket `bucket` of `name` at the end of the walk from after its
/// entry `after`.
fn probing_at(
	moved: u32,
	name: &str,
	bucket: u64,
	names_hash: u64,
	after: Option<ns::NamespaceBackfillAfter>,
) -> PageOutcome {
	next_at(
		moved,
		ns::NamespaceBackfillPosition::NamespaceBackfillProbing(ns::NamespaceBackfillProbing {
			name: name.to_owned().into(),
			bucket,
			names_hash,
			after,
		}),
	)
}

/// The next page, deleting from the bucket `bucket` of `name` at the end of the walk, its probe
/// resuming from after its entry `after`.
fn deleting_at(
	moved: u32,
	name: &str,
	bucket: u64,
	names_hash: u64,
	after: Option<ns::NamespaceBackfillAfter>,
) -> PageOutcome {
	next_at(
		moved,
		ns::NamespaceBackfillPosition::NamespaceBackfillDeleting(ns::NamespaceBackfillDeleting {
			name: name.to_owned().into(),
			bucket,
			names_hash,
			after,
		}),
	)
}

/// Every `(name, bucket)` of `names` in walk order, from `from` on.
fn bucket_positions<'a>(
	names: &'a BTreeSet<String>,
	index_bucket_count: u64,
	from: Option<&'a (String, u64)>,
) -> impl Iterator<Item = (&'a str, u64)> + 'a {
	names
		.iter()
		.flat_map(move |name| (0..index_bucket_count).map(move |bucket| (name.as_str(), bucket)))
		.filter(move |(name, bucket)| match from {
			Some((from_name, from_bucket)) => {
				(*name, *bucket) >= (from_name.as_str(), *from_bucket)
			}
			None => true,
		})
}

/// The entries one listing of the end of the walk's probe reads, the most an index listing
/// answers.
const PROBE_LIST_LIMIT: u32 = 100;

/// What the end of the walk's probe reads from one legacy bucket, so the probe's walk is tested
/// without Depot.
pub(crate) trait BucketEntries {
	/// Lists the bucket's live entries after `after`, newest first, at most `limit` of them.
	async fn list(
		&self,
		after: Option<&ns::NamespaceBackfillAfter>,
		limit: u32,
	) -> Result<wire::IndexListActorsResponse>;

	/// Reads the unprefixed state volume of the actor `actor_id`, `None` when it is gone.
	async fn read(&self, actor_id: Id) -> Result<Option<wire::ActorReadResponse>>;

	/// Whether the page's soft deadline passed.
	fn past_deadline(&self) -> Result<bool>;
}

/// The production [`BucketEntries`]: the legacy bucket `bucket` of `name` in `namespace`, read
/// through Depot, under the page's soft deadline.
struct DepotBucket<'a> {
	ctx: &'a EngineContext,
	namespace: &'a str,
	name: &'a str,
	bucket: u64,
	soft_boot_ms: u64,
}

impl BucketEntries for DepotBucket<'_> {
	async fn list(
		&self,
		after: Option<&ns::NamespaceBackfillAfter>,
		limit: u32,
	) -> Result<wire::IndexListActorsResponse> {
		let position = ns::NamespaceBackfillCursor {
			name: self.name.to_owned().into(),
			bucket: self.bucket,
			after: after.cloned(),
		};
		list_legacy_bucket(self.ctx, self.namespace, &position, limit).await
	}

	async fn read(&self, actor_id: Id) -> Result<Option<wire::ActorReadResponse>> {
		read_actor(self.ctx, &LegacyActorVolume::new(actor_id).volume_key()).await
	}

	fn past_deadline(&self) -> Result<bool> {
		Ok(self.ctx.env().clock().observe()?.boot_ms >= self.soft_boot_ms)
	}
}

/// Reads what the legacy bucket `bucket` of `name` still lists after its entry `after`, apart
/// from the actors given up on, and stops at the first entry that matters (see
/// [`probe_entry`]): a live actor of the namespace, a moved one whose entry its page has not
/// deleted yet, or one a request is still destroying. An entry whose actor is already destroyed,
/// or is not the namespace's, is left to the bucket's delete. It lists up to
/// [`PROBE_LIST_LIMIT`] entries at a time and passes the actors given up on without a read, so a
/// bucket costs one listing per hundred entries and one read per entry that is not skipped. The
/// deadline is checked before each listing and each read, and a probe that reaches it answers
/// the last entry it passed, where the next page resumes, so a bucket of any size is probed to
/// its end over enough pages.
pub(crate) async fn probe_bucket<B: BucketEntries>(
	entries: &B,
	namespace: &str,
	name: &str,
	bucket: u64,
	skipped: &[ns::NamespaceBackfillSkipped],
	mut after: Option<ns::NamespaceBackfillAfter>,
) -> Result<BucketProbe> {
	let position = ns::NamespaceBackfillCursor {
		name: name.to_owned().into(),
		bucket,
		after: None,
	};
	loop {
		if entries.past_deadline()? {
			return Ok(BucketProbe::Deadline { after });
		}
		let listed = entries.list(after.as_ref(), PROBE_LIST_LIMIT).await?;
		for entry in &listed.actors {
			let entry_after = ns::NamespaceBackfillAfter {
				create_unix_ms: entry.create_unix_ms,
				actor_id: entry.actor_id.clone(),
			};
			if !is_skipped(skipped, &position, &entry_after) {
				if entries.past_deadline()? {
					return Ok(BucketProbe::Deadline { after });
				}
				let read = entries.read(Id::from_slice(&entry.actor_id)?).await?;
				match probe_entry(read.as_ref(), namespace) {
					EntryProbe::Passed => {}
					EntryProbe::Destroying => return Ok(BucketProbe::Destroying { after }),
					EntryProbe::Live => return Ok(BucketProbe::Live),
				}
			}
			after = Some(entry_after);
		}
		if !listed.has_more {
			return Ok(BucketProbe::Empty);
		}
	}
}

/// What the end of the walk makes of the actor of one entry it listed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EntryProbe {
	/// The entry is left to the bucket's delete: its actor is destroyed, missing or not the
	/// namespace's.
	Passed,
	/// The actor is being destroyed by a request, and its destroy still deletes the entry.
	Destroying,
	/// The walk must reach the entry again: its actor is live, or the backfill moved it and its
	/// copy may still wait for its confirm, which only a page reaching the entry sends.
	Live,
}

/// Classifies the actor `read` of an entry the end of the walk listed in a bucket of
/// `namespace`. An ended actor the backfill moved keeps its entry until its page confirms the
/// copy and deletes the entry, so it counts as live: the walk returns to it and confirms a copy
/// a crash or a failed confirm left held. Pure, so the decision is tested on its own.
pub(crate) fn probe_entry(read: Option<&wire::ActorReadResponse>, namespace: &str) -> EntryProbe {
	let Some(read) = read else {
		return EntryProbe::Passed;
	};
	let Some(data) = read.data.as_ref() else {
		return EntryProbe::Passed;
	};
	if !crate::catalog::actor_in_namespace(None, namespace, data) {
		return EntryProbe::Passed;
	}
	if old_entry_left_to_page(read) {
		return EntryProbe::Live;
	}
	match read.phase.as_ref() {
		Some(wire::ActorPhase::Destroying(_)) => EntryProbe::Destroying,
		// A key delegated to V2 has no data record here, so the backfill has nothing to move.
		Some(wire::ActorPhase::Destroyed | wire::ActorPhase::DelegatedToV2) | None => {
			EntryProbe::Passed
		}
		Some(
			wire::ActorPhase::Initializing(_)
			| wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_)
			// A held actor is not moved; its bucket waits until the V2 migrator activates or
			// discards it.
			| wire::ActorPhase::MigratingFromV2(_),
		) => EntryProbe::Live,
	}
}

/// Whether the old actor `old` ended by its move's destroy, whose destroy keeps the actor's
/// index entry for the page to delete once the moved copy is confirmed.
pub(crate) fn old_entry_left_to_page(old: &wire::ActorReadResponse) -> bool {
	ended(old.phase.as_ref())
		&& matches!(
			old.legacy_move,
			Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(_))
		)
}

/// Whether a move that came to `result` leaves the old actor's index entry for its page to
/// delete: the move's own destroy of the old actor kept it (`Moved`, `Duplicate`, `Destroyed`,
/// each reached through that destroy), or the old actor had already ended by such a destroy when
/// the move read it (`old_moved`), whatever the resumed move then decided.
pub(crate) fn deletes_old_entry(old_moved: bool, result: &MoveResult) -> bool {
	match result {
		MoveResult::Moved | MoveResult::Duplicate | MoveResult::Destroyed => true,
		MoveResult::Skipped => old_moved,
	}
}

/// Deletes the legacy index entry of the old actor `legacy_id` of incarnation `create_unix_ms`
/// named `name` in `namespace`, which its move's destroy kept: the move is settled, so nothing
/// needs to reach the entry again. Idempotent: an entry already deleted keeps its first stamp.
async fn delete_old_entry(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	legacy_id: Id,
	create_unix_ms: i64,
	index_bucket_count: u64,
) -> Result<()> {
	let bucket = crate::keys::partition::ActorIndexVolume::for_actor_in_namespace(
		None,
		namespace.to_owned(),
		name.to_owned(),
		legacy_id,
		index_bucket_count,
	);
	ctx.depot()
		.command::<IndexDeleteActor>(
			&bucket,
			wire::IndexDeleteActorRequest {
				actor_id: legacy_id.as_bytes().into(),
				create_unix_ms,
			},
		)
		.await
		.context("delete a moved legacy actor's index entry")?;
	Ok(())
}

/// The actor names the backfill of `namespace` walks, in order: those every envoy record of
/// every configured pool names, connected or not, as its record last stored them, and those a
/// legacy fallback read recorded on the namespace's volume. A deleted pool's envoy records are
/// not read; see the module doc for the actors this misses.
async fn legacy_actor_names(ctx: &EngineContext, namespace: &str) -> Result<BTreeSet<String>> {
	let mut names = BTreeSet::new();
	for pool in crate::index::list_envoy_pools(ctx, namespace).await? {
		for envoy in crate::index::list_envoys(ctx, namespace, &pool).await? {
			names.extend(envoy.names.iter().map(|name| name.to_string()));
		}
	}

	let volume = NamespaceVolume::new(namespace.to_owned()).volume_key();
	let mut after = None;
	loop {
		let page = ctx
			.depot()
			.command::<LegacyBackfillNameList>(
				&volume,
				ns::LegacyBackfillNameListRequest {
					after,
					limit: NAME_LIST_LIMIT,
				},
			)
			.await
			.context("list the actor names recorded for the backfill")?;
		names.extend(page.names.iter().map(|name| name.to_string()));
		match page.cursor {
			Some(cursor) => after = Some(cursor),
			None => return Ok(names),
		}
	}
}

/// Legacy: records `name` as an actor name of `namespace` that a legacy fallback read found an
/// actor under, so the backfill walks its legacy buckets even when no envoy record names it. The
/// legacy behavior found names only in envoy records. Taken only by the fallback reads, while
/// the namespace's backfill is not done. Removed in 3.1 with the backfill.
///
/// Each name costs one write per process, remembered up to
/// `pegboard.backfill.recorded_names_capacity` until the process sees the namespace's backfill
/// done, and the write runs in the background, in a scope the process's background shutdown
/// cancels and joins, at most `pegboard.backfill.name_record_writes_in_flight` at once, so the
/// read never waits for it. Reaching that cap warns once, and again only after the writes
/// drained to half of it.
/// A name past either cap is not recorded and is counted in
/// `pegboard_backfill_name_records_skipped_total`: the read never writes for it, and the backfill
/// finds it only in the namespace's node records. A failed write is logged and forgotten, so a
/// later read records it again.
pub async fn legacy_record_actor_name(ctx: &EngineContext, namespace: &str, name: &str) {
	let config = ctx.config().pegboard().backfill.clone();
	let capacity = config.recorded_names_capacity();
	crate::metrics::BACKFILL_RECORDED_NAMES_CAPACITY.set(capacity as i64);
	let mark = ctx
		.legacy_mark_backfill_name(namespace, name, capacity)
		.await;
	crate::metrics::BACKFILL_RECORDED_NAMES.set(ctx.legacy_backfill_names_held() as i64);
	match mark {
		rivet_engine_context::LegacyNameMark::Recorded
		| rivet_engine_context::LegacyNameMark::NamespaceDone => return,
		rivet_engine_context::LegacyNameMark::Full => {
			crate::metrics::BACKFILL_NAME_RECORDS_SKIPPED
				.with_label_values(&[namespace])
				.inc();
			return;
		}
		// The mark that fills the set warns once; later names past it are skipped.
		rivet_engine_context::LegacyNameMark::Marked => {
			if ctx.legacy_backfill_names_held() >= capacity {
				tracing::warn!(
					namespace,
					capacity,
					"the backfill's recorded actor names reached their capacity; later names are not recorded and are found only in node records, raise pegboard.backfill.recorded_names_capacity"
				);
			}
		}
	}

	let writes_capacity = config.name_record_writes_in_flight();
	crate::metrics::BACKFILL_NAME_RECORD_WRITES_CAPACITY.set(writes_capacity as i64);
	let Some(slot) = ctx.legacy_begin_name_write(writes_capacity) else {
		ctx.legacy_unmark_backfill_name(namespace, name).await;
		crate::metrics::BACKFILL_NAME_RECORDS_SKIPPED
			.with_label_values(&[namespace])
			.inc();
		tracing::debug!(
			namespace,
			"every backfill name record write slot is taken; the name is left for a later read"
		);
		return;
	};
	let in_flight = crate::metrics::BACKFILL_NAME_RECORD_WRITES.inc_guard();
	if ctx.legacy_name_writes_reached_cap(ctx.legacy_name_writes_in_flight(), writes_capacity) {
		tracing::warn!(
			namespace,
			capacity = writes_capacity,
			"backfill name record writes reached their capacity; names read meanwhile are left for later reads, raise pegboard.backfill.name_record_writes_in_flight if this persists"
		);
	}

	let ctx2 = ctx.clone();
	let namespace = namespace.to_owned();
	let name = name.to_owned();
	ctx.legacy_spawn_name_write(async move {
		let _slot = slot;
		let _in_flight = in_flight;
		let volume = NamespaceVolume::new(namespace.clone()).volume_key();
		let written = ctx2
			.depot()
			.command::<crate::catalog::LegacyBackfillNameRecord>(
				&volume,
				ns::LegacyBackfillNameRecordRequest {
					name: name.clone().into(),
				},
			)
			.await;
		if let Err(error) = written {
			tracing::warn!(
				?error,
				namespace,
				"an actor name found through the legacy fallback was not recorded for the backfill; a later read records it again"
			);
			ctx2.legacy_unmark_backfill_name(&namespace, &name).await;
		}
	});
}

/// One page of live entries of the legacy bucket at `position`, after its `after`. A bucket
/// never created, or already deleted, is empty.
async fn list_legacy_bucket(
	ctx: &EngineContext,
	namespace: &str,
	position: &ns::NamespaceBackfillCursor,
	limit: u32,
) -> Result<wire::IndexListActorsResponse> {
	let volume = LegacyActorIndexVolume::new(
		namespace.to_owned(),
		position.name.to_string(),
		position.bucket,
	)
	.volume_key();
	let after = position.after.as_ref().map(|after| wire::IndexActorCursor {
		create_unix_ms: after.create_unix_ms,
		actor_id: after.actor_id.clone(),
	});
	let result = ctx
		.depot()
		.command::<IndexListActors>(
			&volume,
			wire::IndexListActorsRequest {
				after,
				limit,
				include_destroyed: false,
			},
		)
		.await;
	match result {
		Ok(page) => Ok(page),
		Err(error) if crate::commands::is_volume_gone(&error) => {
			Ok(wire::IndexListActorsResponse {
				actors: Vec::new(),
				has_more: false,
				destroyed: Vec::new(),
			})
		}
		Err(error) => Err(error),
	}
}

/// Reads the state volume `volume`, or `None` when it is missing or deleted.
async fn read_actor(
	ctx: &EngineContext,
	volume: &rivet_keys::keys::VolumeKey,
) -> Result<Option<wire::ActorReadResponse>> {
	match ctx.depot().command::<ActorRead>(volume, ()).await {
		Ok(read) => Ok(Some(read)),
		Err(error) if crate::commands::is_volume_gone(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// Whether a phase is one an actor leaves only through a new create.
fn ended(phase: Option<&wire::ActorPhase>) -> bool {
	match phase {
		None
		| Some(
			wire::ActorPhase::Destroying(_)
			| wire::ActorPhase::Destroyed
			| wire::ActorPhase::DelegatedToV2,
		) => true,
		Some(
			wire::ActorPhase::Initializing(_)
			| wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_)
			| wire::ActorPhase::MigratingFromV2(_),
		) => false,
	}
}

/// Who holds an actor's new id in the group.
enum Holder {
	/// Nobody alive: the id was never used, or its actor was destroyed.
	Free,
	/// The move of this same legacy actor, from this or an earlier page.
	Move,
	/// A live grouped actor created some other way.
	Other,
}

fn holder(read: Option<&wire::ActorReadResponse>, legacy_id: Id) -> Holder {
	let Some(read) = read else {
		return Holder::Free;
	};
	let Some(data) = &read.data else {
		return Holder::Free;
	};
	if ended(read.phase.as_ref()) {
		return Holder::Free;
	}
	match moved_from(data, legacy_id) {
		true => Holder::Move,
		false => Holder::Other,
	}
}

/// Whether the actor `data` is a moved copy of the legacy actor `legacy_id`.
fn moved_from(data: &wire::ActorData, legacy_id: Id) -> bool {
	data.legacy_moved_from
		.as_ref()
		.is_some_and(|moved_from| moved_from.as_ref() == legacy_id.as_bytes().as_slice())
}

/// Whether the read is a moved copy of `legacy_id` that a request destroyed while it was held
/// for its confirm, which destroys the actor.
fn copy_destroyed_by_request(read: Option<&wire::ActorReadResponse>, legacy_id: Id) -> bool {
	let Some(read) = read else {
		return false;
	};
	let ours = read
		.data
		.as_ref()
		.is_some_and(|data| moved_from(data, legacy_id));
	ours && ended(read.phase.as_ref())
		&& matches!(
			read.legacy_move,
			Some(wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed)
		)
}

/// The hold of a moved copy still held for its move's confirm.
fn hold(read: &wire::ActorReadResponse) -> Option<&wire::LegacyMoveHold> {
	match &read.phase {
		Some(wire::ActorPhase::Initializing(init)) => init.legacy_move.as_ref(),
		Some(
			wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_)
			| wire::ActorPhase::Destroying(_)
			| wire::ActorPhase::Destroyed
			| wire::ActorPhase::MigratingFromV2(_)
			| wire::ActorPhase::DelegatedToV2,
		)
		| None => None,
	}
}

/// The incarnation of the actor read, by its creation time.
fn incarnation(read: &wire::ActorReadResponse) -> Result<i64> {
	Ok(read
		.data
		.as_ref()
		.context("a moved actor has no data")?
		.create_unix_ms)
}

/// What a move did to the durable state it must undo when it fails before the old actor's
/// destroy: the copy it created, by incarnation, and whether the old actor's destroy was sent
/// and may have committed. That holds from the send on, through a committed destroy whose
/// confirm fails, so the copy is then never abandoned: a resumed page or a give-up reads the
/// old actor and confirms the copy if the destroy committed. Only a destroy the read-back proves
/// refused unsets it, so the copy is undone. `keeps_mark` is set when the move failed because
/// the old actor was still stopping or an earlier copy's accepted abandon had not ended, so the
/// undo leaves the mark; see [`StillStopping`] and [`AbandonPending`].
#[derive(Default)]
struct MoveProgress {
	copy_unix_ms: Option<i64>,
	old_destroy_sent: bool,
	keeps_mark: bool,
}

/// Legacy: moves the unprefixed actor `legacy_id` of `namespace` into `group` as the page of
/// `mover`; see the module doc. Every write the move makes to an actor names `mover`: the old
/// actor holds the move's mark from before its database is claimed, refuses the move's destroy
/// once a later page marked it, and a copy is abandoned or confirmed only by its incarnation, so
/// a page that outlived its turn cannot destroy a copy it does not own or confirm one whose old
/// actor was not destroyed for it.
///
/// A page sent again resumes a move a crash or a timeout interrupted: a moved copy still held
/// for its confirm was made under a claim this move no longer holds, so it is abandoned and the
/// move redone; a copy whose old actor this move's mover destroyed is confirmed; a copy a request
/// destroyed while it was held means the actor was destroyed, so the old actor is destroyed too.
/// A failure before the old actor's destroy abandons the copy this attempt made and clears its
/// mark, so the old actor serves wakes until the next attempt. Once the move is decided through
/// the move's destroy of the old actor, it deletes the old actor's index entry in bucket count
/// `index_bucket_count`, which that destroy keeps; see [`deletes_old_entry`]. Removed in 3.1
/// with the backfill.
#[allow(clippy::too_many_arguments, reason = "one move's inputs")]
pub(crate) async fn legacy_move_actor(
	ctx: &EngineContext,
	namespace: &str,
	group: GroupId,
	legacy_id: Id,
	index_bucket_count: u64,
	mover: u64,
	deadline_boot_ms: u64,
	stop_by_boot_ms: u64,
) -> Result<MoveResult> {
	let mut old_entry = None;
	let result = move_actor(
		ctx,
		namespace,
		group,
		legacy_id,
		mover,
		deadline_boot_ms,
		stop_by_boot_ms,
		&mut old_entry,
	)
	.await?;
	settle_old_entry(
		ctx,
		namespace,
		legacy_id,
		index_bucket_count,
		old_entry,
		&result,
	)
	.await?;
	Ok(result)
}

/// The old actor as a move read it, for settling its index entry once the move is decided.
struct OldEntry {
	name: String,
	create_unix_ms: i64,
	/// Whether the old actor had already ended by its move's destroy when the move read it.
	moved: bool,
}

/// Deletes the index entry the old actor's move kept, once the move came to `result`; see
/// [`deletes_old_entry`]. A failed delete fails the page, whose next send reaches the entry
/// again, since the end of the walk counts a moved old actor's entry as live.
async fn settle_old_entry(
	ctx: &EngineContext,
	namespace: &str,
	legacy_id: Id,
	index_bucket_count: u64,
	old_entry: Option<OldEntry>,
	result: &MoveResult,
) -> Result<()> {
	let Some(old_entry) = old_entry else {
		return Ok(());
	};
	if !deletes_old_entry(old_entry.moved, result) {
		return Ok(());
	}
	delete_old_entry(
		ctx,
		namespace,
		&old_entry.name,
		legacy_id,
		old_entry.create_unix_ms,
		index_bucket_count,
	)
	.await
}

/// The move of [`legacy_move_actor`], before the old actor's index entry is settled. Records the
/// old actor in `old_entry` once it read it as the namespace's.
#[allow(clippy::too_many_arguments, reason = "one move's inputs")]
async fn move_actor(
	ctx: &EngineContext,
	namespace: &str,
	group: GroupId,
	legacy_id: Id,
	mover: u64,
	deadline_boot_ms: u64,
	stop_by_boot_ms: u64,
	old_entry: &mut Option<OldEntry>,
) -> Result<MoveResult> {
	let legacy_volume = LegacyActorVolume::new(legacy_id).volume_key();
	let Some(old) = read_actor(ctx, &legacy_volume).await? else {
		return Ok(MoveResult::Skipped);
	};
	let Some(data) = old.data.clone() else {
		return Ok(MoveResult::Skipped);
	};
	if !crate::catalog::actor_in_namespace(None, namespace, &data) {
		return Ok(MoveResult::Skipped);
	}
	let now_boot_ms = ctx.env().clock().observe()?.boot_ms;
	if !has_time_to_stop(old.phase.as_ref(), now_boot_ms, stop_by_boot_ms) {
		bail!(NoTimeToStop);
	}

	// A keyed actor's id hashes its name and key alone in the group; a random id is kept.
	let new_id = match &data.key {
		Some(key) => crate::actor_key::keyed_actor_id(legacy_id.label(), &data.name, key),
		None => legacy_id,
	};
	let new_volume = ActorVolume::new(group, new_id).volume_key();

	// From here the old actor refuses wakes, so a request can neither restart it after the claim
	// fences it nor keep it held in this process, its alarm waits for the move, and every later
	// write of this move is fenced by its mover. Reading it back tells whether the mark took.
	// The mark ends by itself one page timeout later, which outlasts this move, so an actor
	// whose move stopped making progress serves wakes again.
	if !ended(old.phase.as_ref()) {
		let mark = wire::LegacyMarkMoving {
			mover,
			for_ms: page_timeout_ms(ctx.config().pegboard()),
		};
		send(
			ctx,
			&legacy_volume,
			wire::ActorEvent::LegacyMarkMoving(mark),
		)
		.await
		.context("mark a legacy actor as moving")?;
	}
	let old = read_actor(ctx, &legacy_volume)
		.await?
		.context("a legacy actor's state volume is missing after it was read")?;
	*old_entry = Some(OldEntry {
		name: data.name.to_string(),
		create_unix_ms: data.create_unix_ms,
		moved: old_entry_left_to_page(&old),
	});
	let existing = read_actor(ctx, &new_volume).await?;

	// A held copy whose database fork is still running waits on the old database's tail, and its
	// abandon waits on that fork. When the attempt that made it died with the old database's
	// maintainer, before that tail was cold, nothing proves the tail until the old database is
	// opened again, so the move claims it first: the claim's open recovers the tail, the fork
	// finishes, and the abandon below can complete within this page. The claim is the one the
	// redone move then holds.
	//
	// The old actor may have woken since that attempt's mark lapsed, so it is put to sleep
	// first, as `move_marked` does before its own claim: no claim fences a running old actor.
	let mut progress = MoveProgress::default();
	let early_claim = if claims_before_plan(&old, existing.as_ref(), legacy_id, mover) {
		let database = LegacyActorDatabaseVolume::new(legacy_id, data.create_unix_ms).volume_key();
		let claimed = match sleep_old(ctx, &legacy_volume, deadline_boot_ms).await {
			Ok(()) => claim_database(ctx, &legacy_volume, &database, deadline_boot_ms).await,
			Err(error) => Err(error),
		};
		match claimed {
			Ok(claim) => Some(claim),
			Err(error) => {
				// Nothing of this attempt was made yet, but the previous attempt's held copy is
				// known, since only one still forking claims early: the undo abandons it too, so
				// it holds no key reservation, and clears the mark, so the old actor serves wakes
				// until the next attempt, unless the actor was still stopping. The abandon is best
				// effort; the next attempt abandons the copy again if this one is refused.
				progress.copy_unix_ms = existing.as_ref().and_then(|read| incarnation(read).ok());
				progress.keeps_mark = still_stopping(&error);
				undo_failed_move(ctx, &legacy_volume, &new_volume, mover, &progress).await;
				return Err(error);
			}
		}
	} else {
		None
	};

	let plan = match plan_move(&old, existing.as_ref(), legacy_id, mover) {
		Ok(plan) => plan,
		Err(error) => {
			release_early(early_claim).await;
			return Err(error);
		}
	};
	let moved = match plan {
		MovePlan::Done(result) => {
			release_early(early_claim).await;
			return Ok(result);
		}
		MovePlan::Superseded => {
			release_early(early_claim).await;
			bail!("a later backfill page holds this legacy actor's moving mark")
		}
		MovePlan::Confirm {
			copy_unix_ms,
			old_epoch,
			mover: copy_mover,
		} => {
			release_early(early_claim).await;
			return confirm_move(ctx, &new_volume, copy_unix_ms, old_epoch, copy_mover).await;
		}
		MovePlan::AwaitOldStop {
			copy_unix_ms,
			mover: copy_mover,
		} => {
			release_early(early_claim).await;
			let old_epoch =
				wait_old_ended(ctx, &legacy_volume, copy_mover, deadline_boot_ms).await?;
			return confirm_move(ctx, &new_volume, copy_unix_ms, old_epoch, copy_mover).await;
		}
		MovePlan::AbandonAndSkip { copy_unix_ms } => {
			release_early(early_claim).await;
			abandon_copy(ctx, &new_volume, copy_unix_ms, mover, deadline_boot_ms).await?;
			return Ok(MoveResult::Skipped);
		}
		MovePlan::DestroyOld => {
			release_early(early_claim).await;
			destroy_for_request(ctx, &legacy_volume, mover).await
		}
		MovePlan::Duplicate => {
			release_early(early_claim).await;
			destroy_duplicate(ctx, namespace, &data, &legacy_volume, mover).await
		}
		MovePlan::AbandonAndMove { copy_unix_ms } => {
			// A copy made under an earlier attempt's claim, which lapsed with that attempt: an
			// opener may have committed to the old database after the copy's cut. It never
			// started, so it is abandoned and the move redone from a fresh claim.
			match abandon_copy(ctx, &new_volume, copy_unix_ms, mover, deadline_boot_ms).await {
				Ok(()) => {
					move_marked(
						ctx,
						namespace,
						&data,
						legacy_id,
						new_id,
						&new_volume,
						mover,
						deadline_boot_ms,
						early_claim,
						&mut progress,
					)
					.await
				}
				Err(error) => {
					progress.keeps_mark = abandon_pending(&error);
					release_early(early_claim).await;
					Err(error)
				}
			}
		}
		MovePlan::Move => {
			move_marked(
				ctx,
				namespace,
				&data,
				legacy_id,
				new_id,
				&new_volume,
				mover,
				deadline_boot_ms,
				early_claim,
				&mut progress,
			)
			.await
		}
	};
	if moved.is_err() && !progress.old_destroy_sent {
		undo_failed_move(ctx, &legacy_volume, &new_volume, mover, &progress).await;
	}
	moved
}

/// Legacy: whether a resumed page claims the old database before planning: the old actor is
/// live and holds this page's mark, and the actor at its new id is a copy of it still held for
/// its move's confirm whose database fork has not finished. That fork may be waiting on an old
/// database tail only a new claim can prove (see `legacy_move_actor`). Pure, like `plan_move`.
///
/// Whenever this holds, `plan_move` answers `AbandonAndMove`: the old actor is live under this
/// page's mark and the copy is held by the move, unconfirmed. The early claim is released on
/// every other plan only so that a later change to `plan_move` cannot leak it.
/// Removed in 3.1 with the backfill.
fn claims_before_plan(
	old: &wire::ActorReadResponse,
	existing: Option<&wire::ActorReadResponse>,
	legacy_id: Id,
	mover: u64,
) -> bool {
	if ended(old.phase.as_ref()) || mark_mover(old) != Some(mover) {
		return false;
	}
	let Some(read) = existing else {
		return false;
	};
	if !matches!(holder(Some(read), legacy_id), Holder::Move) || hold(read).is_none() {
		return false;
	}
	match &read.phase {
		Some(wire::ActorPhase::Initializing(init)) => init
			.fork_database
			.as_ref()
			.is_some_and(|fork| fork_in_progress(&fork.state.phase)),
		Some(
			wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_)
			| wire::ActorPhase::MigratingFromV2(_)
			| wire::ActorPhase::DelegatedToV2
			| wire::ActorPhase::Destroying(_)
			| wire::ActorPhase::Destroyed,
		)
		| None => false,
	}
}

/// Whether a database fork in `phase` still has a step to run.
fn fork_in_progress(phase: &wire::ForkDatabasePhase) -> bool {
	match phase {
		wire::ForkDatabasePhase::ForkDatabasePhasePreparing
		| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_) => true,
		wire::ForkDatabasePhase::ForkDatabasePhaseForked
		| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => false,
	}
}

/// Closes a claim `claims_before_plan` took when the plan does not redo the move under it.
async fn release_early(claim: Option<Claim>) {
	if let Some(Claim::Claimed(claim)) = claim {
		release(Some(claim)).await;
	}
}

/// The mover of a `LegacyMoveMarkMoving` the actor read holds.
fn mark_mover(read: &wire::ActorReadResponse) -> Option<u64> {
	match &read.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => Some(moving.mover),
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => None,
	}
}

/// The mover of the `LegacyMoveMarkMoved` the actor read holds: the move whose destroy it
/// accepted.
fn moved_mover(read: &wire::ActorReadResponse) -> Option<u64> {
	match &read.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(moved)) => Some(moved.mover),
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoving(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => None,
	}
}

/// What a move does next, decided from the old actor read back after its mark and the actor at
/// its new id.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MovePlan {
	/// Nothing is left to do.
	Done(MoveResult),
	/// The old actor holds a later page's mark, so this page stops.
	Superseded,
	/// The old actor was destroyed by the move of `mover`, whose claim check passed for its held
	/// copy of incarnation `copy_unix_ms`: a crash or timeout came between that destroy and the
	/// confirm, which is sent now with the old actor's final epoch.
	Confirm {
		copy_unix_ms: i64,
		old_epoch: u32,
		mover: u64,
	},
	/// The move of `mover`, whose claim check passed for its held copy of incarnation
	/// `copy_unix_ms`, destroyed the old actor, which is still stopping: the copy is confirmed
	/// once the old actor's epoch has ended, never before, so it cannot run beside the old
	/// generation.
	AwaitOldStop { copy_unix_ms: i64, mover: u64 },
	/// The old actor ended without being moved, by a request, so the actor was destroyed: the
	/// held copy an attempt left goes too.
	AbandonAndSkip { copy_unix_ms: i64 },
	/// A request destroyed the held copy an attempt left, which destroyed the actor: the old
	/// actor is destroyed too instead of moved again.
	DestroyOld,
	/// A live grouped actor not made by the move holds the new id.
	Duplicate,
	/// A held copy made under an earlier attempt's claim is abandoned and the move redone.
	AbandonAndMove { copy_unix_ms: i64 },
	/// The new id is free: the move runs.
	Move,
}

/// Decides what the move of `legacy_id` under `mover` does next from `old`, the old actor read
/// back after the move marked it, and `existing`, the actor at its new id. Pure, so every resume
/// branch is decided the same way wherever it is tested.
pub(crate) fn plan_move(
	old: &wire::ActorReadResponse,
	existing: Option<&wire::ActorReadResponse>,
	legacy_id: Id,
	mover: u64,
) -> Result<MovePlan> {
	let held_copy = existing.and_then(|read| match holder(Some(read), legacy_id) {
		Holder::Move => Some(read),
		Holder::Free | Holder::Other => None,
	});

	if ended(old.phase.as_ref()) {
		let Some(read) = held_copy else {
			return Ok(MovePlan::Done(MoveResult::Skipped));
		};
		let Some(held) = hold(read) else {
			// Confirmed already.
			return Ok(MovePlan::Done(MoveResult::Moved));
		};
		let copy_unix_ms = incarnation(read)?;
		if moved_mover(old) == Some(held.mover) {
			return Ok(MovePlan::Confirm {
				copy_unix_ms,
				old_epoch: old.epoch,
				mover: held.mover,
			});
		}
		return Ok(MovePlan::AbandonAndSkip { copy_unix_ms });
	}

	if let (Some(destroyed_by), Some(read)) = (moved_mover(old), held_copy)
		&& let Some(held) = hold(read)
		&& held.mover == destroyed_by
	{
		return Ok(MovePlan::AwaitOldStop {
			copy_unix_ms: incarnation(read)?,
			mover: destroyed_by,
		});
	}
	if mark_mover(old) != Some(mover) {
		return Ok(MovePlan::Superseded);
	}
	if copy_destroyed_by_request(existing, legacy_id) {
		return Ok(MovePlan::DestroyOld);
	}
	match holder(existing, legacy_id) {
		Holder::Free => Ok(MovePlan::Move),
		Holder::Other => Ok(MovePlan::Duplicate),
		Holder::Move => {
			let Some(read) = existing else {
				bail!("a moved actor's state volume is missing after its create");
			};
			if hold(read).is_none() {
				bail!("a moved copy was confirmed while its old actor holds this move's mark");
			}
			Ok(MovePlan::AbandonAndMove {
				copy_unix_ms: incarnation(read)?,
			})
		}
	}
}

/// The move of a legacy actor whose moving mark `mover` holds; see [`legacy_move_actor`].
#[allow(clippy::too_many_arguments, reason = "one move's inputs")]
async fn move_marked(
	ctx: &EngineContext,
	namespace: &str,
	data: &wire::ActorData,
	legacy_id: Id,
	new_id: Id,
	new_volume: &VolumeKey,
	mover: u64,
	deadline_boot_ms: u64,
	early_claim: Option<Claim>,
	progress: &mut MoveProgress,
) -> Result<MoveResult> {
	let legacy_volume = LegacyActorVolume::new(legacy_id).volume_key();

	// A running old actor is put to sleep first, so its runtime stops through its sleep while it
	// still holds its database: its shutdown save commits there and the fork below copies it,
	// and it never runs the destroy of an actor that lives on as its copy. The move's mark keeps
	// it asleep. Its stop deadline bounds the wait. An actor still awake or stopping at the
	// page's deadline fails the move before any claim and keeps the mark, so the next page
	// resumes the same stop; a failed read fails it and clears the mark.
	if let Err(error) = sleep_old(ctx, &legacy_volume, deadline_boot_ms).await {
		progress.keeps_mark = still_stopping(&error);
		release_early(early_claim).await;
		return Err(error);
	}

	// Claim the old database, which fences the process holding it: its next commit fails and
	// the actor stops. An actor that never opened its database has none to copy. A resumed page
	// may have claimed it already, before planning.
	let database = LegacyActorDatabaseVolume::new(legacy_id, data.create_unix_ms).volume_key();
	let claimed = match early_claim {
		Some(claimed) => claimed,
		None => claim_database(ctx, &legacy_volume, &database, deadline_boot_ms).await?,
	};
	let claim = match claimed {
		Claim::Claimed(claim) => Some(claim),
		Claim::NeverCreated => None,
		Claim::OldEnded => return Ok(MoveResult::Skipped),
	};

	let created = create_copy(
		ctx,
		data,
		legacy_id,
		new_id,
		new_volume,
		&database,
		claim.is_some(),
		mover,
		deadline_boot_ms,
		progress,
	)
	.await;
	let ready = match created {
		Ok(Created::Ready(ready)) => ready,
		Ok(Created::OldEnded) => {
			release(claim).await;
			return Ok(MoveResult::Skipped);
		}
		Ok(Created::Duplicate) => {
			release(claim).await;
			return destroy_duplicate(ctx, namespace, data, &legacy_volume, mover).await;
		}
		Ok(Created::DestroyedByRequest) => {
			release(claim).await;
			return destroy_for_request(ctx, &legacy_volume, mover).await;
		}
		Err(error) => {
			release(claim).await;
			return Err(error);
		}
	};

	// The fork published while this move held the claim. A committed entry proves the claim
	// still holds: had another opener claimed the volume since, it could have committed past the
	// fork's cut, so the copy, which never started, is abandoned and the move starts over on the
	// next page.
	let checked = match &claim {
		Some(claimed) => check_claim(claimed, deadline_boot_ms).await,
		None => Ok(()),
	};
	if let Err(error) = checked {
		tracing::warn!(
			?error,
			namespace,
			actor_id = %legacy_id,
			"a backfill move lost its claim on the old database before it finished; the moved copy is abandoned and the move starts over"
		);
		release(claim).await;
		bail!("the backfill lost its claim on a legacy actor's database");
	}

	// The old actor's destroy commits while the claim still fences its database, then the copy
	// is confirmed and may start once the old actor's epoch has ended. The destroy is the move's
	// own only while its mark holds. From the send on, the copy is kept whatever fails next, the
	// wait and the confirm included, since the destroy may have committed and the copy then
	// holds the only data; only a destroy the read-back proves refused committed nothing, so only
	// then is the copy undone.
	progress.old_destroy_sent = true;
	let destroyed = destroy_old(ctx, &legacy_volume, mover).await;
	release(claim).await;
	progress.old_destroy_sent = old_destroy_may_have_committed(&destroyed);
	let destroyed = destroyed?;
	let old_epoch = match destroyed {
		OldDestroy::Destroyed { epoch } => epoch,
		OldDestroy::Stopping => {
			wait_old_ended(ctx, &legacy_volume, mover, deadline_boot_ms).await?
		}
		OldDestroy::EndedByRequest => {
			// A request destroyed the old actor first, so the actor is destroyed: the copy goes.
			abandon_copy(
				ctx,
				new_volume,
				incarnation(&ready)?,
				mover,
				deadline_boot_ms,
			)
			.await?;
			return Ok(MoveResult::Skipped);
		}
		OldDestroy::Refused => bail!(OLD_DESTROY_REFUSED),
		OldDestroy::NotAdmitted(error) => return Err(error),
	};
	let result = confirm_move(ctx, new_volume, incarnation(&ready)?, old_epoch, mover).await?;
	crate::metrics::BACKFILL_ACTORS_MOVED
		.with_label_values(&[namespace])
		.inc();
	Ok(result)
}

/// Undoes what a failed move did before the old actor's destroy: abandons the copy it created,
/// then clears its mark, so the old actor serves wakes until the next attempt and no held copy
/// blocks its key meanwhile. A move whose old actor was still stopping, or whose earlier copy's
/// accepted abandon had not ended, keeps the mark instead, so a wake cannot restart the actor
/// before the next page resumes the move; that page marks it again under its own mover, and the
/// mark's deadline, one page timeout, ends it otherwise. Best effort: whatever is left, the next
/// attempt abandons the copy, and the mark's deadline clears it.
async fn undo_failed_move(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	new_volume: &VolumeKey,
	mover: u64,
	progress: &MoveProgress,
) {
	if let Some(copy_unix_ms) = progress.copy_unix_ms {
		let abandon = wire::ActorEvent::LegacyMoveAbandon(wire::LegacyMoveAbandon {
			create_unix_ms: copy_unix_ms,
			mover,
		});
		if let Err(error) = send(ctx, new_volume, abandon).await {
			tracing::debug!(
				?error,
				"a failed backfill move's copy was not abandoned; the next attempt abandons it"
			);
		}
	}
	if progress.keeps_mark {
		return;
	}
	let clear = wire::ActorEvent::LegacyClearMoving(wire::LegacyClearMoving { mover });
	if let Err(error) = send(ctx, legacy_volume, clear).await {
		tracing::debug!(
			?error,
			"a failed backfill move's mark was not cleared; the next attempt replaces it"
		);
	}
}

/// Sends `event` to the actor at `volume`.
async fn send(ctx: &EngineContext, volume: &VolumeKey, event: wire::ActorEvent) -> Result<()> {
	ctx.depot().command::<ActorEvent>(volume, event).await?;
	Ok(())
}

/// What the move's destroy of the old actor did.
#[derive(Debug)]
enum OldDestroy {
	/// The destroy was the move's own and committed, and the old actor's epoch has ended;
	/// `epoch` is the old actor's epoch after it, which no later start of the old actor passes.
	Destroyed { epoch: u32 },
	/// The destroy was the move's own and committed, but the old actor is still stopping or
	/// starting, so its generation may still run: the copy waits until the old actor ended.
	Stopping,
	/// A request destroyed the old actor first, so the actor is destroyed rather than moved.
	EndedByRequest,
	/// The actor refused the destroy: it is live and no longer holds the move's mark, because a
	/// later page took it or it was cleared or expired. Nothing was destroyed.
	Refused,
	/// Depot refused the destroy at admission with a retry hint, the error carried, so it never
	/// reached the actor and nothing was destroyed.
	NotAdmitted(anyhow::Error),
}

/// Whether the move's destroy of the old actor may have committed, so the copy must be kept on
/// any later failure: true for every outcome but a refusal the read-back proved and a destroy
/// Depot refused at admission. A committed destroy whose confirm then fails, refused at
/// admission, timed out or unread, leaves a held copy the next page send confirms, since the old
/// actor ended `Moved` for this mover. A destroy that was not admitted is undone at once, so the
/// old actor serves wakes again instead of refusing them until its mark expires.
fn old_destroy_may_have_committed(destroyed: &Result<OldDestroy>) -> bool {
	match destroyed {
		Ok(OldDestroy::Refused | OldDestroy::NotAdmitted(_)) => false,
		Ok(OldDestroy::Destroyed { .. } | OldDestroy::Stopping | OldDestroy::EndedByRequest)
		| Err(_) => true,
	}
}

/// Why a move's destroy of the old actor was refused.
const OLD_DESTROY_REFUSED: &str =
	"this legacy actor no longer held the backfill page's moving mark at its destroy";

/// Destroys the old actor at `legacy_volume` for the move of `mover` and reads back that the
/// destroy was the move's own: the actor refuses it unless it still holds the move's mark.
async fn destroy_old(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	mover: u64,
) -> Result<OldDestroy> {
	let destroy = wire::ActorEvent::LegacyMoveDestroy(wire::LegacyMoveDestroy { mover });
	if let Err(error) = send(ctx, legacy_volume, destroy).await {
		let error = error.context("destroy a moved legacy actor");
		// A refusal with a retry hint is a mutation Depot did not admit; any other failure may
		// have committed it.
		if depot_client::retry_after_ms(&error).is_some() {
			return Ok(OldDestroy::NotAdmitted(error));
		}
		return Err(error);
	}
	let old = read_actor(ctx, legacy_volume)
		.await?
		.context("a moved legacy actor's state volume is missing")?;
	Ok(classify_old_destroy(&old, mover))
}

/// Classifies the old actor `old`, read back after the move of `mover` sent its destroy. The
/// mark turns `Moved` when the destroy is accepted, which for a starting, running or stopping
/// actor is before its runtime stopped, so only an ended phase counts as destroyed. Pure, so the
/// barrier is tested on its own.
fn classify_old_destroy(old: &wire::ActorReadResponse, mover: u64) -> OldDestroy {
	let ended = ended(old.phase.as_ref());
	if moved_mover(old) == Some(mover) {
		return match ended {
			true => OldDestroy::Destroyed { epoch: old.epoch },
			false => OldDestroy::Stopping,
		};
	}
	match ended {
		true => OldDestroy::EndedByRequest,
		false => OldDestroy::Refused,
	}
}

/// Waits until the old actor at `legacy_volume`, whose destroy the move of `mover` committed,
/// has ended, and returns its epoch then, which the copy's confirm raises the copy's epoch
/// above. The old actor ends only through its stopped report, its envoy proven gone or its stop
/// deadline, the same barrier the actor state machine keeps before its own next epoch. A wait
/// past `deadline_boot_ms` fails the page with the copy still held, and the next page's
/// `AwaitOldStop` resumes it. Nothing notifies the end, so the wait reads the old actor at
/// `MOVE_POLL_INTERVAL`, as `wait_ready` does.
async fn wait_old_ended(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	mover: u64,
	deadline_boot_ms: u64,
) -> Result<u32> {
	let clock = ctx.env().clock();
	loop {
		let old = read_actor(ctx, legacy_volume)
			.await?
			.context("a moved legacy actor's state volume is missing")?;
		match classify_old_destroy(&old, mover) {
			OldDestroy::Destroyed { epoch } => return Ok(epoch),
			OldDestroy::Stopping => {}
			OldDestroy::EndedByRequest | OldDestroy::Refused | OldDestroy::NotAdmitted(_) => {
				bail!(
					"a legacy actor whose move's destroy committed no longer holds its move's mark"
				)
			}
		}
		if clock.observe()?.boot_ms >= deadline_boot_ms {
			bail!(
				"a moved legacy actor was still stopping at the end of the backfill page; the next page waits again"
			);
		}
		rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
	}
}

/// Asks the old actor at `legacy_volume` to sleep when it is starting or running, and waits
/// until it sleeps or ended, so the move's claim fences no running runtime and its destroy
/// reaches an actor whose runtime already stopped through its sleep. The move's mark keeps it
/// asleep. The sleep is sent again while the actor reads as starting or running, so one that
/// raced a start still reaches it, backing off as `claim_database` does.
///
/// An actor still starting, running or stopping at `deadline_boot_ms` fails the move with
/// [`StillStopping`]: a claim then would fence a runtime whose shutdown save has not committed.
/// The move's undo keeps its mark, so the actor stays asleep once it stops, and the next page
/// resumes the stop. Its stop deadline bounds how long it stops, and the page timeout leaves a
/// page's first move more time than that; see [`page_timeout_ms`].
async fn sleep_old(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	deadline_boot_ms: u64,
) -> Result<()> {
	let clock = ctx.env().clock();
	let page_timeout_ms = page_timeout_ms(ctx.config().pegboard());
	let max_retry_interval = Duration::from_millis(page_timeout_ms / 16)
		.clamp(MOVE_POLL_INTERVAL, CLAIM_RETRY_MAX_INTERVAL);
	let mut retry_interval = MOVE_POLL_INTERVAL;
	let mut next_sleep_boot_ms = 0;
	loop {
		let old = read_actor(ctx, legacy_volume)
			.await?
			.context("a legacy actor's state volume is missing during its move")?;
		let now_boot_ms = clock.observe()?.boot_ms;
		match old.phase.as_ref() {
			Some(wire::ActorPhase::Starting(_) | wire::ActorPhase::Running(_)) => {
				if now_boot_ms >= next_sleep_boot_ms {
					send(ctx, legacy_volume, wire::ActorEvent::Sleep)
						.await
						.context("ask a moved legacy actor to sleep before its destroy")?;
					next_sleep_boot_ms = now_boot_ms.saturating_add(duration_ms(retry_interval));
					retry_interval = retry_interval.saturating_mul(2).min(max_retry_interval);
				}
			}
			Some(wire::ActorPhase::Stopping(_)) => {}
			Some(
				wire::ActorPhase::Initializing(_)
				| wire::ActorPhase::Sleeping(_)
				| wire::ActorPhase::Destroying(_)
				| wire::ActorPhase::Destroyed
				| wire::ActorPhase::MigratingFromV2(_)
				| wire::ActorPhase::DelegatedToV2,
			)
			| None => return Ok(()),
		}
		if now_boot_ms >= deadline_boot_ms {
			bail!(StillStopping);
		}
		rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
	}
}

/// `duration` in whole milliseconds, saturating.
fn duration_ms(duration: Duration) -> u64 {
	u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Legacy: whether a page may start the move of an old actor in `phase` at `now_boot_ms`. An
/// actor starting, running or stopping must stop before its database is claimed, which can take
/// its whole stop deadline, so its move starts only until `stop_by_boot_ms`; any other actor's
/// move starts at once. Pure, so it is tested on its own. Removed in 3.1 with the backfill.
pub(crate) fn has_time_to_stop(
	phase: Option<&wire::ActorPhase>,
	now_boot_ms: u64,
	stop_by_boot_ms: u64,
) -> bool {
	let must_stop = match phase {
		Some(
			wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_),
		) => true,
		Some(
			wire::ActorPhase::Initializing(_)
			| wire::ActorPhase::Sleeping(_)
			| wire::ActorPhase::Destroying(_)
			| wire::ActorPhase::Destroyed
			| wire::ActorPhase::MigratingFromV2(_)
			| wire::ActorPhase::DelegatedToV2,
		)
		| None => false,
	};
	!must_stop || now_boot_ms <= stop_by_boot_ms
}

/// Legacy: whether a move failed because its old actor was still stopping at the page's
/// deadline, so its undo keeps the mark; see [`StillStopping`]. Removed in 3.1 with the
/// backfill.
pub(crate) fn still_stopping(error: &anyhow::Error) -> bool {
	error.downcast_ref::<StillStopping>().is_some()
}

/// Destroys the old actor at `legacy_volume` because a request destroyed its held copy, which
/// destroyed the actor. Fenced by the move's mark like the move's own destroy.
async fn destroy_for_request(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	mover: u64,
) -> Result<MoveResult> {
	match destroy_old(ctx, legacy_volume, mover).await? {
		OldDestroy::Destroyed { .. } | OldDestroy::Stopping | OldDestroy::EndedByRequest => {
			Ok(MoveResult::Destroyed)
		}
		OldDestroy::Refused => bail!(OLD_DESTROY_REFUSED),
		OldDestroy::NotAdmitted(error) => Err(error),
	}
}

/// Confirms the move of the held copy of incarnation `copy_unix_ms` at `new_volume`, made by
/// `mover`, with the old actor's epoch read after its destroy, and reads back that the confirm
/// was accepted: the copy left its hold in that same incarnation. A copy a request destroyed
/// while it was held answers `Destroyed`, since the actor was destroyed after its move.
async fn confirm_move(
	ctx: &EngineContext,
	new_volume: &VolumeKey,
	copy_unix_ms: i64,
	old_epoch: u32,
	mover: u64,
) -> Result<MoveResult> {
	let confirm = wire::ActorEvent::LegacyMoveConfirm(wire::LegacyMoveConfirm {
		create_unix_ms: copy_unix_ms,
		old_epoch,
		mover,
	});
	send(ctx, new_volume, confirm)
		.await
		.context("confirm a backfill move")?;
	let read = read_actor(ctx, new_volume)
		.await?
		.context("a moved actor's state volume is missing at its confirm")?;
	if incarnation(&read)? != copy_unix_ms {
		bail!("a moved copy was replaced by another incarnation before its confirm");
	}
	if read.legacy_move == Some(wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed) {
		return Ok(MoveResult::Destroyed);
	}
	if hold(&read).is_some() || ended(read.phase.as_ref()) {
		bail!("a backfill move's confirm was not accepted");
	}
	Ok(MoveResult::Moved)
}

/// Abandons the held copy of incarnation `copy_unix_ms` at `volume`, which an earlier or failed
/// attempt made, and waits until it is destroyed, so the move can create its next incarnation.
/// The copy refuses it unless `mover` is at least the copy's own, so a page that outlived its
/// turn cannot abandon a later page's copy. A copy that accepted the abandon but is not destroyed
/// by `deadline_boot_ms` fails with [`AbandonPending`].
async fn abandon_copy(
	ctx: &EngineContext,
	volume: &VolumeKey,
	copy_unix_ms: i64,
	mover: u64,
	deadline_boot_ms: u64,
) -> Result<()> {
	let abandon = wire::ActorEvent::LegacyMoveAbandon(wire::LegacyMoveAbandon {
		create_unix_ms: copy_unix_ms,
		mover,
	});
	send(ctx, volume, abandon)
		.await
		.context("abandon a moved copy made under a lapsed claim")?;
	let clock = ctx.env().clock();
	loop {
		let read = read_actor(ctx, volume).await?;
		match abandon_progress(read.as_ref(), copy_unix_ms, mover) {
			AbandonProgress::Done => return Ok(()),
			AbandonProgress::Waiting => {}
			AbandonProgress::Refused => {
				bail!("a moved copy refused its abandon, so a later backfill page owns it")
			}
		}
		if clock.observe()?.boot_ms >= deadline_boot_ms {
			bail!(AbandonPending);
		}
		rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
	}
}

/// Where the abandon of a moved copy stands, as a read of the copy after the abandon shows it.
#[derive(Debug, PartialEq, Eq)]
enum AbandonProgress {
	/// The copy is gone, ended, or replaced by another incarnation.
	Done,
	/// The copy accepted the abandon and is destroying, or it is still held while a database
	/// fork that already sent work runs to its end, after which its pending destroy runs.
	Waiting,
	/// The copy refused the abandon: a later page holds it, or it left its hold, so its move was
	/// confirmed.
	Refused,
}

/// Legacy: where the abandon of the copy of incarnation `copy_unix_ms` by `mover` stands, from
/// a read of the copy after the abandon was sent. The copy accepts an abandon of its own
/// incarnation from a mover at least its hold's, and its destroy then waits for a database fork
/// in flight, during which it still reads as held; the read evaluates the same rule, so an
/// accepted abandon is waited for rather than taken as a refusal. Pure, like `plan_move`.
/// Removed in 3.1 with the backfill.
fn abandon_progress(
	read: Option<&wire::ActorReadResponse>,
	copy_unix_ms: i64,
	mover: u64,
) -> AbandonProgress {
	let Some(read) = read else {
		return AbandonProgress::Done;
	};
	if read
		.data
		.as_ref()
		.is_none_or(|data| data.create_unix_ms != copy_unix_ms)
	{
		return AbandonProgress::Done;
	}
	match &read.phase {
		None | Some(wire::ActorPhase::Destroyed | wire::ActorPhase::DelegatedToV2) => {
			AbandonProgress::Done
		}
		Some(wire::ActorPhase::Destroying(_)) => AbandonProgress::Waiting,
		Some(wire::ActorPhase::Initializing(_)) => match hold(read) {
			Some(held) if held.mover <= mover => AbandonProgress::Waiting,
			Some(_) | None => AbandonProgress::Refused,
		},
		Some(
			wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_)
			| wire::ActorPhase::MigratingFromV2(_),
		) => AbandonProgress::Refused,
	}
}

/// Legacy: whether a move failed because its copy accepted an abandon that had not ended by the
/// page's deadline, so its undo keeps the mark; see [`AbandonPending`]. Removed in 3.1 with the
/// backfill.
fn abandon_pending(error: &anyhow::Error) -> bool {
	error.downcast_ref::<AbandonPending>().is_some()
}

/// What a claim of the old database found.
enum Claim {
	/// The database is claimed as its writer.
	Claimed(depot_client::filesystem::OpenedFilesystem),
	/// The old actor never created its database, so there is none to copy.
	NeverCreated,
	/// The old actor ended, which deleted its database: a request destroyed it during the move.
	OldEnded,
}

/// Claims the old database `database` of the legacy actor at `legacy_volume` as its writer. An
/// open from another process claims at a new claim epoch, which fences the holder. Depot refuses
/// an open while this same process holds the volume (`InUse`), as a node connection on this
/// worker does for a running actor, and offers no way to fence a local holder; then the old
/// actor is asked to sleep, which closes its database, and the open is tried again until the
/// page's deadline, backing off from `MOVE_POLL_INTERVAL` to a fraction of the page timeout.
/// The sleep, a durable write, is sent again only while the actor reads as starting or running:
/// the actor is marked moving, so no wake restarts it, and a sleep that raced a start still
/// reaches it.
///
/// A database Depot reports missing or deleted is read against the old actor, whose destroy is
/// the only thing that deletes it: an ended old actor answers `OldEnded`. A live one answers
/// `NeverCreated` only when the writer's open reports a volume never created or the actor never
/// sent a start command; any other missing database is an error, never an empty copy.
async fn claim_database(
	ctx: &EngineContext,
	legacy_volume: &VolumeKey,
	database: &VolumeKey,
	deadline_boot_ms: u64,
) -> Result<Claim> {
	let depot = ctx.depot();
	let clock = ctx.env().clock();
	let page_timeout_ms = page_timeout_ms(ctx.config().pegboard());
	let max_retry_interval = Duration::from_millis(page_timeout_ms / 16)
		.clamp(MOVE_POLL_INTERVAL, CLAIM_RETRY_MAX_INTERVAL);
	let mut retry_interval = MOVE_POLL_INTERVAL;
	loop {
		let error = match depot.open_filesystem(database.clone()).await {
			Ok(claim) => return Ok(Claim::Claimed(claim)),
			Err(error) => error,
		};
		let gone = crate::actor_machine::executor::source_gone(&error);
		let not_created = never_created(&error);
		if gone || not_created {
			let old = read_actor(ctx, legacy_volume)
				.await?
				.context("a legacy actor's state volume is missing during its move")?;
			if ended(old.phase.as_ref()) {
				return Ok(Claim::OldEnded);
			}
			if not_created || old.command_index == Some(0) {
				return Ok(Claim::NeverCreated);
			}
			return Err(error.context("a live legacy actor's database is missing"));
		}
		let in_use = error.chain().any(|cause| {
			matches!(
				cause.downcast_ref::<depot_client::volumes::Error>(),
				Some(depot_client::volumes::Error::InUse)
			)
		});
		if !in_use {
			return Err(error.context("claim a legacy actor's database"));
		}
		let old = read_actor(ctx, legacy_volume)
			.await?
			.context("a legacy actor's state volume is missing during its move")?;
		let up = match old.phase.as_ref() {
			Some(wire::ActorPhase::Starting(_) | wire::ActorPhase::Running(_)) => true,
			Some(
				wire::ActorPhase::Initializing(_)
				| wire::ActorPhase::Stopping(_)
				| wire::ActorPhase::Sleeping(_)
				| wire::ActorPhase::Destroying(_)
				| wire::ActorPhase::Destroyed
				| wire::ActorPhase::MigratingFromV2(_)
				| wire::ActorPhase::DelegatedToV2,
			)
			| None => false,
		};
		if up {
			send(ctx, legacy_volume, wire::ActorEvent::Sleep)
				.await
				.context("ask a legacy actor held by this process to sleep")?;
		}
		if clock.observe()?.boot_ms >= deadline_boot_ms {
			bail!(
				"a legacy actor's database stayed held by this process through the backfill page"
			);
		}
		rivet_env::sleep(clock.as_ref(), retry_interval).await?;
		retry_interval = retry_interval.saturating_mul(2).min(max_retry_interval);
	}
}

/// Whether a writer's open failed because the volume was never created.
fn never_created(error: &anyhow::Error) -> bool {
	error.chain().any(|cause| {
		cause
			.downcast_ref::<depot_client::volumes::Error>()
			.is_some_and(depot_client::volumes::Error::is_write_volume_not_found)
	})
}

/// What creating the moved copy came to.
enum Created {
	/// The copy is held with its key reserved and its database copied.
	Ready(wire::ActorReadResponse),
	/// The old actor ended after the claim: a request destroyed it.
	OldEnded,
	/// A live grouped actor not made by the move holds the new id.
	Duplicate,
	/// A request destroyed the copy while it was held, which destroyed the actor.
	DestroyedByRequest,
}

/// Creates the moved copy at `new_volume` under the claim: the old actor is read again first,
/// so a request's destroy that landed before the claim ends the move, and its alarm, or the alarm
/// that came due while it was marked, is carried. Then waits for the copy to be ready.
#[allow(clippy::too_many_arguments, reason = "one move's inputs")]
async fn create_copy(
	ctx: &EngineContext,
	data: &wire::ActorData,
	legacy_id: Id,
	new_id: Id,
	new_volume: &VolumeKey,
	database: &VolumeKey,
	claimed: bool,
	mover: u64,
	deadline_boot_ms: u64,
	progress: &mut MoveProgress,
) -> Result<Created> {
	let depot = ctx.depot();
	let legacy_volume = LegacyActorVolume::new(legacy_id).volume_key();
	let old = read_actor(ctx, &legacy_volume)
		.await?
		.context("a legacy actor's state volume is missing during its move")?;
	if ended(old.phase.as_ref()) {
		return Ok(Created::OldEnded);
	}
	if mark_mover(&old) != Some(mover) {
		bail!("a later backfill page took this legacy actor's moving mark");
	}
	// The alarm read answers the armed alarm and one that came due while the actor was marked,
	// which the mark keeps instead of arming, from one snapshot, so an alarm firing meanwhile is
	// not lost. A storage worker from before that answers the armed alarm alone, so the deferred
	// alarm the mark held when it was read above is folded in too.
	let alarm_read_unix_ms = depot
		.command::<LegacyActorAlarmRead>(&legacy_volume, ())
		.await?
		.alarm_unix_ms;
	let read_deferred_unix_ms = match &old.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => moving.deferred_alarm_unix_ms,
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => None,
	};
	let alarm_unix_ms = match (read_deferred_unix_ms, alarm_read_unix_ms) {
		(Some(deferred), Some(read)) => Some(deferred.min(read)),
		(deferred, read) => deferred.or(read),
	};
	let source = match claimed {
		true => Some(wire::ActorForkSource {
			fork_id: ctx
				.env()
				.rng("pegboard.backfill.fork")
				.uuid_v4()
				.as_bytes()
				.to_vec()
				.into(),
			actor_id: legacy_id.as_bytes().into(),
			key: data.key.clone(),
			database_volume: database.clone(),
			// A move's database copy is not recorded as a fork, so it starts no chain.
			chain: Vec::new(),
		}),
		false => None,
	};
	depot
		.command::<ActorCreate>(
			new_volume,
			wire::ActorCreateRequest {
				init: wire::ActorCreateData {
					actor_id: new_id.as_bytes().into(),
					pool_name: data.pool_name.clone(),
					name: data.name.clone(),
					key: data.key.clone(),
					input: data.input.clone(),
					legacy_namespace: None,
					fork_from: None,
					legacy_move: Some(wire::LegacyActorMove {
						moved_from: legacy_id.as_bytes().into(),
						create_unix_ms: data.create_unix_ms,
						alarm_unix_ms,
						database: source,
						mover,
					}),
				},
				waiter: None,
				// The legacy actor already lives in this cluster, so its moved copy does too, even
				// while its namespace migrates in from V2.
				force_actor_v3: true,
			},
		)
		.await?;
	wait_ready(
		ctx,
		new_volume,
		legacy_id,
		mover,
		deadline_boot_ms,
		progress,
	)
	.await
}

/// Proves the claim on the old database still holds by committing one empty entry to a quorum
/// of its log. The log shards refuse an append at a stale claim epoch, so a newer claim fails the
/// proof. The proof stops at the move's deadline.
async fn check_claim(
	claim: &depot_client::filesystem::OpenedFilesystem,
	deadline_boot_ms: u64,
) -> Result<()> {
	claim
		.filesystem()
		.prove_tail(Some(deadline_boot_ms))
		.await?;
	Ok(())
}

/// Closes the claim on the old database, if one was taken. A failed close is harmless: the old
/// actor's destroy deletes the volume.
async fn release(claim: Option<depot_client::filesystem::OpenedFilesystem>) {
	let Some(claim) = claim else {
		return;
	};
	if let Err(error) = claim.shutdown().await {
		tracing::debug!(
			?error,
			"a backfill move's claim on an old database did not close cleanly"
		);
	}
}

/// Waits until the moved copy at `volume` is ready for its confirm: its key reserved and its
/// database copied, still held for `mover`. Records the copy's incarnation in `progress` once it
/// is this move's, so a failure abandons it. A fork that failed for good, or a copy destroyed by
/// anything but a request, is an error, so the page sends the move again, whose create starts
/// the actor's next incarnation.
async fn wait_ready(
	ctx: &EngineContext,
	volume: &VolumeKey,
	legacy_id: Id,
	mover: u64,
	deadline_boot_ms: u64,
	progress: &mut MoveProgress,
) -> Result<Created> {
	let clock = ctx.env().clock();
	loop {
		let read = read_actor(ctx, volume)
			.await?
			.context("a moved actor's state volume is missing after its create")?;
		if copy_destroyed_by_request(Some(&read), legacy_id) {
			return Ok(Created::DestroyedByRequest);
		}
		match holder(Some(&read), legacy_id) {
			Holder::Move => {}
			Holder::Other => return Ok(Created::Duplicate),
			Holder::Free => {
				let failed = matches!(
					read.database_origin,
					Some(wire::ActorDatabaseOrigin::DatabaseForkFailed(_))
				);
				if failed {
					bail!("a moved actor's database fork failed for good");
				}
				bail!("a moved actor was destroyed before its move finished");
			}
		}
		let Some(held) = hold(&read) else {
			bail!("a moved copy left its hold before this move confirmed it");
		};
		if held.mover != mover {
			bail!("a moved copy held for another backfill page took the new id");
		}
		progress.copy_unix_ms = Some(incarnation(&read)?);
		let Some(wire::ActorPhase::Initializing(init)) = &read.phase else {
			bail!("a held moved copy is not initializing");
		};
		let reserved = matches!(
			init.reserve_epoxy.state.phase,
			wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved
		);
		let copied = match &init.fork_database {
			None => true,
			Some(fork) => matches!(
				fork.state.phase,
				wire::ForkDatabasePhase::ForkDatabasePhaseForked
			),
		};
		if reserved && copied {
			return Ok(Created::Ready(read));
		}
		if clock.observe()?.boot_ms >= deadline_boot_ms {
			bail!("a moved actor's initialization did not finish within the backfill page");
		}
		rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
	}
}

/// Destroys the legacy actor of `data` instead of moving it, because a live grouped actor already
/// holds its new id: one made in the moment between a create's namespace re-read and the
/// namespace's move, or by a create that found the legacy actor gone mid-move. The grouped actor
/// is kept. The destroy is fenced by the move's mark. Counted and logged without the actor's key.
async fn destroy_duplicate(
	ctx: &EngineContext,
	namespace: &str,
	data: &wire::ActorData,
	legacy_volume: &VolumeKey,
	mover: u64,
) -> Result<MoveResult> {
	tracing::warn!(
		namespace,
		actor_name = %data.name,
		"a legacy actor's new id is held by a live grouped actor; the legacy duplicate is destroyed and the grouped actor kept"
	);
	match destroy_old(ctx, legacy_volume, mover).await? {
		OldDestroy::Destroyed { .. } | OldDestroy::Stopping => {
			crate::metrics::BACKFILL_DUPLICATES
				.with_label_values(&[namespace])
				.inc();
			Ok(MoveResult::Duplicate)
		}
		OldDestroy::EndedByRequest => Ok(MoveResult::Skipped),
		OldDestroy::Refused => bail!(OLD_DESTROY_REFUSED),
		OldDestroy::NotAdmitted(error) => Err(error),
	}
}

/// What a give-up came to.
#[derive(Debug, PartialEq, Eq)]
enum GiveUp {
	/// The actor stays in the legacy shape, serving wakes, and is listed as skipped with `key`,
	/// its key as its record holds it.
	Skipped { key: Option<String> },
	/// The actor needed no give-up: its move had already been decided, as `result` says, for
	/// example a destroy that committed before the confirm failed, which this give-up confirmed.
	Resolved(MoveResult),
}

/// What a give-up does with what `plan_move` decided, so a give-up never abandons a copy whose
/// old actor its move already destroyed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GiveUpPlan {
	/// Nothing to give up: the decided result stands.
	Done(MoveResult),
	/// A later page holds the mark; this page fails.
	Superseded,
	/// The old actor was destroyed for the held copy: confirm it, retrying, never abandon it.
	Confirm {
		copy_unix_ms: i64,
		old_epoch: u32,
		mover: u64,
	},
	/// The old actor was destroyed for the held copy and is still stopping: wait until it ended,
	/// then confirm the copy as `Confirm` does, never abandon or skip it.
	AwaitOldStop { copy_unix_ms: i64, mover: u64 },
	/// A request destroyed the held copy, so the old actor is destroyed too.
	DestroyOld,
	/// The old actor ended without a move for the held copy: abandon the copy, nothing is skipped.
	AbandonAndResolve { copy_unix_ms: i64 },
	/// The old actor is live and was not moved: abandon the held copy, if any, clear the mark and
	/// skip the actor.
	Skip { abandon_unix_ms: Option<i64> },
}

/// Decides a give-up from `plan_move`: only `AbandonAndMove` and `AbandonAndSkip` abandon a
/// copy, and `Confirm` and `AwaitOldStop` confirm it.
pub(crate) fn plan_give_up(plan: MovePlan) -> GiveUpPlan {
	match plan {
		MovePlan::Done(result) => GiveUpPlan::Done(result),
		MovePlan::Superseded => GiveUpPlan::Superseded,
		MovePlan::Confirm {
			copy_unix_ms,
			old_epoch,
			mover,
		} => GiveUpPlan::Confirm {
			copy_unix_ms,
			old_epoch,
			mover,
		},
		MovePlan::AwaitOldStop {
			copy_unix_ms,
			mover,
		} => GiveUpPlan::AwaitOldStop {
			copy_unix_ms,
			mover,
		},
		MovePlan::AbandonAndSkip { copy_unix_ms } => GiveUpPlan::AbandonAndResolve { copy_unix_ms },
		MovePlan::DestroyOld => GiveUpPlan::DestroyOld,
		MovePlan::AbandonAndMove { copy_unix_ms } => GiveUpPlan::Skip {
			abandon_unix_ms: Some(copy_unix_ms),
		},
		MovePlan::Duplicate | MovePlan::Move => GiveUpPlan::Skip {
			abandon_unix_ms: None,
		},
	}
}

/// Legacy: gives up on the legacy actor `legacy_id` after the last allowed attempt failed to
/// move it, deciding from the actors as they are now, the way a resumed move does: the old actor
/// is marked again under this page's `mover`, so no earlier page can destroy it meanwhile, and
/// read back, and the actor at its new id is read. A copy whose old actor this move already
/// destroyed is confirmed, retrying until `deadline_boot_ms`, and never abandoned, since it holds
/// the only data left; its failure fails the page, whose next send gives up again. A live, unmoved
/// old actor's held copy is abandoned and waited for, so nothing blocks its key, then the mark is
/// cleared, so the actor serves wakes in the legacy shape again and is skipped. A copy that cannot
/// be abandoned yet keeps the mark and fails the page. A give-up that finds the move decided
/// deletes the old actor's kept index entry as a move does. Removed in 3.1 with the backfill.
#[allow(clippy::too_many_arguments, reason = "one give-up's inputs")]
async fn give_up_actor(
	ctx: &EngineContext,
	namespace: &str,
	group: GroupId,
	legacy_id: Id,
	index_bucket_count: u64,
	mover: u64,
	deadline_boot_ms: u64,
) -> Result<GiveUp> {
	let legacy_volume = LegacyActorVolume::new(legacy_id).volume_key();
	let Some(old) = read_actor(ctx, &legacy_volume).await? else {
		return Ok(GiveUp::Resolved(MoveResult::Skipped));
	};
	let Some(data) = &old.data else {
		return Ok(GiveUp::Resolved(MoveResult::Skipped));
	};
	let new_id = match &data.key {
		Some(key) => crate::actor_key::keyed_actor_id(legacy_id.label(), &data.name, key),
		None => legacy_id,
	};
	let new_volume = ActorVolume::new(group, new_id).volume_key();

	if !ended(old.phase.as_ref()) {
		let mark = wire::LegacyMarkMoving {
			mover,
			for_ms: page_timeout_ms(ctx.config().pegboard()),
		};
		send(
			ctx,
			&legacy_volume,
			wire::ActorEvent::LegacyMarkMoving(mark),
		)
		.await
		.context("mark a legacy actor as moving before giving up on it")?;
	}
	let old = read_actor(ctx, &legacy_volume)
		.await?
		.context("a legacy actor's state volume is missing after it was read")?;
	let old_entry = OldEntry {
		name: data.name.to_string(),
		create_unix_ms: data.create_unix_ms,
		moved: old_entry_left_to_page(&old),
	};
	let existing = read_actor(ctx, &new_volume).await?;

	let gave_up = match plan_give_up(plan_move(&old, existing.as_ref(), legacy_id, mover)?) {
		GiveUpPlan::Done(result) => GiveUp::Resolved(result),
		GiveUpPlan::Superseded => {
			bail!("a later backfill page holds this legacy actor's moving mark")
		}
		GiveUpPlan::Confirm {
			copy_unix_ms,
			old_epoch,
			mover: copy_mover,
		} => GiveUp::Resolved(
			confirm_retrying(
				ctx,
				&new_volume,
				copy_unix_ms,
				old_epoch,
				copy_mover,
				deadline_boot_ms,
			)
			.await?,
		),
		GiveUpPlan::AwaitOldStop {
			copy_unix_ms,
			mover: copy_mover,
		} => {
			let old_epoch =
				wait_old_ended(ctx, &legacy_volume, copy_mover, deadline_boot_ms).await?;
			GiveUp::Resolved(
				confirm_retrying(
					ctx,
					&new_volume,
					copy_unix_ms,
					old_epoch,
					copy_mover,
					deadline_boot_ms,
				)
				.await?,
			)
		}
		GiveUpPlan::DestroyOld => {
			GiveUp::Resolved(destroy_for_request(ctx, &legacy_volume, mover).await?)
		}
		GiveUpPlan::AbandonAndResolve { copy_unix_ms } => {
			abandon_copy(ctx, &new_volume, copy_unix_ms, mover, deadline_boot_ms).await?;
			GiveUp::Resolved(MoveResult::Skipped)
		}
		GiveUpPlan::Skip { abandon_unix_ms } => {
			if let Some(copy_unix_ms) = abandon_unix_ms {
				abandon_copy(ctx, &new_volume, copy_unix_ms, mover, deadline_boot_ms).await?;
			}
			send(
				ctx,
				&legacy_volume,
				wire::ActorEvent::LegacyClearMoving(wire::LegacyClearMoving { mover }),
			)
			.await
			.context("clear a skipped legacy actor's moving mark")?;
			GiveUp::Skipped {
				key: data.key.as_ref().map(ToString::to_string),
			}
		}
	};
	if let GiveUp::Resolved(result) = &gave_up {
		settle_old_entry(
			ctx,
			namespace,
			legacy_id,
			index_bucket_count,
			Some(old_entry),
			result,
		)
		.await?;
	}
	Ok(gave_up)
}

/// Confirms a held copy whose old actor its move destroyed and ended, retrying until
/// `deadline_boot_ms`, for a give-up, which must never abandon such a copy. A failure past the
/// deadline fails the page, whose next send confirms it.
async fn confirm_retrying(
	ctx: &EngineContext,
	new_volume: &VolumeKey,
	copy_unix_ms: i64,
	old_epoch: u32,
	mover: u64,
	deadline_boot_ms: u64,
) -> Result<MoveResult> {
	let clock = ctx.env().clock();
	loop {
		let confirmed = confirm_move(ctx, new_volume, copy_unix_ms, old_epoch, mover).await;
		match confirmed {
			Ok(result) => return Ok(result),
			Err(error) if clock.observe()?.boot_ms >= deadline_boot_ms => {
				return Err(error.context(
					"a moved copy whose old actor is destroyed was not confirmed; the next send confirms it",
				));
			}
			Err(error) => {
				tracing::debug!(
					?error,
					"a give-up's confirm of a moved copy failed; it is retried"
				);
				rivet_env::sleep(clock.as_ref(), MOVE_POLL_INTERVAL).await?;
			}
		}
	}
}
