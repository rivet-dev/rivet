//! Implementation of a workflow database driver with PostgreSQL (CockroachDB) and NATS.

use std::{
	collections::HashSet,
	sync::Arc,
	time::{Duration, Instant},
};

use futures_util::{stream::BoxStream, StreamExt};
use indoc::indoc;
use rivet_pools::prelude::*;
use sqlx::{pool::PoolConnection, Acquire, PgPool, Postgres};
use tracing::Instrument;
use types::*;
use uuid::Uuid;

use super::{Database, PulledWorkflowData, SignalData, WorkflowData};
use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{EventId, EventType, SleepState},
		location::Location,
	},
	metrics,
};

mod debug;
mod types;

// HACK: We alias global error here because its hardcoded into the sql macros
type GlobalError = WorkflowError;

/// Max amount of workflows pulled from the database with each call to `pull_workflows`.
const MAX_PULLED_WORKFLOWS: i64 = 50;
// Base retry for query retry backoff
const QUERY_RETRY_MS: usize = 500;
// Time in between transaction retries
const TXN_RETRY: Duration = Duration::from_millis(100);
/// Maximum times a query ran by this database adapter is retried.
const MAX_QUERY_RETRIES: usize = 16;
/// How long before considering the leases of a given worker instance "expired".
const WORKER_INSTANCE_EXPIRED_THRESHOLD_MS: i64 = rivet_util::duration::seconds(30);
/// How long before overwriting an existing GC lock.
const GC_LOCK_TIMEOUT_MS: i64 = rivet_util::duration::seconds(30);
/// How long before overwriting an existing metrics lock.
const METRICS_LOCK_TIMEOUT_MS: i64 = GC_LOCK_TIMEOUT_MS;
/// For SQL macros.
const CONTEXT_NAME: &str = "chirp_workflow_crdb_nats_engine";
/// For NATS wake mechanism.
const WORKER_WAKE_SUBJECT: &str = "chirp.workflow.crdb_nats.worker.wake";

pub struct DatabaseCrdbNats {
	pool: PgPool,
	nats: NatsPool,
}

impl DatabaseCrdbNats {
	#[tracing::instrument(skip_all)]
	async fn conn(&self) -> WorkflowResult<PoolConnection<Postgres>> {
		// Attempt to use an existing connection
		if let Some(conn) = self.pool.try_acquire() {
			Ok(conn)
		} else {
			// Create a new connection
			self.pool.acquire().await.map_err(WorkflowError::Sqlx)
		}
	}

	// Alias function for sql macro compatibility
	#[tracing::instrument(skip_all)]
	async fn crdb(&self) -> WorkflowResult<PgPool> {
		Ok(self.pool.clone())
	}

	// For SQL macros
	fn name(&self) -> &str {
		CONTEXT_NAME
	}

	/// Spawns a new thread and publishes a worker wake message to nats.
	fn wake_worker(&self) {
		let nats = self.nats.clone();

		let spawn_res = tokio::task::Builder::new().name("wake").spawn(
			async move {
				// Fail gracefully
				if let Err(err) = nats.publish(WORKER_WAKE_SUBJECT, Vec::new().into()).await {
					tracing::warn!(?err, "failed to publish wake message");
				}
			}
			.instrument(tracing::info_span!("wake_worker_publish")),
		);
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn wake task");
		}
	}

	/// Executes queries and explicitly handles retry errors.
	#[tracing::instrument(skip_all)]
	async fn query<'a, F, Fut, T>(&self, mut cb: F) -> WorkflowResult<T>
	where
		F: FnMut() -> Fut,
		Fut: std::future::Future<Output = WorkflowResult<T>> + 'a,
		T: 'a,
	{
		let mut backoff = rivet_util::Backoff::new(4, None, QUERY_RETRY_MS, 50);
		let mut i = 0;

		loop {
			match cb().await {
				Err(WorkflowError::Sqlx(err)) => {
					i += 1;
					if i > MAX_QUERY_RETRIES {
						return Err(WorkflowError::MaxSqlRetries(err));
					}

					use sqlx::Error::*;
					match &err {
						// Retry transaction errors in a tight loop
						Database(db_err)
							if db_err
								.message()
								.contains("TransactionRetryWithProtoRefreshError") =>
						{
							tracing::warn!(message=%db_err.message(), "transaction retry");
							tokio::time::sleep(TXN_RETRY).await;
						}
						// Retry other errors with a backoff
						Database(_) | Io(_) | Tls(_) | Protocol(_) | PoolTimedOut | PoolClosed
						| WorkerCrashed => {
							tracing::warn!(?err, "query retry");
							backoff.tick().await;
						}
						// Throw error
						_ => return Err(WorkflowError::Sqlx(err)),
					}
				}
				x => return x,
			}
		}
	}
}

#[async_trait::async_trait]
impl Database for DatabaseCrdbNats {
	fn from_pools(pools: rivet_pools::Pools) -> Result<Arc<DatabaseCrdbNats>, rivet_pools::Error> {
		Ok(Arc::new(DatabaseCrdbNats {
			pool: pools.crdb()?,
			nats: pools.nats()?,
		}))
	}

	#[tracing::instrument(skip_all)]
	async fn wake_sub<'a, 'b>(&'a self) -> WorkflowResult<BoxStream<'b, ()>> {
		let stream = self
			.nats
			.subscribe(WORKER_WAKE_SUBJECT)
			.await
			.map_err(|x| WorkflowError::CreateSubscription(x.into()))?
			.map(|_| ());

		Ok(stream.boxed())
	}

	#[tracing::instrument(skip_all)]
	async fn update_worker_ping(&self, worker_instance_id: Uuid) -> WorkflowResult<()> {
		// Always update ping
		metrics::WORKER_LAST_PING
			.with_label_values(&[&worker_instance_id.to_string()])
			.set(rivet_util::timestamp::now());

		sql_execute!(
			[self]
			"
			UPSERT INTO db_workflow.worker_instances (worker_instance_id, last_ping_ts)
			VALUES ($1, $2)
			",
			worker_instance_id,
			rivet_util::timestamp::now(),
		)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn clear_expired_leases(&self, worker_instance_id: Uuid) -> WorkflowResult<()> {
		let acquired_lock = sql_fetch_optional!(
			[self, (i64,)]
			"
			UPDATE db_workflow.workflow_gc
			SET
				worker_instance_id = $1,
				lock_ts = $2
			WHERE lock_ts IS NULL OR lock_ts < $2 - $3
			RETURNING 1
			",
			worker_instance_id,
			rivet_util::timestamp::now(),
			GC_LOCK_TIMEOUT_MS,
		)
		.await?
		.is_some();

		if acquired_lock {
			// Reset all workflows on worker instances that have not had a ping in the last 30 seconds
			let rows = sql_fetch_all!(
				[self, (Uuid, Uuid,)]
				"
				UPDATE db_workflow.workflows@workflows_active_idx AS w
				SET
					worker_instance_id = NULL,
					wake_immediate = true,
					wake_deadline_ts = NULL,
					wake_signals = ARRAY[],
					wake_sub_workflow_id = NULL
				FROM db_workflow.worker_instances@worker_instances_ping_idx AS wi
				WHERE
					wi.last_ping_ts < $1 AND
					wi.worker_instance_id = w.worker_instance_id AND
					w.output IS NULL AND
					w.silence_ts IS NULL AND
					-- Check for any wake condition so we don't restart a permanently dead workflow
					(
						w.wake_immediate OR
						w.wake_deadline_ts IS NOT NULL OR
						cardinality(w.wake_signals) > 0 OR
						w.wake_sub_workflow_id IS NOT NULL
					)
				RETURNING w.workflow_id, wi.worker_instance_id
				",
				rivet_util::timestamp::now() - WORKER_INSTANCE_EXPIRED_THRESHOLD_MS,
			)
			.await?;

			if !rows.is_empty() {
				let unique_worker_instance_ids = rows
					.iter()
					.map(|(_, worker_instance_id)| worker_instance_id)
					.collect::<HashSet<_>>();

				tracing::info!(
					worker_instance_ids=?unique_worker_instance_ids,
					total_workflows=%rows.len(),
					"handled failover",
				);

				self.wake_worker();
			}

			// Clear lock
			sql_execute!(
				[self]
				"
				UPDATE db_workflow.workflow_gc
				SET
					worker_instance_id = NULL,
					lock_ts = NULL
				WHERE worker_instance_id = $1
				",
				worker_instance_id,
			)
			.await?;
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_metrics(&self, worker_instance_id: Uuid) -> WorkflowResult<()> {
		let acquired_lock = sql_fetch_optional!(
			[self, (i64,)]
			"
			UPDATE db_workflow.workflow_metrics
			SET
				worker_instance_id = $1,
				lock_ts = $2
			WHERE lock_ts IS NULL OR lock_ts < $2 - $3
			RETURNING 1
			",
			worker_instance_id,
			rivet_util::timestamp::now(),
			METRICS_LOCK_TIMEOUT_MS,
		)
		.await?
		.is_some();

		if acquired_lock {
			let (
				total_workflow_count,
				active_workflow_count,
				dead_workflow_count,
				sleeping_workflow_count,
				pending_signal_count,
			) = tokio::try_join!(
				sql_fetch_all!(
					[self, (String, i64)]
					"
					SELECT workflow_name, COUNT(*)
					FROM db_workflow.workflows@workflows_total_count_idx
					AS OF SYSTEM TIME '-1s'
					GROUP BY workflow_name
					",
				),
				sql_fetch_all!(
					[self, (String, i64)]
					"
					SELECT workflow_name, COUNT(*)
					FROM db_workflow.workflows@workflows_active_count_idx
					AS OF SYSTEM TIME '-1s'
					WHERE
						output IS NULL AND
						worker_instance_id IS NOT NULL AND
						silence_ts IS NULL
					GROUP BY workflow_name
					",
				),
				sql_fetch_all!(
					[self, (String, String, i64)]
					"
					SELECT workflow_name, error, COUNT(*)
					FROM db_workflow.workflows@workflows_dead_count_idx
					AS OF SYSTEM TIME '-1s'
					WHERE
						error IS NOT NULL AND
						output IS NULL AND
						silence_ts IS NULL AND
						wake_immediate = FALSE AND
						wake_deadline_ts IS NULL AND
						cardinality(wake_signals) = 0 AND
						wake_sub_workflow_id IS NULL
					GROUP BY workflow_name, error
					",
				),
				sql_fetch_all!(
					[self, (String, i64)]
					"
					SELECT workflow_name, COUNT(*)
					FROM db_workflow.workflows@workflows_sleeping_count_idx
					AS OF SYSTEM TIME '-1s'
					WHERE
						worker_instance_id IS NULL AND
						output IS NULL AND
						silence_ts IS NULL AND
						(
							wake_immediate OR
							wake_deadline_ts IS NOT NULL OR
							cardinality(wake_signals) > 0 OR
							wake_sub_workflow_id IS NOT NULL
						)
					GROUP BY workflow_name
					",
				),
				sql_fetch_all!(
					[self, (String, i64)]
					"
					SELECT signal_name, COUNT(*)
					FROM (
						SELECT signal_name
						FROM db_workflow.signals@signals_unack_idx
						WHERE
							ack_ts IS NULL AND
							silence_ts IS NULL
						UNION ALL
						SELECT signal_name
						FROM db_workflow.tagged_signals@tagged_signals_unack_idx
						WHERE
							ack_ts IS NULL AND
							silence_ts IS NULL
					)
					AS OF SYSTEM TIME '-1s'
					GROUP BY signal_name
					",
				),
			)?;

			// Get rid of metrics that don't exist in the db anymore (declarative)
			metrics::WORKFLOW_TOTAL.reset();
			metrics::WORKFLOW_ACTIVE.reset();
			metrics::WORKFLOW_DEAD.reset();
			metrics::WORKFLOW_SLEEPING.reset();
			metrics::SIGNAL_PENDING.reset();

			for (workflow_name, count) in total_workflow_count {
				metrics::WORKFLOW_TOTAL
					.with_label_values(&[&workflow_name])
					.set(count);
			}

			for (workflow_name, count) in active_workflow_count {
				metrics::WORKFLOW_ACTIVE
					.with_label_values(&[&workflow_name])
					.set(count);
			}

			for (workflow_name, error, count) in dead_workflow_count {
				metrics::WORKFLOW_DEAD
					.with_label_values(&[&workflow_name, &error])
					.set(count);
			}

			for (workflow_name, count) in sleeping_workflow_count {
				metrics::WORKFLOW_SLEEPING
					.with_label_values(&[&workflow_name])
					.set(count);
			}

			for (signal_name, count) in pending_signal_count {
				metrics::SIGNAL_PENDING
					.with_label_values(&[&signal_name])
					.set(count);
			}

			// Clear lock
			sql_execute!(
				[self]
				"
				UPDATE db_workflow.workflow_metrics
				SET
					worker_instance_id = NULL,
					lock_ts = NULL
				WHERE worker_instance_id = $1
				",
				worker_instance_id,
			)
			.await?;
		}

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(%workflow_id, %workflow_name, unique))]
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		unique: bool,
	) -> WorkflowResult<Uuid> {
		let query = if unique {
			// Check if an incomplete workflow with the given name and tags already exists
			indoc!(
				"
				WITH
					select_existing AS (
						SELECT workflow_id, input, output
						FROM db_workflow.workflows
						WHERE
							workflow_name = $2 AND
							tags <@ $5 AND
							output IS NULL
						LIMIT 1
					),
					insert_workflow AS (
						INSERT INTO db_workflow.workflows (
							workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
						)
						SELECT $1, $2, $3, $4, $5, $6, true
						WHERE NOT EXISTS(SELECT 1 FROM select_existing)
						RETURNING workflow_id
					)
				SELECT COALESCE(s.workflow_id, i.workflow_id)
				FROM select_existing AS s
				FULL OUTER JOIN insert_workflow AS i
				ON TRUE
				",
			)
		} else {
			indoc!(
				"
				INSERT INTO db_workflow.workflows (
					workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
				)
				VALUES ($1, $2, $3, $4, $5, $6, true)
				RETURNING workflow_id
				"
			)
		};

		let (actual_workflow_id,) = self
			.query(|| async {
				sql_fetch_one!(
					[self, (Uuid,)]
					query,
					workflow_id,
					workflow_name,
					rivet_util::timestamp::now(),
					ray_id,
					tags,
					sqlx::types::Json(input),
				)
				.await
			})
			.await?;

		if workflow_id == actual_workflow_id {
			self.wake_worker();
		}

		Ok(actual_workflow_id)
	}

	#[tracing::instrument(skip_all, fields(%workflow_id))]
	async fn get_workflow(&self, workflow_id: Uuid) -> WorkflowResult<Option<WorkflowData>> {
		sql_fetch_optional!(
			[self, WorkflowRow]
			"
			SELECT
				workflow_id,
				input,
				output,
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				) AS has_wake_condition
			FROM db_workflow.workflows
			WHERE workflow_id = $1
			",
			workflow_id,
		)
		.await
		.map(|row| row.map(Into::into))
	}

	#[tracing::instrument(skip_all, fields(%workflow_name))]
	async fn find_workflow(
		&self,
		workflow_name: &str,
		_tags: &serde_json::Value,
	) -> WorkflowResult<Option<Uuid>> {
		unimplemented!();
	}

	#[tracing::instrument(skip_all)]
	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflowData>> {
		let start_instant = Instant::now();

		// Select all workflows that have a wake condition
		let workflow_rows = self
			.query(|| async {
				sql_fetch_all!(
					[self, PulledWorkflowRow]
					"
					WITH select_pending_workflows AS (
						SELECT workflow_id
						FROM db_workflow.workflows@workflows_pred_standard
						WHERE
							-- Filter
							workflow_name = ANY($2) AND
							-- Not already complete
							output IS NULL AND
							-- No assigned node (not running)
							worker_instance_id IS NULL AND
							-- Not silenced
							silence_ts IS NULL AND
							-- Check for wake condition
							(
								-- Immediate
								wake_immediate OR
								-- After deadline
								(
									wake_deadline_ts IS NOT NULL AND
									$3 > wake_deadline_ts - $4
								)
							)
						UNION
						SELECT workflow_id
						FROM db_workflow.workflows@workflows_pred_signals AS w
						WHERE
							-- Filter
							workflow_name = ANY($2) AND
							-- Not already complete
							output IS NULL AND
							-- No assigned node (not running)
							worker_instance_id IS NULL AND
							-- Not silenced
							silence_ts IS NULL AND
							-- Has signals to listen to
							array_length(wake_signals, 1) != 0 AND
							-- Signal exists
							(
								SELECT true
								FROM db_workflow.signals@signals_partial AS s
								WHERE
									s.workflow_id = w.workflow_id AND
									s.signal_name = ANY(w.wake_signals) AND
									s.ack_ts IS NULL AND
									s.silence_ts IS NULL
								LIMIT 1
							)
						UNION
						SELECT workflow_id
						FROM db_workflow.workflows@workflows_pred_signals AS w
						WHERE
							-- Filter
							workflow_name = ANY($2) AND
							-- Not already complete
							output IS NULL AND
							-- No assigned node (not running)
							worker_instance_id IS NULL AND
							-- Not silenced
							silence_ts IS NULL AND
							-- Has signals to listen to
							array_length(wake_signals, 1) != 0 AND
							-- Tagged signal exists
							(
								SELECT true
								FROM db_workflow.tagged_signals@tagged_signals_partial AS s
								WHERE
									s.signal_name = ANY(w.wake_signals) AND
									s.tags <@ w.tags AND
									s.ack_ts IS NULL AND
									s.silence_ts IS NULL
								LIMIT 1
							)
						UNION
						SELECT workflow_id
						FROM db_workflow.workflows@workflows_pred_sub_workflow AS w
						WHERE
							-- Filter
							workflow_name = ANY($2) AND
							-- Not already complete
							output IS NULL AND
							-- No assigned node (not running)
							worker_instance_id IS NULL AND
							-- Not silenced
							silence_ts IS NULL AND
							wake_sub_workflow_id IS NOT NULL AND
							-- Sub workflow completed
							(
								SELECT true
								FROM db_workflow.workflows@workflows_pred_sub_workflow_internal AS w2
								WHERE
									w2.workflow_id = w.wake_sub_workflow_id AND
									output IS NOT NULL
							)
						LIMIT $5
					)
					UPDATE db_workflow.workflows@workflows_pkey AS w
					-- Assign current node to this workflow
					SET
						worker_instance_id = $1,
						last_pull_ts = $3
					FROM select_pending_workflows AS pw
					WHERE w.workflow_id = pw.workflow_id
					RETURNING w.workflow_id, workflow_name, create_ts, ray_id, input, wake_deadline_ts
					",
					worker_instance_id,
					filter,
					rivet_util::timestamp::now(),
					// Add padding to the tick interval so that the workflow deadline is never passed before its pulled.
					// The worker sleeps internally to handle this
					self.worker_poll_interval().as_millis() as i64 + 1,
					MAX_PULLED_WORKFLOWS,
				)
				.await
			})
			.await?;

		let worker_instance_id_str = worker_instance_id.to_string();
		let dt = start_instant.elapsed().as_secs_f64();
		metrics::LAST_PULL_WORKFLOWS_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.set(dt);
		metrics::PULL_WORKFLOWS_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.observe(dt);

		if workflow_rows.is_empty() {
			return Ok(Vec::new());
		}

		let workflow_ids = workflow_rows
			.iter()
			.map(|row| row.workflow_id)
			.collect::<Vec<_>>();

		let start_instant2 = Instant::now();

		// Fetch all events for all fetched workflows
		let events = sql_fetch_all!(
			[self, AmalgamEventRow]
			"
			-- Activity events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				0 AS event_type, -- EventType
				activity_name AS name,
				NULL AS auxiliary_id,
				input_hash AS hash,
				NULL AS input,
				output AS output,
				create_ts AS create_ts,
				(
					SELECT COUNT(*)
					FROM db_workflow.workflow_activity_errors AS err
					WHERE
						ev.workflow_id = err.workflow_id AND
						ev.location2 = err.location2
				) AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_activity_events AS ev
			WHERE ev.workflow_id = ANY($1) AND forgotten = FALSE
			-- Should only require `workflow_id` and `location2` but because `location2` is nullable the
			-- database can't determine uniqueness
			GROUP BY
				ev.workflow_id,
				ev.location,
				ev.location2,
				ev.version,
				ev.activity_name,
				ev.input_hash,
				ev.output,
				ev.create_ts
			UNION ALL
			-- Signal listen events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				1 AS event_type, -- EventType
				signal_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				body AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_signal_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Signal send events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				2 AS event_type, -- EventType
				signal_name AS name,
				signal_id AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_signal_send_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Message send events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				3 AS event_type, -- EventType
				message_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_message_send_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Sub workflow events
			SELECT
				sw.workflow_id,
				sw.location,
				sw.location2,
				version,
				4 AS event_type, -- crdb_nats::types::EventType
				w.workflow_name AS name,
				sw.sub_workflow_id AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_sub_workflow_events AS sw
			JOIN db_workflow.workflows AS w
			ON sw.sub_workflow_id = w.workflow_id
			WHERE sw.workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Loop events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				5 AS event_type, -- crdb_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				state AS input,
				output,
				NULL AS create_ts,
				NULL AS error_count,
				iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_loop_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Sleep events
			SELECT
				workflow_id,
				location,
				location2,
				version,
				6 AS event_type, -- crdb_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				deadline_ts,
				state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_sleep_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Branch events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				version,
				7 AS event_type, -- crdb_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_branch_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Removed events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				1 AS version, -- Default
				8 AS event_type, -- crdb_nats::types::EventType
				event_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				event_type AS inner_event_type
			FROM db_workflow.workflow_removed_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			UNION ALL
			-- Version check events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				version,
				9 AS event_type, -- crdb_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS input,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_version_check_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			ORDER BY workflow_id ASC, location2 ASC
			",
			&workflow_ids,
		)
		.await?;

		let workflows = build_histories(workflow_rows, events)?;

		let dt2 = start_instant2.elapsed().as_secs_f64();
		let dt = start_instant.elapsed().as_secs_f64();
		metrics::LAST_PULL_WORKFLOWS_FULL_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.set(dt);
		metrics::PULL_WORKFLOWS_FULL_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.observe(dt);
		metrics::LAST_PULL_WORKFLOWS_HISTORY_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.set(dt2);
		metrics::PULL_WORKFLOWS_HISTORY_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.observe(dt2);

		Ok(workflows)
	}

	#[tracing::instrument(skip_all)]
	async fn complete_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		self.query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE db_workflow.workflows
				SET output = $2
				WHERE workflow_id = $1
				",
			))
			.bind(workflow_id)
			.bind(sqlx::types::Json(output))
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::COMPLETE_WORKFLOW_DURATION
			.with_label_values(&[workflow_name])
			.observe(dt);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow_id: Option<Uuid>,
		error: &str,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		self.query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE db_workflow.workflows
				SET
					worker_instance_id = NULL,
					wake_immediate = $2,
					wake_deadline_ts = $3,
					wake_signals = $4,
					wake_sub_workflow_id = $5,
					error = $6
				WHERE workflow_id = $1
				",
			))
			.bind(workflow_id)
			.bind(immediate)
			.bind(wake_deadline_ts)
			.bind(wake_signals)
			.bind(wake_sub_workflow_id)
			.bind(error)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		// Wake worker again if the deadline is before the next tick
		if let Some(deadline_ts) = wake_deadline_ts {
			if deadline_ts
				<= rivet_util::timestamp::now() + self.worker_poll_interval().as_millis() as i64
			{
				self.wake_worker();
			}
		}

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::COMMIT_WORKFLOW_DURATION
			.with_label_values(&[workflow_name])
			.observe(dt);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn pull_next_signal(
		&self,
		workflow_id: Uuid,
		_workflow_name: &str,
		filter: &[&str],
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
		_last_try: bool,
	) -> WorkflowResult<Option<SignalData>> {
		let signal = self
			.query(|| async {
				sql_fetch_optional!(
					[self, SignalRow]
					"
					WITH
						-- Finds the oldest signal matching the signal name filter in either the normal signals table
						-- or tagged signals table
						next_signal AS (
							SELECT false AS tagged, signal_id, create_ts, signal_name, body
							FROM db_workflow.signals@signals_partial
							WHERE
								workflow_id = $1 AND
								signal_name = ANY($2) AND
								ack_ts IS NULL AND
								silence_ts IS NULL
							UNION ALL
							SELECT true AS tagged, signal_id, s.create_ts, signal_name, body
							FROM db_workflow.tagged_signals@tagged_signals_partial AS s
							JOIN db_workflow.workflows AS w
							ON s.tags <@ w.tags
							WHERE
								w.workflow_id = $1 AND
								s.signal_name = ANY($2) AND
								s.ack_ts IS NULL AND
								s.silence_ts IS NULL
							ORDER BY create_ts ASC
							LIMIT 1
						),
						-- If the next signal is not tagged, acknowledge it with this statement
						ack_signal AS (
							UPDATE db_workflow.signals
							SET ack_ts = $5
							WHERE signal_id = (
								SELECT signal_id
								FROM next_signal
								WHERE tagged = false
							)
							RETURNING 1
						),
						-- If the next signal is tagged, acknowledge it with this statement
						ack_tagged_signal AS (
							UPDATE db_workflow.tagged_signals
							SET ack_ts = $5
							WHERE signal_id = (
								SELECT signal_id
								FROM next_signal
								WHERE tagged = true
							)
							RETURNING 1
						),
						-- After acking the signal, add it to the events table
						insert_event AS (
							INSERT INTO db_workflow.workflow_signal_events (
								workflow_id,
								location2,
								version,
								signal_id,
								signal_name,
								body,
								ack_ts,
								loop_location2
							)
							SELECT
								$1 AS workflow_id,
								$3 AS location2,
								$4 AS version,
								signal_id,
								signal_name,
								body,
								$5 AS ack_ts,
								$6 AS loop_location2
							FROM next_signal
							RETURNING 1
						)
					SELECT * FROM next_signal
					",
					workflow_id,
					filter,
					location,
					version as i64,
					rivet_util::timestamp::now(),
					loop_location,
				)
				.await
				.map(|row| row.map(Into::into))
			})
			.await?;

		Ok(signal)
	}

	#[tracing::instrument(skip_all)]
	async fn get_sub_workflow(
		&self,
		_workflow_id: Uuid,
		_workflow_name: &str,
		sub_workflow_id: Uuid,
	) -> WorkflowResult<Option<WorkflowData>> {
		self.get_workflow(sub_workflow_id).await
	}

	#[tracing::instrument(skip_all)]
	async fn publish_signal(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sql_execute!(
				[self]
				"
				INSERT INTO db_workflow.signals (
					signal_id, workflow_id, signal_name, body, ray_id, create_ts
				)			
				VALUES ($1, $2, $3, $4, $5, $6)
				",
				signal_id,
				workflow_id,
				signal_name,
				sqlx::types::Json(body),
				ray_id,
				rivet_util::timestamp::now(),
			)
			.await
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_tagged_signal(
		&self,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sql_execute!(
				[self]
				"
				INSERT INTO db_workflow.tagged_signals (
					signal_id, tags, signal_name, body, ray_id, create_ts
				)			
				VALUES ($1, $2, $3, $4, $5, $6)
				",
				signal_id,
				tags,
				signal_name,
				sqlx::types::Json(body),
				ray_id,
				rivet_util::timestamp::now(),
			)
			.await
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		ray_id: Uuid,
		to_workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sql_execute!(
				[self]
				"
				WITH
					signal AS (
						INSERT INTO db_workflow.signals (
							signal_id, workflow_id, signal_name, body, ray_id, create_ts
						)			
						VALUES ($1, $2, $3, $4, $5, $6)
						RETURNING 1
					),
					send_event AS (
						INSERT INTO db_workflow.workflow_signal_send_events(
							workflow_id, location2, version, signal_id, signal_name, body, loop_location2
						)
						VALUES($7, $8, $9, $1, $3, $4, $10)
						RETURNING 1
					)
				SELECT 1
				",
				signal_id,
				to_workflow_id,
				signal_name,
				sqlx::types::Json(body),
				ray_id,
				rivet_util::timestamp::now(),
				from_workflow_id,
				location,
				version as i64,
				loop_location,
			)
			.await
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_tagged_signal_from_workflow(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sql_execute!(
				[self]
				"
				WITH
					signal AS (
						INSERT INTO db_workflow.tagged_signals (
							signal_id, tags, signal_name, body, ray_id, create_ts
						)			
						VALUES ($1, $2, $3, $4, $5, $6)
						RETURNING 1
					),
					send_event AS (
						INSERT INTO db_workflow.workflow_signal_send_events (
							workflow_id, location2, version, signal_id, signal_name, body, loop_location2
						)
						VALUES($7, $8, $9, $1, $3, $4, $10)
						RETURNING 1
					)
				SELECT 1
				",
				signal_id,
				tags,
				signal_name,
				sqlx::types::Json(body),
				ray_id,
				rivet_util::timestamp::now(),
				from_workflow_id,
				location,
				version as i64,
				loop_location,
			)
			.await
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(%sub_workflow_id, %sub_workflow_name, unique))]
	async fn dispatch_sub_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
		unique: bool,
	) -> WorkflowResult<Uuid> {
		let query = if unique {
			// Check if an incomplete workflow with the given name and tags already exists
			indoc!(
				"
				WITH
					select_existing AS (
						SELECT workflow_id, input, output
						FROM db_workflow.workflows
						WHERE
							workflow_name = $2 AND
							tags <@ $5 AND
							output IS NULL
						LIMIT 1
					),
					insert_workflow AS (
						INSERT INTO db_workflow.workflows (
							workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
						)
						SELECT $9, $2, $3, $4, $5, $6, true
						WHERE NOT EXISTS(SELECT 1 FROM select_existing)
						RETURNING workflow_id
					),
					insert_sub_workflow_event AS (
						INSERT INTO db_workflow.workflow_sub_workflow_events (
							workflow_id, location2, version, sub_workflow_id, create_ts, loop_location2
						)
						SELECT $1, $7, $8, $9, $3, $10
						WHERE NOT EXISTS(SELECT 1 FROM select_existing)
						RETURNING 1
					)
				SELECT COALESCE(s.workflow_id, i.workflow_id)
				FROM select_existing AS s
				FULL OUTER JOIN insert_workflow AS i
				ON TRUE
				",
			)
		} else {
			indoc!(
				"
				WITH
					insert_workflow AS (
						INSERT INTO db_workflow.workflows (
							workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
						)
						VALUES ($9, $2, $3, $4, $5, $6, true)
						RETURNING workflow_id
					),
					insert_sub_workflow_event AS (
						INSERT INTO db_workflow.workflow_sub_workflow_events (
							workflow_id, location2, version, sub_workflow_id, create_ts, loop_location2
						)
						VALUES($1, $7, $8, $9, $3, $10)
						RETURNING 1
					)
				SELECT workflow_id FROM insert_workflow
				",
			)
		};

		let (actual_sub_workflow_id,) = self
			.query(|| async {
				sqlx::query_as::<_, (Uuid,)>(query)
					.bind(workflow_id)
					.bind(sub_workflow_name)
					.bind(rivet_util::timestamp::now())
					.bind(ray_id)
					.bind(tags)
					.bind(sqlx::types::Json(input))
					.bind(location)
					.bind(version as i64)
					.bind(sub_workflow_id)
					.bind(loop_location)
					.fetch_one(&mut *self.conn().await?)
					.await
					.map_err(WorkflowError::Sqlx)
			})
			.await?;

		if sub_workflow_id == actual_sub_workflow_id {
			self.wake_worker();
		}

		Ok(actual_sub_workflow_id)
	}

	#[tracing::instrument(skip_all)]
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		_workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE db_workflow.workflows
				SET tags = $2
				WHERE workflow_id = $1
				",
			))
			.bind(workflow_id)
			.bind(tags)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_activity_event(
		&self,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		event_id: &EventId,
		create_ts: i64,
		input: &serde_json::value::RawValue,
		res: Result<&serde_json::value::RawValue, &str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		match res {
			Ok(output) => {
				self.query(|| async {
					sqlx::query(indoc!(
						"
						INSERT INTO db_workflow.workflow_activity_events (
							workflow_id,
							location2,
							version,
							activity_name,
							input_hash,
							input,
							output,
							create_ts,
							loop_location2
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
						ON CONFLICT (workflow_id, location2_hash) DO UPDATE
						SET output = EXCLUDED.output
						",
					))
					.bind(workflow_id)
					.bind(location)
					.bind(version as i64)
					.bind(&event_id.name)
					.bind(event_id.input_hash.to_le_bytes())
					.bind(sqlx::types::Json(input))
					.bind(sqlx::types::Json(output))
					.bind(create_ts)
					.bind(loop_location)
					.execute(&mut *self.conn().await?)
					.await
					.map_err(WorkflowError::Sqlx)
				})
				.await?;
			}
			Err(err) => {
				self.query(|| async {
					sqlx::query(indoc!(
						"
						WITH
							event AS (
								INSERT INTO db_workflow.workflow_activity_events (
									workflow_id,
									location2,
									version,
									activity_name,
									input_hash,
									input,
									create_ts,
									loop_location2
								)
								VALUES ($1, $2, $3, $4, $5, $6, $8, $9)
								ON CONFLICT (workflow_id, location2_hash) DO NOTHING
								RETURNING 1
							),
							err AS (
								INSERT INTO db_workflow.workflow_activity_errors (
									workflow_id, location2, activity_name, error, ts
								)
								VALUES ($1, $2, $4, $7, $10)
								RETURNING 1
							)
						SELECT 1
						",
					))
					.bind(workflow_id)
					.bind(location)
					.bind(version as i64)
					.bind(&event_id.name)
					.bind(event_id.input_hash.to_le_bytes())
					.bind(sqlx::types::Json(input))
					.bind(err)
					.bind(create_ts)
					.bind(loop_location)
					.bind(rivet_util::timestamp::now())
					.execute(&mut *self.conn().await?)
					.await
					.map_err(WorkflowError::Sqlx)
				})
				.await?;
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_message_send_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		tags: &serde_json::Value,
		message_name: &str,
		body: &serde_json::value::RawValue,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_message_send_events (
					workflow_id, location2, version, tags, message_name, body, loop_location2
				)
				VALUES($1, $2, $3, $4, $5, $6, $7)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(tags)
			.bind(message_name)
			.bind(sqlx::types::Json(body))
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn upsert_workflow_loop_event(
		&self,
		workflow_id: Uuid,
		_workflow_name: &str,
		location: &Location,
		version: usize,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			let mut conn = self.conn().await?;
			let mut tx = conn.begin().await.map_err(WorkflowError::Sqlx)?;

			sql_execute!(
				[self, @tx &mut tx]
				"
				INSERT INTO db_workflow.workflow_loop_events (
					workflow_id,
					location2,
					version,
					iteration,
					state,
					output,
					loop_location2
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7)
				ON CONFLICT (workflow_id, location2_hash) DO UPDATE
				SET
					iteration = $4,
					state = $5,
					output = $6
				RETURNING 1
				",
				workflow_id,
				location,
				version as i64,
				iteration as i64,
				sqlx::types::Json(state),
				output.map(sqlx::types::Json),
				loop_location,
			)
			.await?;

			// 0-th iteration is the initial insertion
			if iteration != 0 {
				sql_execute!(
					[self, @tx &mut tx]
					"
					WITH
						forget_activity_events AS (
							UPDATE db_workflow.workflow_activity_events@workflow_activity_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_signal_events AS (
							UPDATE db_workflow.workflow_signal_events@workflow_signal_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_sub_workflow_events AS (
							UPDATE db_workflow.workflow_sub_workflow_events@workflow_sub_workflow_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_signal_send_events AS (
							UPDATE db_workflow.workflow_signal_send_events@workflow_signal_send_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_message_send_events AS (
							UPDATE db_workflow.workflow_message_send_events@workflow_message_send_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_loop_events AS (
							UPDATE db_workflow.workflow_loop_events@workflow_loop_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_sleep_events AS (
							UPDATE db_workflow.workflow_sleep_events@workflow_sleep_events_workflow_id_loop_location2_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location2_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_branch_events AS (
							UPDATE db_workflow.workflow_branch_events@workflow_branch_events_workflow_id_loop_location_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_removed_events AS (
							UPDATE db_workflow.workflow_removed_events@workflow_removed_events_workflow_id_loop_location_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						),
						forget_version_check_events AS (
							UPDATE db_workflow.workflow_version_check_events@workflow_version_check_events_workflow_id_loop_location_hash_idx
							SET forgotten = TRUE
							WHERE
								workflow_id = $1 AND
								loop_location_hash = $2 AND
								forgotten = FALSE
							RETURNING 1
						)
					SELECT 1
					",
					workflow_id,
					hash_location(location),
				)
				.await?;
			}

			tx.commit().await.map_err(WorkflowError::Sqlx)?;

			Ok(())
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_sleep_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		deadline_ts: i64,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_sleep_events (
					workflow_id, location2, version, deadline_ts, loop_location2, state
				)
				VALUES($1, $2, $3, $4, $5, $6)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(deadline_ts)
			.bind(loop_location)
			.bind(SleepState::Normal as i64)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn update_workflow_sleep_event_state(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		state: SleepState,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE db_workflow.workflow_sleep_events
				SET state = $3
				WHERE workflow_id = $1 AND location2 = $2
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(state as i64)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_branch_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_branch_events (
					workflow_id, location, version, loop_location
				)
				VALUES($1, $2, $3, $4)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_removed_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		event_type: EventType,
		event_name: Option<&str>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_removed_events (
					workflow_id, location, event_type, event_name, loop_location
				)
				VALUES($1, $2, $3, $4, $5)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(event_type as i32)
			.bind(event_name)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_version_check_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_version_check_events (
					workflow_id, location, version, loop_location
				)
				VALUES($1, $2, $3, $4)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}
}
