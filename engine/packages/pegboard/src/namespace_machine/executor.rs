//! The namespace state machine's executor: runs a root's group create, a fork's group writes,
//! the group archive and the listing index write after a transition commits, and reports each outcome
//! back to the namespace's volume as a step. It also sends the namespace's state to waiting
//! creates.

use super::{BackfillPhase, Effect, Event, NamespaceStateMachine, Policy, Preparation, State};
use crate::catalog::{NamespaceIndexPut, NamespaceStep};
use crate::keys::{
	partition::{NamespaceCatalogVolume, NamespaceVolume},
	ups::NamespaceStartNoticeSubject,
};
use anyhow::{Context, Result};
use depot_client::groups::GroupFork;
use rivet_data::generated::pegboard_namespace_v1 as ns;
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{KeyAddress, codec::GroupId};
use rivet_state_machine::EffectContext;
use rivet_state_machine_durable::RuntimeEvent;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use vbare::Versioned;

#[derive(Clone)]
pub struct Executor {
	ctx: EngineContext,
	/// The namespace whose volume this executor serves.
	name: String,
	/// Legacy: the opening of the namespace's volume this executor runs for, which owns the
	/// backfill page tasks. Absent for an executor that only runs a group create outside any
	/// command. Removed in 3.1 with the backfill.
	opening: Option<LegacyPageOwner>,
}

/// Legacy: the backfill page tasks of one opening of a namespace's volume. Kept in the opening's
/// shared state and nowhere else: a page task holds no reference to it, so when the opening goes,
/// which is when this process loses the volume, the scope drops and cancels every page task in
/// it. Losing the right to serve while the opening stays stops a page at its next ownership
/// check, before its next move or bucket delete, and the actors fence its writes by mover token.
/// At most one page runs: a re-send of the page aborts the previous one. Removed in 3.1 with the
/// backfill.
#[derive(Default)]
pub(crate) struct LegacyBackfillPages {
	scope: std::sync::OnceLock<rivet_env::TaskScope>,
	/// Forced sync: set from the executor's sync `execute`; no async code waits on it.
	current: parking_lot::Mutex<Option<rivet_env::TaskHandle>>,
	/// The namespace whose `pegboard_backfill_skipped_listed` series this opening set, removed
	/// when the opening goes.
	listed_series: std::sync::OnceLock<String>,
}

impl Drop for LegacyBackfillPages {
	fn drop(&mut self) {
		if let Some(namespace) = self.listed_series.get() {
			// A missing series is already gone; nothing else can fail here.
			let _ = crate::metrics::BACKFILL_SKIPPED_LISTED.remove_label_values(&[namespace]);
		}
	}
}

/// Legacy: the page tasks of an opening and the opening's weak identity, which a page checks
/// before each move. Only the executor holds it, never a task it spawns. Removed in 3.1 with the
/// backfill.
#[derive(Clone)]
struct LegacyPageOwner {
	pages: std::sync::Arc<LegacyBackfillPages>,
	owner: depot_worker::command::KvOwner,
}

impl LegacyBackfillPages {
	/// Legacy: records that this opening set the skipped list gauge of `namespace`, whose series
	/// it removes when it goes. Removed in 3.1 with the backfill.
	fn legacy_track_listed(&self, namespace: &str) {
		self.listed_series.get_or_init(|| namespace.to_owned());
	}

	/// Legacy: starts a page task through `spawn` in this opening's scope, and aborts the page it
	/// outdates, which this opening may still run, so that page stops rather than walk on beside
	/// the new one. The task must not hold this value, or the scope would never drop. Removed in
	/// 3.1 with the backfill.
	pub(crate) fn legacy_start(
		&self,
		env: &std::sync::Arc<dyn rivet_env::Env>,
		spawn: impl FnOnce(&rivet_env::TaskScope) -> rivet_env::TaskHandle,
	) {
		let scope = self
			.scope
			.get_or_init(|| rivet_env::TaskScope::new(env.clone()));
		let task = spawn(scope);
		if let Some(previous) = self.current.lock().replace(task) {
			previous.abort();
		}
	}
}

impl Executor {
	/// An executor for a group create run outside any command, which starts no backfill page.
	pub fn new(ctx: &EngineContext, name: String) -> Self {
		Executor {
			ctx: ctx.clone(),
			name,
			opening: None,
		}
	}

	/// An executor for the events a command of the namespace's volume `db` runs.
	pub(crate) async fn for_volume(
		ctx: &EngineContext,
		name: String,
		db: &depot_worker::command::KvHandle,
	) -> Result<Self> {
		let pages = db.opening_state::<LegacyBackfillPages>().await?;
		Ok(Executor {
			ctx: ctx.clone(),
			name,
			opening: Some(LegacyPageOwner {
				pages,
				owner: db.owner(),
			}),
		})
	}

	/// Legacy: sets the skipped list gauge from `state` the first time this executor's opening
	/// sees it, so a namespace whose volume opened in this process after its backfill finished
	/// reports its list, which no transition would set again. Removed in 3.1 with the backfill.
	fn legacy_observe_listed_once(&self, state: &State) {
		let Some(opening) = &self.opening else {
			return;
		};
		legacy_observe_listed_at_open(&self.name, &opening.pages, state.0.as_ref());
	}

	/// The policy and limits a transition applies, from configuration.
	fn policy(&self) -> Policy {
		let pegboard = self.ctx.config().pegboard();
		Policy {
			fork_retry_ms: pegboard.namespace_fork_retry_ms(),
			retry_max_ms: pegboard.namespace_job_retry_max_ms(),
			start_waiters_capacity: pegboard.namespace_start_waiters_capacity() as usize,
			backfill_start_delay_ms: pegboard.backfill.start_delay_ms(),
			backfill_page_timeout_ms: crate::legacy_backfill::page_timeout_ms(pegboard),
			backfill_max_actor_attempts: pegboard.backfill.max_actor_attempts(),
			backfill_max_skipped: pegboard.backfill.max_skipped(),
		}
	}

	/// Creates the namespace's root group record and reports the outcome to its machine, as the
	/// `CreateGroup` effect does. The record create is idempotent and the machine ignores an
	/// outcome outside `Creating`, so running it beside the effect is harmless.
	pub(crate) async fn run_group_create(&self, group: GroupId) {
		let outcome = create_group(&self.ctx, &self.name, group).await;
		let result = match &outcome {
			ns::NamespaceCreateOutcome::NamespaceCreateOutcomeOk => "ok",
			ns::NamespaceCreateOutcome::NamespaceOutcomeUnknown(_) => "unknown",
		};
		crate::metrics::NAMESPACE_GROUP_CREATES
			.with_label_values(&[result])
			.inc();
		self.step(ns::NamespaceStepRequest::NamespaceStepCreate(
			ns::NamespaceStepCreate { outcome },
		))
		.await;
	}

	/// Legacy: runs one backfill page and reports it to the namespace's machine with the cursor,
	/// attempt and mover it was sent with. The page is bounded by the page timeout end to end, after
	/// which `BackfillRetry` sends it again. A page that failed before moving anything is sent
	/// again by its `BackfillRetry`; one that failed at its first actor reports that actor in
	/// `BackfillPageFailed`, and any other failure reports nothing. Removed in 3.1 with the backfill.
	async fn legacy_run_page(
		&self,
		page: crate::legacy_backfill::PageRequest,
		owner: depot_worker::command::KvOwner,
	) {
		let clock = self.ctx.env().clock();
		let timeout = Duration::from_millis(self.policy().backfill_page_timeout_ms);
		let cursor = page.cursor.clone();
		let attempt = page.attempt;
		let mover = page.mover;
		let still_owner = move || owner.may_serve();
		let ran = rivet_env::timeout(
			clock.as_ref(),
			timeout,
			crate::legacy_backfill::legacy_move_page(&self.ctx, &self.name, page, &still_owner),
		)
		.await;
		let result = match ran {
			Ok(result) => result,
			Err(error) => {
				Err(anyhow::Error::from(error).context("a backfill page ran past its timeout"))
			}
		};
		let page = match result {
			Ok(page) => page,
			Err(error) => {
				crate::metrics::BACKFILL_PAGES
					.with_label_values(&[self.name.as_str(), "failed"])
					.inc();
				tracing::debug!(?error, namespace = %self.name, "a backfill page failed before moving anything; its retry timer sends it again");
				// A failure at the page's first actor names it, so the machine can tell the
				// operator which actor the walk waits on once it can no longer give up.
				if let Some(failed_at) =
					error.downcast_ref::<crate::legacy_backfill::LegacyPageFailedAt>()
				{
					self.step(ns::NamespaceStepRequest::NamespaceStepBackfillPageFailed(
						ns::NamespaceStepBackfillPageFailed {
							failed: ns::NamespaceBackfillPageFailed {
								from: cursor,
								attempt,
								mover,
								name: failed_at.name.clone().into(),
								actor_id: failed_at.actor_id.clone().into(),
							},
						},
					))
					.await;
				}
				return;
			}
		};
		let result = match &page.next {
			ns::NamespaceBackfillNext::NamespaceBackfillNextPage(_) => "next",
			ns::NamespaceBackfillNext::NamespaceBackfillFinished => "finished",
		};
		crate::metrics::BACKFILL_PAGES
			.with_label_values(&[self.name.as_str(), result])
			.inc();
		self.step(ns::NamespaceStepRequest::NamespaceStepBackfill(
			ns::NamespaceStepBackfill {
				outcome: ns::NamespaceBackfillOutcome {
					from: cursor,
					attempt,
					mover,
					moved: page.moved,
					skipped: page.skipped,
					next: page.next,
				},
			},
		))
		.await;
	}

	/// Legacy: reads whether the V2 migrator holds an in-flight entry at the legacy placement
	/// that may target this namespace, and reports it. A failed read reports nothing; the pending
	/// backfill's `BackfillRetry` reads again. Removed in 3.1 with the backfill.
	async fn legacy_read_migrator_holds(&self) {
		// A read refused for now is read again at the refusal's hint, doubled for each refusal,
		// within one start delay: the backfill's retry timer, armed with this read, reads again
		// after that anyway, but a start delay is minutes.
		let clock = self.ctx.env().clock();
		let deadline_boot_ms = clock
			.observe()
			.map(|now| {
				now.boot_ms
					.saturating_add(self.policy().backfill_start_delay_ms)
			})
			.ok();
		let mut refusals = 0u32;
		loop {
			let error = match crate::migrate_from_v2::legacy_migrator_holds_legacy_entries(
				&self.ctx, &self.name,
			)
			.await
			{
				Ok(held) => {
					if held {
						tracing::info!(
							namespace = %self.name,
							"the backfill waits for the V2 migrator's in-flight migrations at the legacy placement to finish"
						);
					}
					self.step(ns::NamespaceStepRequest::NamespaceStepLegacyMigratorHolds(
						ns::NamespaceStepLegacyMigratorHolds { held },
					))
					.await;
					return;
				}
				Err(error) => error,
			};
			let read_again_at = match (deadline_boot_ms, clock.observe()) {
				(Some(deadline_boot_ms), Ok(now)) => migrator_read_again_at(
					depot_client::retry_after_ms(&error),
					refusals,
					now.boot_ms,
					deadline_boot_ms,
				),
				(None, _) | (_, Err(_)) => None,
			};
			let Some(read_again_at) = read_again_at else {
				tracing::warn!(
					?error,
					namespace = %self.name,
					"reading the V2 migrator's state for the backfill failed; the backfill's retry timer reads it again"
				);
				return;
			};
			tracing::debug!(
				?error,
				namespace = %self.name,
				"reading the V2 migrator's state for the backfill was refused for now; it is read again at the refusal's hint"
			);
			refusals = refusals.saturating_add(1);
			if clock.sleep_until(read_again_at).await.is_err() {
				return;
			}
		}
	}

	/// Reports an effect's outcome to the namespace's machine. A lost report costs one timer
	/// firing, which for a backfill page is a whole page timeout.
	///
	/// A report Depot refused at admission with a retry hint never reached the machine, so the
	/// same report is sent again at the hint while that stays within one backfill page timeout,
	/// the longest wait before the machine's own retry timer re-drives an effect. Any other
	/// failure leaves the effect to that timer.
	async fn step(&self, request: ns::NamespaceStepRequest) {
		let clock = self.ctx.env().clock();
		let deadline_boot_ms = clock
			.observe()
			.map(|now| {
				now.boot_ms
					.saturating_add(self.policy().backfill_page_timeout_ms)
			})
			.ok();
		loop {
			let result = self
				.ctx
				.depot()
				.command::<NamespaceStep>(
					&NamespaceVolume::new(self.name.clone()).volume_key(),
					request.clone(),
				)
				.await;
			let Err(error) = result else {
				return;
			};
			let resend_at = match (deadline_boot_ms, clock.observe()) {
				(Some(deadline_boot_ms), Ok(now)) => outcome_resend_at(
					depot_client::retry_after_ms(&error),
					now.boot_ms,
					deadline_boot_ms,
				),
				(None, _) | (_, Err(_)) => None,
			};
			let Some(resend_at) = resend_at else {
				tracing::warn!(?error, namespace = %self.name, "a namespace effect's outcome was not recorded; its retry timer re-drives the effect");
				return;
			};
			tracing::debug!(?error, namespace = %self.name, "a namespace effect's outcome was refused at admission; it is sent again at the refusal's hint");
			if clock.sleep_until(resend_at).await.is_err() {
				return;
			}
		}
	}
}

/// The most times a refused read of the V2 migrator's state doubles its refusal's hint.
const MIGRATOR_READ_BACKOFF_DOUBLINGS: u32 = 6;

/// Legacy: when the V2 migrator's state is read again after the read's `refusals`th refusal for
/// now, which carried the hint `retry_after_ms`: at the hint, doubled for each earlier refusal up
/// to [`MIGRATOR_READ_BACKOFF_DOUBLINGS`] times, when that falls before `deadline_boot_ms`.
/// `None` leaves the read to the backfill's retry timer. Removed in 3.1 with the backfill.
pub(super) fn migrator_read_again_at(
	retry_after_ms: Option<u64>,
	refusals: u32,
	now_boot_ms: u64,
	deadline_boot_ms: u64,
) -> Option<u64> {
	let backoff_ms =
		retry_after_ms?.saturating_mul(1u64 << refusals.min(MIGRATOR_READ_BACKOFF_DOUBLINGS));
	outcome_resend_at(Some(backoff_ms), now_boot_ms, deadline_boot_ms)
}

/// When a namespace effect's outcome report that Depot refused is sent again: at the refusal's
/// hint `retry_after_ms`, when the refusal carried one and the hint falls before
/// `deadline_boot_ms`. `None` leaves the effect to the machine's own retry timer.
pub(super) fn outcome_resend_at(
	retry_after_ms: Option<u64>,
	now_boot_ms: u64,
	deadline_boot_ms: u64,
) -> Option<u64> {
	let resend_at = now_boot_ms.saturating_add(retry_after_ms?);
	if resend_at >= deadline_boot_ms {
		return None;
	}
	Some(resend_at)
}

impl rivet_state_machine_durable::Executor<NamespaceStateMachine> for Executor {
	fn prepare<'a>(
		&'a self,
		state: &'a State,
		_event: &'a RuntimeEvent<Event, ns::NamespaceTimer>,
	) -> Pin<Box<dyn Future<Output = Result<Preparation>> + Send + 'a>> {
		Box::pin(async move {
			self.legacy_observe_listed_once(state);
			let config = self.ctx.config();
			Ok(Preparation {
				policy: self.policy(),
				event_shards: config.depot().events.shards(),
				index_bucket_count: config.pegboard().actor_index_buckets_per_name(),
				backfill_fleet_ready: matches!(
					crate::catalog::namespace_volumes_open(&self.ctx),
					Ok(true)
				),
			})
		})
	}

	fn execute(&self, effect: Effect, ctx: EffectContext<()>) {
		// Spawned tasks get an executor without the opening, so none of them keeps the opening's
		// backfill pages, and with them their own scope, alive.
		let self2 = Executor::new(&self.ctx, self.name.clone());
		match effect {
			Effect::CreateGroup { group, attempt } => {
				count_retry("create", attempt);
				ctx.spawn(Box::pin(async move {
					self2.run_group_create(group).await;
				}));
			}
			Effect::ForkGroup {
				parent,
				child,
				operation_id,
				unix_ms,
				attempt,
			} => {
				count_retry("fork", attempt);
				ctx.spawn(Box::pin(async move {
					let outcome = fork_group(
						&self2.ctx,
						parent,
						&self2.name,
						child,
						operation_id,
						unix_ms,
					)
					.await;
					let result = match &outcome {
						ns::NamespaceForkOutcome::NamespaceForkOutcomeOk(_) => "ok",
						ns::NamespaceForkOutcome::NamespaceOutcomeRefused(_) => "refused",
						ns::NamespaceForkOutcome::NamespaceOutcomeUnknown(_) => "unknown",
					};
					crate::metrics::NAMESPACE_FORKS
						.with_label_values(&[result])
						.inc();
					self2
						.step(ns::NamespaceStepRequest::NamespaceStepFork(
							ns::NamespaceStepFork { outcome },
						))
						.await;
				}));
			}
			Effect::ArchiveGroup {
				group,
				shards,
				attempt,
			} => {
				count_retry("archive", attempt);
				ctx.spawn(Box::pin(async move {
					let outcome = archive_group(&self2.ctx, group, shards).await;
					let result = match &outcome {
						ns::NamespaceArchiveOutcome::NamespaceArchiveOutcomeOk => "ok",
						ns::NamespaceArchiveOutcome::NamespaceOutcomeRefused(_) => "refused",
						ns::NamespaceArchiveOutcome::NamespaceOutcomeUnknown(_) => "unknown",
					};
					crate::metrics::NAMESPACE_ARCHIVES
						.with_label_values(&[result])
						.inc();
					self2
						.step(ns::NamespaceStepRequest::NamespaceStepArchive(
							ns::NamespaceStepArchive { outcome },
						))
						.await;
				}));
			}
			Effect::WriteIndex {
				attempt,
				create_unix_ms,
			} => {
				count_retry("index", attempt);
				ctx.spawn(Box::pin(async move {
					let written = self2
						.ctx
						.depot()
						.command::<NamespaceIndexPut>(
							&NamespaceCatalogVolume::new().volume_key(),
							ns::NamespaceIndexPutRequest {
								name: self2.name.clone().into(),
								create_unix_ms,
							},
						)
						.await;
					if let Err(error) = &written {
						tracing::debug!(?error, namespace = %self2.name, "a namespace's listing index write failed; its retry timer writes it again");
					}
					self2
						.step(ns::NamespaceStepRequest::NamespaceStepIndex(
							ns::NamespaceStepIndex {
								outcome: ns::NamespaceIndexOutcome {
									written: written.is_ok(),
								},
							},
						))
						.await;
				}));
			}
			Effect::MovePage {
				group,
				index_bucket_count,
				cursor,
				attempt,
				skipped,
				give_up,
				mover,
			} => {
				count_retry("backfill", attempt);
				legacy_warn_backfill_retries(&self.name, attempt);
				legacy_observe_listed(&self.name, self.opening.as_ref(), count_u32(skipped.len()));
				let Some(opening) = self.opening.clone() else {
					tracing::error!(
						namespace = %self.name,
						"a backfill page was sent outside a command of the namespace's volume and does not run; its retry timer sends it again"
					);
					return;
				};
				let page = crate::legacy_backfill::PageRequest {
					group,
					index_bucket_count,
					cursor,
					attempt,
					skipped,
					give_up,
					mover,
				};
				let owner = opening.owner.clone();
				opening.pages.legacy_start(self.ctx.env(), |scope| {
					ctx.spawn_in(
						scope,
						Box::pin(async move {
							self2.legacy_run_page(page, owner).await;
						}),
					)
				});
			}
			Effect::ObserveBackfill { phase } => {
				let label = match phase {
					BackfillPhase::Pending => "pending",
					BackfillPhase::Running => "running",
					BackfillPhase::Done => "done",
				};
				crate::metrics::BACKFILL_PHASE_ENTRIES
					.with_label_values(&[label])
					.inc();
			}
			Effect::ObserveBackfillSkippedFull {
				actor_name,
				actor_id,
				listed,
				capacity,
			} => {
				observe_skipped_listed(&self.name, self.opening.as_ref(), listed, capacity);
				let actor_id = legacy_actor_id_label(&actor_id);
				tracing::error!(
					namespace = %self.name,
					%actor_name,
					%actor_id,
					listed,
					capacity,
					"the backfill carries its cap of skipped actors and refuses to give up on this actor; the walk waits on it and the namespace never finishes its backfill, so destroy or repair this actor, or destroy a skipped actor to make room"
				);
				crate::metrics::BACKFILL_GIVE_UPS_REFUSED
					.with_label_values(&[self.name.as_str()])
					.inc();
			}
			Effect::ObserveBackfillSkippedListed { listed } => {
				legacy_observe_listed(&self.name, self.opening.as_ref(), listed);
			}
			Effect::LegacyReadMigratorHolds => {
				ctx.spawn(Box::pin(async move {
					self2.legacy_read_migrator_holds().await;
				}));
			}
			Effect::ObserveBackfillSkipped {
				skipped,
				listed,
				capacity,
			} => {
				observe_skipped_listed(&self.name, self.opening.as_ref(), listed, capacity);
				if listed.saturating_mul(10) >= capacity.saturating_mul(9) {
					tracing::warn!(
						namespace = %self.name,
						listed,
						capacity,
						"the backfill's skipped actors are near their cap; at the cap it stops giving up and the backfill waits, so destroy the skipped actors to make room"
					);
				}
				let actor_id = legacy_actor_id_label(&skipped.actor_id);
				tracing::error!(
					namespace = %self.name,
					actor_name = %skipped.name,
					%actor_id,
					"the backfill gave up on an actor it could not move after its configured attempts; it stays in the legacy shape, reachable by its id and key, and the namespace refuses forks and archives until it is destroyed through the actor API; export and destroy it before 3.1 removes the backfill"
				);
				crate::metrics::BACKFILL_SKIPPED
					.with_label_values(&[self.name.as_str()])
					.inc();
			}
			Effect::NotifyWaiters { waiters, state } => {
				ctx.spawn(Box::pin(async move {
					let notice = ns::NamespaceStartNotice {
						name: self2.name.clone().into(),
						state,
					};
					let sends = waiters.into_iter().map(|waiter| {
						let ctx = self2.ctx.clone();
						let notice = notice.clone();
						async move {
							if let Err(error) = send_start_notice(&ctx, notice, &waiter).await {
								tracing::debug!(?error, "a namespace start notice was not delivered; the create reads the namespace at its deadline");
							}
						}
					});
					futures_util::future::join_all(sends).await;
				}));
			}
		}
	}
}

/// Counts a re-send of one of the machine's effects; the first send is not a retry.
fn count_retry(kind: &str, attempt: u32) {
	if attempt > 0 {
		crate::metrics::NAMESPACE_RETRIES
			.with_label_values(&[kind])
			.inc();
	}
}

/// Legacy: how many sends of one backfill page pass before its failures are logged as a
/// warning, and how many more between two warnings. Removed in 3.1 with the backfill.
const BACKFILL_RETRIES_WARN_AFTER: u32 = 3;
const BACKFILL_RETRIES_WARN_EVERY: u32 = 10;

/// Legacy: warns when a backfill page keeps failing: at its third re-send, then every tenth.
/// Removed in 3.1 with the backfill.
fn legacy_warn_backfill_retries(namespace: &str, attempt: u32) {
	if attempt >= BACKFILL_RETRIES_WARN_AFTER
		&& (attempt - BACKFILL_RETRIES_WARN_AFTER) % BACKFILL_RETRIES_WARN_EVERY == 0
	{
		tracing::warn!(
			namespace,
			attempt,
			"a namespace's backfill page keeps failing; an actor of it cannot be moved, see the backfill's debug logs"
		);
	}
}

/// Sends a namespace's state to one waiting create's subject.
async fn send_start_notice(
	ctx: &EngineContext,
	notice: ns::NamespaceStartNotice,
	waiter: &ns::NamespaceWaiter,
) -> Result<()> {
	let worker = universalpubsub::WorkerId::from_bytes(
		waiter
			.worker_id
			.as_ref()
			.try_into()
			.context("invalid start waiter worker id")?,
	);
	let payload = rivet_data::versioned::NamespaceStartNotice::serialize_with_embedded_version(
		&notice,
		rivet_data::generated::PEGBOARD_NAMESPACE_VERSION,
	)?;
	// Namespace notices share the actor start notice class: each goes to a process waiting on
	// its own subject, under the same caps.
	ctx.ups()?
		.request_with_timeout(
			universalpubsub::RequestClass::ActorStartNotice,
			&worker,
			NamespaceStartNoticeSubject::new(waiter.wait_id),
			&payload,
			Duration::from_millis(ctx.config().pegboard().actor_start_notice_timeout_ms()),
		)
		.await?;
	Ok(())
}

/// Which effect a group command ran for, since each reads a different set of refusals.
#[derive(Clone, Copy, Debug)]
enum GroupWrite {
	Fork,
	Archive,
}

/// A refusal of the group command that settles the effect, by code, or `None` when the outcome
/// is unknown and the retry timer re-drives it. A fork is refused for good only when its parent
/// is archived or the lineage is at its depth cap, which retires the name; an archive only when
/// the group has children that are not archived, which returns the namespace to `Live`. Any
/// other `group` code says the effect met state it did not expect, which a re-drive resolves.
fn group_refusal(write: GroupWrite, error: &anyhow::Error) -> Option<String> {
	let structured = rivet_error::RivetError::extract(error);
	if structured.group() != "group" {
		return None;
	}
	let settles = match write {
		GroupWrite::Fork => matches!(structured.code(), "archived" | "lineage_too_deep"),
		GroupWrite::Archive => structured.code() == "has_children",
	};
	if !settles {
		tracing::warn!(
			?write,
			code = structured.code(),
			"a namespace's group command answered an unexpected group error; its retry timer re-drives it"
		);
		return None;
	}
	Some(format!("{}.{}", structured.group(), structured.code()))
}

/// Creates the root group record of the namespace `name`. Depot answers an existing root record
/// as created, so a re-send settles the same way. Any error leaves the outcome unknown and the
/// retry timer re-drives it; `group.exists`, a forked record at a root's id, cannot settle by
/// retrying and is logged as an error.
async fn create_group(
	ctx: &EngineContext,
	name: &str,
	group: GroupId,
) -> ns::NamespaceCreateOutcome {
	match ctx.depot().create_group(group, group_key(name)).await {
		Ok(_) => ns::NamespaceCreateOutcome::NamespaceCreateOutcomeOk,
		Err(error) => {
			let structured = rivet_error::RivetError::extract(&error);
			if structured.group() == "group" && structured.code() == "exists" {
				tracing::error!(
					namespace = name,
					"a root namespace's group id holds a forked group record; its create cannot finish until that record is removed"
				);
			} else {
				tracing::debug!(
					?error,
					namespace = name,
					"a root namespace's group create did not finish; its retry timer re-drives it"
				);
			}
			ns::NamespaceCreateOutcome::NamespaceOutcomeUnknown(unknown(&error))
		}
	}
}

/// Runs the three group writes of a fork. The parent's record exists, since its namespace
/// created it before going `Live`.
async fn fork_group(
	ctx: &EngineContext,
	parent: GroupId,
	name: &str,
	child: GroupId,
	operation_id: [u8; 16],
	unix_ms: i64,
) -> ns::NamespaceForkOutcome {
	let fork = GroupFork {
		parent,
		child,
		child_key: group_key(name),
		operation_id,
		unix_ms,
		max_lineage_depth: ctx.config().depot().groups.max_lineage_depth(),
		event_shards: ctx.config().depot().events.shards(),
		event_retained_rows: ctx.config().depot().events.retained_rows(),
	};
	match ctx.depot().fork_group(&fork).await {
		Ok(record) => match record.base {
			Some(base) => {
				ns::NamespaceForkOutcome::NamespaceForkOutcomeOk(ns::NamespaceForkOutcomeOk {
					fork_number: base.fork_number,
				})
			}
			None => {
				tracing::error!("a forked group's record names no base; the fork is re-driven");
				ns::NamespaceForkOutcome::NamespaceOutcomeUnknown(ns::NamespaceOutcomeUnknown {
					retry_after_ms: None,
				})
			}
		},
		Err(error) => match group_refusal(GroupWrite::Fork, &error) {
			Some(code) => {
				ns::NamespaceForkOutcome::NamespaceOutcomeRefused(ns::NamespaceOutcomeRefused {
					code: code.into(),
				})
			}
			None => {
				tracing::debug!(
					?error,
					"a namespace fork's group writes did not finish; its retry timer re-drives them"
				);
				ns::NamespaceForkOutcome::NamespaceOutcomeUnknown(unknown(&error))
			}
		},
	}
}

/// The key bytes of the namespace `name`'s group, which its group record keeps.
fn group_key(name: &str) -> Vec<u8> {
	depot_types::keys::partition::NamespaceGroupKey::new(name.to_owned())
		.pack()
		.to_vec()
}

/// The unknown outcome of a group write that failed with `error`, carrying the refusal's hint
/// when depot refused the write before it ran, so the machine re-drives it promptly.
fn unknown(error: &anyhow::Error) -> ns::NamespaceOutcomeUnknown {
	ns::NamespaceOutcomeUnknown {
		retry_after_ms: depot_client::retry_after_ms(error),
	}
}

/// Archives the namespace's group, which is what closes its volumes at every tablet. The
/// group's record exists, since its namespace created it before going `Live`.
async fn archive_group(
	ctx: &EngineContext,
	group: GroupId,
	shards: u16,
) -> ns::NamespaceArchiveOutcome {
	let retained_rows = ctx.config().depot().events.retained_rows();
	match ctx
		.depot()
		.archive_group(group, shards, retained_rows)
		.await
	{
		Ok(_) => ns::NamespaceArchiveOutcome::NamespaceArchiveOutcomeOk,
		Err(error) => {
			match group_refusal(GroupWrite::Archive, &error) {
				Some(code) => ns::NamespaceArchiveOutcome::NamespaceOutcomeRefused(
					ns::NamespaceOutcomeRefused { code: code.into() },
				),
				None => {
					tracing::debug!(
						?error,
						"a namespace's group archive did not finish; its retry timer re-drives it"
					);
					ns::NamespaceArchiveOutcome::NamespaceOutcomeUnknown(unknown(&error))
				}
			}
		}
	}
}

/// Legacy: sets the gauges of a namespace's list of skipped actors. Removed in 3.1 with the
/// backfill.
fn observe_skipped_listed(
	namespace: &str,
	opening: Option<&LegacyPageOwner>,
	listed: u32,
	capacity: u32,
) {
	legacy_observe_listed(namespace, opening, listed);
	crate::metrics::BACKFILL_SKIPPED_CAPACITY.set(i64::from(capacity));
}

/// Legacy: sets `pegboard_backfill_skipped_listed` of `namespace` to `listed`, sampled from the
/// namespace's state, and ties the series to `opening`, the opening of its volume in this
/// process, which removes it when the opening goes, so a process that lost the volume shows no
/// stale value beside the one that holds it. Removed in 3.1 with the backfill.
fn legacy_observe_listed(namespace: &str, opening: Option<&LegacyPageOwner>, listed: u32) {
	crate::metrics::BACKFILL_SKIPPED_LISTED
		.with_label_values(&[namespace])
		.set(i64::from(listed));
	if let Some(opening) = opening {
		opening.pages.legacy_track_listed(namespace);
	}
}

/// Legacy: sets `pegboard_backfill_skipped_listed` of `namespace` from `state`, the namespace's
/// stored state, unless `pages`, its volume's opening in this process, already set it. The
/// transitions set the gauge only when the list changes, and an opening removes its series when
/// it goes, so without this a namespace whose backfill finished before its volume opened here
/// would show no series while its skipped actors still refuse forks and archives. A backfill not
/// yet running lists nothing and sets nothing. Removed in 3.1 with the backfill.
pub(crate) fn legacy_observe_listed_at_open(
	namespace: &str,
	pages: &LegacyBackfillPages,
	state: Option<&ns::NamespaceState>,
) {
	if pages.listed_series.get().is_some() {
		return;
	}
	let Some(state) = state else {
		return;
	};
	let listed = match &state.backfill {
		ns::NamespaceBackfill::NamespaceBackfillPending(_) => return,
		ns::NamespaceBackfill::NamespaceBackfillRunning(running) => running.skipped.len(),
		ns::NamespaceBackfill::NamespaceBackfillDone(done) => done.skipped.len(),
	};
	crate::metrics::BACKFILL_SKIPPED_LISTED
		.with_label_values(&[namespace])
		.set(i64::from(count_u32(listed)));
	pages.legacy_track_listed(namespace);
}

/// Legacy: an actor id from a backfill record, for a log field. Removed in 3.1 with the
/// backfill.
fn legacy_actor_id_label(actor_id: &[u8]) -> String {
	rivet_util::Id::from_slice(actor_id)
		.map(|id| id.to_string())
		.unwrap_or_else(|_| "invalid".to_owned())
}

/// A count as the `u32` a gauge takes, saturating.
fn count_u32(count: usize) -> u32 {
	u32::try_from(count).unwrap_or(u32::MAX)
}
