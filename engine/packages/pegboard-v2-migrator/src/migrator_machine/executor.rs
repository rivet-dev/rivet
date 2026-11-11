//! Performs the migrator state machine's effects: Depot commands on the migration and actor
//! volumes, and V2 migration API calls. Decisions stay in the core; every effect returns its
//! outcome as the matching response event.
use anyhow::{Context, Result};
use migrate_from_v2_protocol as proto;
use pegboard::{keys::partition::V2MigratorVolume, wire as pb};
use rivet_engine_context::EngineContext;
use rivet_env::TaskScope;
use rivet_singleton_coordinator::authority::Authority;
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use rivet_util::Id;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{MigratorStateMachine, effect::Effect, event::Event, types::*};
use crate::{
	commands::{MigrateFromV2StateLoad, MigrateFromV2StateWrite},
	import::{self, CopyOutcome},
	metrics,
	requests::Replies,
	v2_client::{V2ApiError, V2Client},
	v3_actor,
};

pub struct MigratorExecutor {
	ctx: EngineContext,
	authority: Authority,
	/// Rebuilt when the V2 endpoints or token change in dynamic config.
	v2: parking_lot::Mutex<Option<V2Client>>,
	tasks: TaskScope,
	/// Where answers to operator requests go.
	replies: Arc<Replies>,
}

impl MigratorExecutor {
	pub fn new(
		ctx: EngineContext,
		authority: Authority,
		v2: Option<V2Client>,
		tasks: TaskScope,
		replies: Arc<Replies>,
	) -> Self {
		Self {
			ctx,
			authority,
			v2: parking_lot::Mutex::new(v2),
			tasks,
			replies,
		}
	}

	/// Cancels every running effect. A request already sent may still land; the targets'
	/// idempotency and the migration volume's fence make that safe.
	pub fn cancel(&self) {
		self.tasks.cancel();
	}

	pub async fn shutdown(&self) {
		self.tasks.shutdown().await;
	}

	pub fn set_v2(&self, v2: Option<V2Client>) {
		*self.v2.lock() = v2;
	}

	/// The namespace the migration lands in, as a metric label.
	fn namespace(&self) -> String {
		self.v2
			.lock()
			.as_ref()
			.map(|v2| v2.target_namespace().to_owned())
			.unwrap_or_default()
	}
}

impl Executor<MigratorStateMachine> for MigratorExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<MigratorStateMachine>) {
		match effect {
			Effect::RecordEffectFailure { phase } => {
				metrics::EFFECT_FAILURES_TOTAL
					.with_label_values(&[self.namespace().as_str(), phase.as_str()])
					.inc();
				return;
			}
			Effect::RecordResult { outcome, reason } => {
				let namespace = self.namespace();
				metrics::ACTOR_RESULT_TOTAL
					.with_label_values(&[namespace.as_str(), outcome.as_str(), reason.as_str()])
					.inc();
				return;
			}
			Effect::RecordCopyAdmitted { bytes } => {
				metrics::COPY_ADMITTED_BYTES_TOTAL
					.with_label_values(&[self.namespace().as_str()])
					.inc_by(bytes);
				return;
			}
			Effect::ReportPassEnd { pass } => {
				metrics::report_pass_end(&self.namespace(), &pass);
				return;
			}
			Effect::MigrateActorResponse { request, result } => {
				self.replies.complete(request, result);
				return;
			}
			Effect::LoadStateRequest { .. }
			| Effect::WriteStateRequest { .. }
			| Effect::ListCandidatesRequest { .. }
			| Effect::ReadCandidateRequest { .. }
			| Effect::CopyRequest { .. }
			| Effect::ReadReservationRequest { .. }
			| Effect::CutoverRequest { .. }
			| Effect::ConfirmRequest { .. }
			| Effect::ActivateRequest { .. }
			| Effect::DiscardRequest { .. }
			| Effect::NotifyLegacySettled { .. } => {}
		}

		// The tenure must still hold the lease when the effect is dispatched. A write already sent
		// by a revoked tenure is refused by the migration volume's fence.
		if !self.authority.permits() {
			return;
		}

		let inbox = ctx.inbox().clone();
		let engine = self.ctx.clone();
		let v2 = self.v2.lock().clone();
		ctx.spawn_in(&self.tasks, async move {
			let event = run(&engine, v2.as_ref(), effect).await;
			if let Some(event) = event {
				let _ = send(&inbox, event).await;
			}
		});
	}
}

async fn send(inbox: &mpsc::Sender<Event>, event: Event) -> Result<()> {
	inbox.send(event).await.context("migrator inbox closed")
}

fn missing_v2() -> String {
	"migrate_from_v2 is not configured".to_string()
}

async fn run(ctx: &EngineContext, v2: Option<&V2Client>, effect: Effect) -> Option<Event> {
	Some(match effect {
		Effect::LoadStateRequest { op, tenure } => Event::LoadStateResponse {
			op,
			result: load(ctx, tenure)
				.await
				.unwrap_or_else(|error| LoadResult::Failed(format!("{error:#}"))),
		},
		Effect::WriteStateRequest {
			op,
			tenure,
			pass,
			puts,
			deletes,
		} => Event::WriteStateResponse {
			op,
			result: write(ctx, tenure, pass, puts, deletes)
				.await
				.unwrap_or_else(|error| match depot_client::retry_after_ms(&error) {
					Some(retry_after_ms) => WriteResult::Refused {
						retry_after_ms,
						error: format!("{error:#}"),
					},
					None => WriteResult::Failed(format!("{error:#}")),
				}),
		},
		Effect::ListCandidatesRequest {
			op,
			datacenter_label,
			after,
			limit,
		} => Event::ListCandidatesResponse {
			op,
			datacenter_label,
			result: match v2 {
				Some(v2) => list(v2, datacenter_label, after, limit)
					.await
					.unwrap_or_else(|error| ListResult::Failed(format!("{error:#}"))),
				None => ListResult::Failed(missing_v2()),
			},
		},
		Effect::ReadCandidateRequest { op, v2_actor_id } => Event::ReadCandidateResponse {
			op,
			result: match v2 {
				Some(v2) => read_candidate(v2, v2_actor_id)
					.await
					.unwrap_or_else(|error| CandidateResult::Failed(format!("{error:#}"))),
				None => CandidateResult::Failed(missing_v2()),
			},
		},
		Effect::CopyRequest {
			op,
			v2_actor_id,
			v3_actor_id,
			migration_id,
			candidate,
			group,
			namespace,
		} => Event::CopyResponse {
			op,
			v2_actor_id,
			result: match v2 {
				Some(v2) => copy(
					ctx,
					v2,
					namespace.as_deref().unwrap_or(v2.target_namespace()),
					group,
					v2_actor_id,
					v3_actor_id,
					migration_id,
					&candidate,
				)
				.await
				.unwrap_or_else(|error| CopyResult::Failed(format!("{error:#}"))),
				None => CopyResult::Failed(missing_v2()),
			},
		},
		Effect::CutoverRequest {
			op,
			v2_actor_id,
			generation,
			cutover_ts,
		} => Event::CutoverResponse {
			op,
			v2_actor_id,
			result: match v2 {
				Some(v2) => match v2
					.cutover(
						v2_actor_id,
						&proto::CutoverRequest {
							generation,
							cutover_ts,
						},
					)
					.await
				{
					Ok(proto::CutoverResponse::Cut) | Ok(proto::CutoverResponse::AlreadyCut) => {
						CutoverResult::Cut
					}
					Ok(proto::CutoverResponse::Woke) => CutoverResult::Woke,
					Ok(proto::CutoverResponse::Destroyed) => CutoverResult::Destroyed,
					Ok(proto::CutoverResponse::Ineligible(_)) => CutoverResult::Ineligible,
					Err(error) => CutoverResult::Failed(format!("{error:#}")),
				},
				None => CutoverResult::Failed(missing_v2()),
			},
		},
		Effect::ConfirmRequest {
			op,
			v2_actor_id,
			v3_actor_id,
			candidate,
			cutover_ts,
			horizon_ms,
		} => Event::ConfirmResponse {
			op,
			v2_actor_id,
			result: match v2 {
				Some(v2) => {
					let request = proto::ConfirmRequest {
						generation: candidate.generation,
						sleep_ts: candidate.sleep_ts,
						wake_deadline_ts: candidate.wake_deadline_ts,
						cutover_ts,
						horizon_ms,
						v3_actor_id: v3_actor_id.as_bytes().into(),
					};
					match v2.confirm(v2_actor_id, &request).await {
						Ok(proto::ConfirmResponse::Confirmed)
						| Ok(proto::ConfirmResponse::AlreadyConfirmed) => ConfirmResult::Confirmed,
						Ok(proto::ConfirmResponse::ConfirmWoke(woke)) => {
							tracing::info!(%v2_actor_id, reason = ?woke.reason, "V2 confirm found a raced wake and cleared the cutover");
							ConfirmResult::Woke
						}
						Ok(proto::ConfirmResponse::Destroyed) => ConfirmResult::Destroyed,
						Err(error)
							if error
								.downcast_ref::<V2ApiError>()
								.is_some_and(|error| error.is("migration", "cutover_mismatch")) =>
						{
							ConfirmResult::CutoverMismatch
						}
						Err(error) => ConfirmResult::Failed(format!("{error:#}")),
					}
				}
				None => ConfirmResult::Failed(missing_v2()),
			},
		},
		Effect::ReadReservationRequest {
			op,
			v2_actor_id,
			v3_actor_id,
			migration_id,
			group,
		} => Event::ReadReservationResponse {
			op,
			v2_actor_id,
			result: read_reservation(ctx, group, v3_actor_id, migration_id)
				.await
				.unwrap_or_else(|error| ReservationResult::Failed(format!("{error:#}"))),
		},
		Effect::ActivateRequest {
			op,
			v2_actor_id,
			v3_actor_id,
			migration_id,
			candidate,
			group,
		} => Event::ActivateResponse {
			op,
			v2_actor_id,
			result: activate(ctx, group, v3_actor_id, migration_id, &candidate)
				.await
				.unwrap_or_else(|error| match v3_actor::is_hold_vanished(&error) {
					true => ActivateResult::Vanished(format!("{error:#}")),
					false => ActivateResult::Failed(format!("{error:#}")),
				}),
		},
		Effect::DiscardRequest {
			op,
			v2_actor_id,
			v3_actor_id,
			migration_id,
			group,
		} => Event::DiscardResponse {
			op,
			v2_actor_id,
			result: discard(ctx, group, v3_actor_id, migration_id)
				.await
				.unwrap_or_else(|error| V3Result::Failed(format!("{error:#}"))),
		},
		Effect::NotifyLegacySettled { namespace } => {
			notify_legacy_settled(ctx, namespace).await;
			return None;
		}
		Effect::RecordResult { .. }
		| Effect::RecordEffectFailure { .. }
		| Effect::RecordCopyAdmitted { .. }
		| Effect::ReportPassEnd { .. }
		| Effect::MigrateActorResponse { .. } => return None,
	})
}

async fn load(ctx: &EngineContext, tenure: u64) -> Result<LoadResult> {
	let response = ctx
		.depot()
		.command::<MigrateFromV2StateLoad>(
			&V2MigratorVolume::new().volume_key(),
			wire::MigrateFromV2StateLoadRequest {
				coordinator_generation: tenure,
			},
		)
		.await?;
	Ok(match response {
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(loaded) => {
			LoadResult::Loaded {
				pass: loaded.pass,
				in_flight: loaded.in_flight,
			}
		}
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(_) => LoadResult::Fenced,
	})
}

async fn write(
	ctx: &EngineContext,
	tenure: u64,
	pass: Option<wire::MigrateFromV2Pass>,
	puts: Vec<wire::MigrateFromV2InFlight>,
	deletes: Vec<Id>,
) -> Result<WriteResult> {
	let response = ctx
		.depot()
		.command::<MigrateFromV2StateWrite>(
			&V2MigratorVolume::new().volume_key(),
			wire::MigrateFromV2StateWriteRequest {
				coordinator_generation: tenure,
				pass,
				puts,
				deletes: deletes.into_iter().map(|id| id.as_bytes().into()).collect(),
			},
		)
		.await?;
	Ok(match response {
		wire::MigrateFromV2StateWriteResponse::MigrateFromV2StateWritten => WriteResult::Written,
		wire::MigrateFromV2StateWriteResponse::MigrateFromV2StateFenced(_) => WriteResult::Fenced,
	})
}

fn candidate_facts(candidate: proto::Candidate) -> Result<CandidateFacts> {
	Ok(CandidateFacts {
		actor_id: Id::from_slice(&candidate.actor_id).context("candidate actor id")?,
		name: candidate.name.to_string(),
		key: candidate.key.map(|key| key.to_string()),
		pool_name: candidate.pool_name.to_string(),
		create_ts: candidate.create_ts,
		version: candidate.version,
		generation: candidate.generation,
		sleep_ts: candidate.sleep_ts,
		allocated: candidate.allocated,
		destroy_ts: candidate.destroy_ts,
		wake_deadline_ts: candidate.wake_deadline_ts,
		hibernating: candidate.hibernating,
		sqlite_v1: candidate.sqlite_v1,
		parked: candidate.parked,
		db_size_pages: candidate.db_size_pages,
		kv_bytes_estimate: candidate.kv_bytes_estimate,
	})
}

/// Reads one actor's facts for an operator request. An actor whose reply names another id is
/// refused, so a wrong reply never admits the wrong actor.
async fn read_candidate(v2: &V2Client, v2_actor_id: Id) -> Result<CandidateResult> {
	let Some(candidate) = v2.candidate(v2_actor_id).await? else {
		return Ok(CandidateResult::NotFound);
	};
	let facts = candidate_facts(candidate)?;
	anyhow::ensure!(
		facts.actor_id == v2_actor_id,
		"V2 answered a candidate read for {v2_actor_id} with actor {}",
		facts.actor_id
	);
	Ok(CandidateResult::Found(facts))
}

async fn list(
	v2: &V2Client,
	datacenter_label: u16,
	after: Option<Vec<u8>>,
	limit: u32,
) -> Result<ListResult> {
	let page = v2
		.candidates(datacenter_label, after.as_deref(), limit as usize)
		.await?;
	let candidates = page
		.candidates
		.into_iter()
		.map(|mut candidate| {
			let cursor = std::mem::take(&mut candidate.cursor).to_vec();
			Ok(ListedCandidate {
				facts: candidate_facts(candidate)?,
				cursor,
			})
		})
		.collect::<Result<Vec<_>>>()?;
	Ok(ListResult::Page {
		candidates,
		next: page.next.map(|next| next.to_vec()),
	})
}

#[allow(
	clippy::too_many_arguments,
	reason = "one argument per part of the copy effect"
)]
async fn copy(
	ctx: &EngineContext,
	v2: &V2Client,
	target_namespace: &str,
	group: Option<[u8; 16]>,
	v2_actor_id: Id,
	v3_actor_id: Id,
	migration_id: u64,
	candidate: &wire::MigrateFromV2Candidate,
) -> Result<CopyResult> {
	// A legacy-placed entry whose namespace has a group by now is placed in the group before
	// anything is written for it. The namespace gains its group once, so a grouped entry is never
	// moved back, even when this process's cached standing predates the group.
	let target = pegboard::catalog::namespace_standing(ctx, target_namespace).await?;
	if let (None, Some(standing_group)) = (group, target.group_id()) {
		return Ok(CopyResult::PlacementMoved {
			group: standing_group,
		});
	}

	// Hold the V3 actor before writing its database, so a native V3 actor of the same id (a
	// keyed actor whose key V3 already holds) is never touched. Requests still reach V2. A legacy
	// placement names the namespace in the actor's record, as a create there does.
	let legacy_namespace = match group {
		Some(_) => None,
		None => Some(target_namespace.to_owned()),
	};
	let init = pb::ActorCreateData {
		actor_id: v3_actor_id.as_bytes().into(),
		legacy_namespace: legacy_namespace.map(Into::into),
		pool_name: candidate.pool_name.clone(),
		name: candidate.name.clone(),
		// The candidate carries V2's slash key; V3 stores the canonical key. Selection admits
		// only keys that translate.
		key: candidate
			.key
			.as_deref()
			.map(|key| {
				super::v3_key(key).context("migrating V2 actor key has no canonical translation")
			})
			.transpose()?
			.map(Into::into),
		input: None,
		fork_from: None,
		legacy_move: None,
	};
	// A refused begin ran nothing, so it is sent again after the hint rather than failing the
	// copy and leaving a discard behind.
	let begun = import::resend_refused(ctx.env().clock().as_ref(), || {
		v3_actor::begin(
			ctx,
			group,
			v3_actor_id,
			migration_id,
			init.clone(),
			candidate.create_ts,
		)
	})
	.await?;
	let create_unix_ms = match begun {
		v3_actor::Begun::Held { create_unix_ms } => create_unix_ms,
		v3_actor::Begun::NativeActor => {
			tracing::warn!(%v2_actor_id, %v3_actor_id, "the V3 id already holds an actor; skipping");
			return Ok(CopyResult::NativeActor);
		}
		v3_actor::Begun::Elsewhere => {
			return Ok(CopyResult::Failed(
				"the V3 actor is held by another migration or being destroyed".into(),
			));
		}
	};

	#[allow(
		clippy::disallowed_methods,
		reason = "the copy duration only feeds a metric"
	)]
	let started = std::time::Instant::now();
	let outcome = import::copy(
		ctx,
		v2,
		group,
		v2_actor_id,
		v3_actor_id,
		create_unix_ms,
		candidate.generation,
		candidate.sleep_ts,
	)
	.await?;
	Ok(match outcome {
		CopyOutcome::Copied(copied) => {
			metrics::observe_copy(
				target_namespace,
				copied.kv_bytes,
				u64::from(copied.pages) * crate::SQLITE_PAGE_SIZE as u64,
				started.elapsed(),
			);
			CopyResult::Copied {
				kv_bytes: copied.kv_bytes,
				pages: copied.pages,
			}
		}
		CopyOutcome::FenceMoved | CopyOutcome::Aborted(proto::AbortReason::FenceMoved) => {
			CopyResult::FenceMoved
		}
		CopyOutcome::Aborted(proto::AbortReason::Destroyed) => CopyResult::Destroyed,
		CopyOutcome::Aborted(proto::AbortReason::Ineligible) => CopyResult::Ineligible,
	})
}

async fn read_reservation(
	ctx: &EngineContext,
	group: Option<[u8; 16]>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<ReservationResult> {
	Ok(
		match v3_actor::reservation(ctx, group, v3_actor_id, migration_id).await? {
			v3_actor::Reservation::Reserved => ReservationResult::Reserved,
			v3_actor::Reservation::Pending => ReservationResult::Pending,
			v3_actor::Reservation::OwnedElsewhere => ReservationResult::OwnedElsewhere,
			v3_actor::Reservation::Exhausted => ReservationResult::Exhausted,
			v3_actor::Reservation::NotHeld => ReservationResult::NotHeld,
		},
	)
}

/// Activates the held actor at the entry's placement. An actor activated at the legacy placement
/// has its name recorded on the volume of the namespace its record names before the step reports
/// success, so the backfill's walk lists the name and moves the actor into the group; a failed
/// record fails the step, whose retry activates nothing again and records the name again. A
/// namespace with no volume yet has no backfill either, and its walk will find the name where it
/// finds every other legacy actor's. The record names the namespace whatever the configuration
/// says now, so a migration removed or retargeted since the entry's admission records it where
/// the actor lives.
async fn activate(
	ctx: &EngineContext,
	group: Option<[u8; 16]>,
	v3_actor_id: Id,
	migration_id: u64,
	candidate: &wire::MigrateFromV2Candidate,
) -> Result<ActivateResult> {
	let activation = v3_actor::activate(
		ctx,
		group,
		v3_actor_id,
		migration_id,
		candidate.wake_deadline_ts,
	)
	.await?;
	Ok(match activation {
		v3_actor::Activation::Applied { legacy_namespace } => {
			if group.is_none() {
				record_legacy_name(ctx, legacy_namespace.as_deref(), &candidate.name).await?;
			}
			ActivateResult::Applied
		}
		v3_actor::Activation::IndexPending => ActivateResult::IndexPending,
		v3_actor::Activation::StillHeld => {
			ActivateResult::Failed("the activation did not apply; the actor is still held".into())
		}
		v3_actor::Activation::Elsewhere => ActivateResult::Refused,
	})
}

/// The namespace an entry names, or the configured target namespace for an entry written by
/// version 1, which did not record it. `None` when neither is known.
fn entry_namespace(ctx: &EngineContext, namespace: Option<&str>) -> Option<String> {
	match namespace {
		Some(namespace) => Some(namespace.to_owned()),
		None => ctx
			.config()
			.dynamic()
			.migrate_from_v2()
			.map(|migration| migration.target_namespace().to_owned()),
	}
}

/// Tells `namespace` that legacy-placed entries finished, so its backfill reads the migrator's
/// state again. Best effort, since the backfill's retry timer reads it anyway.
async fn notify_legacy_settled(ctx: &EngineContext, namespace: Option<String>) {
	let Some(namespace) = entry_namespace(ctx, namespace.as_deref()) else {
		return;
	};
	let namespace = namespace.as_str();
	if let Err(error) = pegboard::catalog::legacy_notify_migrator_settled(ctx, namespace).await {
		tracing::warn!(
			?error,
			namespace,
			"telling the namespace that legacy-placed migrations finished failed; its backfill reads the migrator's state on its retry timer"
		);
	}
}

/// Records `name` for the backfill of `namespace`, the namespace the activated actor's record
/// names, which walks the legacy index buckets of the names it lists. A record that names none
/// falls back to the configured target namespace.
async fn record_legacy_name(
	ctx: &EngineContext,
	namespace: Option<&str>,
	name: &str,
) -> Result<()> {
	let namespace = entry_namespace(ctx, namespace).context(
		"the activated actor's record names no namespace and no migration is configured",
	)?;
	let recorded = pegboard::catalog::legacy_record_backfill_name(ctx, &namespace, name).await?;
	if !recorded {
		tracing::debug!(
			namespace,
			"the namespace has no volume yet; its backfill finds the activated actor's name as it finds every legacy actor's"
		);
	}
	Ok(())
}

async fn discard(
	ctx: &EngineContext,
	group: Option<[u8; 16]>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<V3Result> {
	Ok(
		match v3_actor::discard(ctx, group, v3_actor_id, migration_id).await? {
			v3_actor::Settled::Applied | v3_actor::Settled::Elsewhere => V3Result::Applied,
			v3_actor::Settled::StillHeld => {
				V3Result::Failed("the discard did not apply; the actor is still held".into())
			}
		},
	)
}
