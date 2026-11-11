//! Operator commands of the migration from V2, behind `rivet-engine migrate-from-v2`. They read
//! and act through the cluster like any client, so they work from any process that joins it.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use pegboard::{keys::partition::V2MigratorVolume, migrate_from_v2::MigrateFromV2StateRead};
use rivet_engine_context::EngineContext;
use rivet_singleton_coordinator::{
	singleton_coordinator_machine::{executor::LeaseStore, types::LeaseRead},
	store::ObjectLeaseStore,
};
use rivet_util::Id;
use serde_json::{Value, json};

use crate::{
	migrator_machine::{snapshot::phase_name, types::wire},
	v2_client::V2Client,
};

/// The durable migration state: the pass entry and every actor in flight. It reads without
/// claiming the state, so a running migrator is unaffected.
pub async fn status(ctx: &EngineContext) -> Result<Value> {
	let response = ctx
		.depot()
		.command::<MigrateFromV2StateRead>(&V2MigratorVolume::new().volume_key(), ())
		.await?;
	let loaded = match response {
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(loaded) => loaded,
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(_) => {
			bail!("a state read is never fenced")
		}
	};
	let in_flight = loaded
		.in_flight
		.iter()
		.map(|entry| {
			Ok(json!({
				"v2_actor_id": Id::from_slice(&entry.candidate.v2_actor_id)?.to_string(),
				"v3_actor_id": Id::from_slice(&entry.v3_actor_id)?.to_string(),
				"migration_id": entry.migration_id,
				"phase": phase_name(&entry.phase),
				"started_at": entry.started_at,
			}))
		})
		.collect::<Result<Vec<_>>>()?;
	Ok(json!({
		"pass": loaded.pass.as_ref().map(pass_json),
		"in_flight": in_flight,
	}))
}

/// The pass entry as JSON.
fn pass_json(pass: &wire::MigrateFromV2Pass) -> serde_json::Value {
	json!({
		"pass": pass.pass,
		"started_at": pass.started_at,
		"ended_at": pass.ended_at,
		"cursors": pass.cursors.iter().map(|cursor| json!({
			"datacenter_label": cursor.datacenter_label,
			"done": cursor.done,
		})).collect::<Vec<_>>(),
		"counts": {
			"listed": pass.counts.listed,
			"selected": pass.counts.selected,
			"confirmed": pass.counts.confirmed,
			"discarded": pass.counts.discarded,
			"skipped": pass.counts.skipped,
			"duplicate_key": pass.counts.duplicate_key,
			"already_migrated": pass.counts.already_migrated,
			"not_selected": pass.counts.not_selected,
			"too_large": pass.counts.too_large,
			"needs_recovery": pass.counts.needs_recovery,
		},
	})
}

/// Hands a migrated actor back to V2; see [`crate::recover`].
pub async fn recover(
	ctx: &EngineContext,
	v2_actor_id: Id,
	v3_actor_id: Option<Id>,
) -> Result<Value> {
	let recovered = crate::recover::recover(ctx, v2_actor_id, v3_actor_id).await?;
	Ok(serde_json::to_value(recovered)?)
}

/// The longest one request waits for the holder's answer, which is UPS's request ceiling.
const MIGRATE_TIMEOUT: Duration = Duration::from_secs(30);

/// Asks the migrator to move one V2 actor now, whatever `migrate_ratio` says; see
/// [`crate::requests`]. Every other selection rule still applies. One request, one answer: an
/// operator that waits through a busy migrator or for the migration to finish repeats it, which
/// is safe because an actor already in flight is answered as admitted.
pub async fn migrate_once(
	ctx: &EngineContext,
	v2_actor_id: Id,
) -> Result<wire::MigrateFromV2ActorResponse> {
	let lease = rivet_env::timeout(
		ctx.env().clock().as_ref(),
		MIGRATE_TIMEOUT,
		ObjectLeaseStore::new(ctx.object_stores().lease.clone()).read(),
	)
	.await?;
	let tenure = match lease {
		LeaseRead::Found(lease) if !lease.released => lease.tenure,
		LeaseRead::Found(_) | LeaseRead::Missing | LeaseRead::Unknown => {
			bail!("no worker holds the singleton tenure, so no migrator is running")
		}
	};
	crate::requests::request(
		&ctx.ups()?,
		ctx.topology().as_ref(),
		&tenure,
		v2_actor_id,
		MIGRATE_TIMEOUT,
	)
	.await
}

/// The phase of `v2_actor_id`'s in-flight entry, or `None` when it has none: it never started
/// or its migration ended. Reads the state without claiming it.
pub async fn in_flight_phase(ctx: &EngineContext, v2_actor_id: Id) -> Result<Option<&'static str>> {
	let response = ctx
		.depot()
		.command::<MigrateFromV2StateRead>(&V2MigratorVolume::new().volume_key(), ())
		.await?;
	let loaded = match response {
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(loaded) => loaded,
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(_) => {
			bail!("a state read is never fenced")
		}
	};
	for entry in &loaded.in_flight {
		if Id::from_slice(&entry.candidate.v2_actor_id)? == v2_actor_id {
			return Ok(Some(phase_name(&entry.phase)));
		}
	}
	Ok(None)
}

/// Whether V2 parked `v2_actor_id`, which it does only when a migration confirmed it, so V3 now
/// holds the actor. `None` when V2 holds no such actor.
pub async fn parked_in_v2(ctx: &EngineContext, v2_actor_id: Id) -> Result<Option<bool>> {
	let migration = ctx
		.config()
		.dynamic()
		.migrate_from_v2()
		.cloned()
		.context("migrate_from_v2 is not configured")?;
	let v2 = V2Client::new(ctx.external_http(), &migration)?;
	Ok(v2
		.candidate(v2_actor_id)
		.await?
		.map(|candidate| candidate.parked))
}
