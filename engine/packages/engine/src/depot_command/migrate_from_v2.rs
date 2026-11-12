//! Operator commands of the migration of a V2 namespace into this cluster.
use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use pegboard_v2_migrator::migrator_machine::types::wire;
use rivet_config::{DynamicConfigUpdate, config::MigrateFromV2};
use rivet_engine_context::EngineContext;
use rivet_util::Id;
use serde_json::{Value, json};

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Replaces the cluster's `migrate_from_v2` settings with a JSON file, through dynamic
	/// config. Every process applies it within about a minute. `migrate_seed` must stay as loaded
	/// at startup.
	SetConfig {
		/// JSON object with the `migrate_from_v2` fields.
		file: PathBuf,
	},
	/// Clears the dynamic `migrate_from_v2` override, so the startup config applies again.
	ClearConfig,
	/// Prints the durable migration state: the current pass and every actor in flight.
	Status,
	/// Moves one V2 actor into this cluster now, whatever `migrate_ratio` says, and waits until
	/// it has migrated. While the migrator is busy the command keeps retrying and prints that it
	/// is still waiting. The JSON result names the outcome in `result`: `migrated`, `admitted`
	/// (with `--no-wait`), `ineligible` with a `reason` such as `recently_awake` (every other
	/// selection rule still applies), or `not_found`. Running it again for the same actor is safe.
	Migrate {
		v2_actor_id: String,
		/// Return once the actor is admitted instead of waiting for its migration to finish.
		#[arg(long)]
		no_wait: bool,
		/// Keep retrying an actor that is not eligible yet, such as one that is awake.
		#[arg(long)]
		wait_eligible: bool,
		/// Give up after this many seconds with an error.
		#[arg(long, default_value_t = 1800)]
		timeout_secs: u64,
	},
	/// Hands a migrated actor back to V2, which serves it again as it was at migration. Writes
	/// the actor made here after it migrated are dropped.
	Recover {
		v2_actor_id: String,
		/// The actor's id here, for a keyed actor; a keyless actor keeps its V2 id.
		#[arg(long)]
		v3_actor_id: Option<String>,
	},
}

pub(crate) async fn execute(ctx: &EngineContext, command: Command) -> Result<Value> {
	match command {
		Command::SetConfig { file } => {
			let raw = ctx
				.fs()
				.read(&file)
				.await
				.with_context(|| format!("read {}", file.display()))?;
			let migration: MigrateFromV2 =
				serde_json::from_slice(&raw).context("parse migrate_from_v2 json")?;
			migration.validate()?;
			update_config(ctx, Some(migration)).await
		}
		Command::ClearConfig => update_config(ctx, None).await,
		// Every status read and recovery step is idempotent, so a refused attempt is sent again
		// whole.
		Command::Status => super::routed(ctx, || pegboard_v2_migrator::operator::status(ctx)).await,
		// The request is sent once: an operator who got no answer checks `status` and sends it
		// again, which is safe.
		Command::Migrate {
			v2_actor_id,
			no_wait,
			wait_eligible,
			timeout_secs,
		} => {
			let v2_actor_id = Id::parse(&v2_actor_id).context("invalid V2 actor id")?;
			migrate(
				ctx,
				v2_actor_id,
				MigrateOptions {
					no_wait,
					wait_eligible,
					timeout: Duration::from_secs(timeout_secs),
				},
			)
			.await
		}
		Command::Recover {
			v2_actor_id,
			v3_actor_id,
		} => {
			let v2_actor_id = Id::parse(&v2_actor_id).context("invalid V2 actor id")?;
			let v3_actor_id = v3_actor_id
				.as_deref()
				.map(|id| Id::parse(id).context("invalid V3 actor id"))
				.transpose()?;
			super::routed(ctx, || {
				pegboard_v2_migrator::operator::recover(ctx, v2_actor_id, v3_actor_id)
			})
			.await
		}
	}
}

struct MigrateOptions {
	no_wait: bool,
	wait_eligible: bool,
	timeout: Duration,
}

/// Shortest and longest wait between requests while the migrator is busy.
const BUSY_RETRY_MIN: Duration = Duration::from_secs(2);
const BUSY_RETRY_MAX: Duration = Duration::from_secs(15);
/// How often the state is read while the admitted migration runs.
const MIGRATION_POLL: Duration = Duration::from_secs(5);
/// How long an ineligible actor waits before it is asked for again with `--wait-eligible`.
const ELIGIBLE_RETRY: Duration = Duration::from_secs(30);
/// How often a still-waiting line is printed while the migration runs.
const WAITING_NOTICE: Duration = Duration::from_secs(30);

/// Requests the migration of one actor until the migrator admits it, then waits until it leaves
/// the in-flight state. Every request is idempotent: an actor already in flight is answered as
/// admitted, so a busy refusal, a lost reply or an abandoned attempt is simply asked again.
async fn migrate(ctx: &EngineContext, v2_actor_id: Id, options: MigrateOptions) -> Result<Value> {
	let clock = ctx.env().clock();
	let started_ms = clock.observe()?.boot_ms;
	let deadline_ms = started_ms.saturating_add(options.timeout.as_millis() as u64);
	let elapsed_s = |now_ms: u64| now_ms.saturating_sub(started_ms) / 1000;
	let mut busy_retry = BUSY_RETRY_MIN;

	loop {
		let now_ms = clock.observe()?.boot_ms;
		if now_ms >= deadline_ms {
			bail!(
				"gave up on migrating {v2_actor_id} after {}s; it may still be in flight, so check \
				 `migrate-from-v2 status` and run the command again",
				elapsed_s(now_ms)
			);
		}

		let response = match pegboard_v2_migrator::operator::migrate_once(ctx, v2_actor_id).await {
			Ok(response) => response,
			Err(error) => {
				progress(&format!(
					"could not reach the migrator ({error:#}); still waiting, retrying in {}s",
					busy_retry.as_secs()
				));
				rivet_env::tokio::time::sleep(clock.as_ref(), busy_retry).await?;
				busy_retry = (busy_retry * 2).min(BUSY_RETRY_MAX);
				continue;
			}
		};

		match response {
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorAdmitted(admitted) => {
				let v3_actor_id = Id::from_slice(&admitted.v3_actor_id)?.to_string();
				let result = json!({
					"v2_actor_id": v2_actor_id.to_string(),
					"v3_actor_id": v3_actor_id,
					"migration_id": admitted.migration_id,
				});
				if options.no_wait {
					return Ok(with_result(
						result,
						"admitted",
						elapsed_s(clock.observe()?.boot_ms),
					));
				}
				match wait_for_migration(ctx, v2_actor_id, deadline_ms).await? {
					Some(true) => {
						return Ok(with_result(
							result,
							"migrated",
							elapsed_s(clock.observe()?.boot_ms),
						));
					}
					// The attempt was abandoned before V2 confirmed it, for example because the
					// actor woke. Asking again reports why, or starts a new attempt.
					Some(false) | None => {
						progress(
							"the migration was abandoned before V2 confirmed it; asking again",
						);
						busy_retry = BUSY_RETRY_MIN;
					}
				}
			}
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorIneligible(ineligible) => {
				let reason = ineligible.reason.to_string();
				if reason == "already_migrated" {
					return Ok(json!({
						"v2_actor_id": v2_actor_id.to_string(),
						"result": "migrated",
						"already": true,
						"elapsed_s": elapsed_s(clock.observe()?.boot_ms),
					}));
				}
				if !options.wait_eligible {
					return Ok(json!({
						"v2_actor_id": v2_actor_id.to_string(),
						"result": "ineligible",
						"reason": reason,
					}));
				}
				progress(&format!(
					"not eligible yet ({reason}); still waiting, retrying in {}s",
					ELIGIBLE_RETRY.as_secs()
				));
				rivet_env::tokio::time::sleep(clock.as_ref(), ELIGIBLE_RETRY).await?;
			}
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorNotFound => {
				return Ok(json!({
					"v2_actor_id": v2_actor_id.to_string(),
					"result": "not_found",
				}));
			}
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorUnavailable(unavailable) => {
				progress(&format!(
					"still waiting on the migrator queue ({}), retrying in {}s",
					unavailable.reason,
					busy_retry.as_secs()
				));
				rivet_env::tokio::time::sleep(clock.as_ref(), busy_retry).await?;
				busy_retry = (busy_retry * 2).min(BUSY_RETRY_MAX);
			}
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorFailed(failed) => {
				progress(&format!(
					"the migrator could not read the actor from V2 ({}); still waiting, retrying in {}s",
					failed.message,
					busy_retry.as_secs()
				));
				rivet_env::tokio::time::sleep(clock.as_ref(), busy_retry).await?;
				busy_retry = (busy_retry * 2).min(BUSY_RETRY_MAX);
			}
		}
	}
}

/// Waits until the actor's in-flight entry is gone, then reports whether V2 parked it, which
/// means the migration was confirmed. `None` when the deadline passes first.
async fn wait_for_migration(
	ctx: &EngineContext,
	v2_actor_id: Id,
	deadline_ms: u64,
) -> Result<Option<bool>> {
	let clock = ctx.env().clock();
	let mut last_notice_ms = clock.observe()?.boot_ms;
	progress("admitted; waiting for the migration to finish");
	loop {
		let now_ms = clock.observe()?.boot_ms;
		if now_ms >= deadline_ms {
			return Ok(None);
		}
		let phase = super::routed(ctx, || {
			pegboard_v2_migrator::operator::in_flight_phase(ctx, v2_actor_id)
		})
		.await?;
		if phase.is_none() {
			let parked = pegboard_v2_migrator::operator::parked_in_v2(ctx, v2_actor_id).await?;
			return Ok(Some(parked == Some(true)));
		}
		if now_ms.saturating_sub(last_notice_ms) >= WAITING_NOTICE.as_millis() as u64 {
			progress("still migrating...");
			last_notice_ms = now_ms;
		}
		rivet_env::tokio::time::sleep(clock.as_ref(), MIGRATION_POLL).await?;
	}
}

fn with_result(mut value: Value, result: &str, elapsed_s: u64) -> Value {
	value["result"] = json!(result);
	value["elapsed_s"] = json!(elapsed_s);
	value
}

/// A still-waiting notice for the operator watching the command. The result stays on stdout.
#[allow(
	clippy::disallowed_macros,
	reason = "operator CLI notices are explicitly requested stderr output, not logs"
)]
fn progress(message: &str) {
	eprintln!("{message}");
}

/// Writes only the `migrate_from_v2` field of the cluster's dynamic config, so unrelated
/// settings survive. `None` removes the override.
async fn update_config(ctx: &EngineContext, migration: Option<MigrateFromV2>) -> Result<Value> {
	let update = DynamicConfigUpdate {
		migrate_from_v2: Some(migration),
		..Default::default()
	};
	rivet_cluster_config::update_dynamic_config(
		ctx.env().clone(),
		ctx.object_stores().primary.clone(),
		ctx.config(),
		&update,
	)
	.await?;
	Ok(json!({ "updated": "migrate_from_v2" }))
}
