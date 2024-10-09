//! Implementation of a workflow database driver with PostgreSQL (CockroachDB) and NATS.

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use indoc::indoc;
use rivet_pools::prelude::NatsPool;
use sqlx::{pool::PoolConnection, Acquire, PgPool, Postgres};
use tokio::sync::Mutex;
use tracing::Instrument;
use uuid::Uuid;

use super::{Database, PulledWorkflow, SignalData, WorkflowData};
use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{EventId, EventType, SleepState},
		location::Location,
	},
	message, worker,
};

/// Max amount of workflows pulled from the database with each call to `pull_workflows`.
const MAX_PULLED_WORKFLOWS: i64 = 50;
// Base retry for query retry backoff
const QUERY_RETRY_MS: usize = 750;
// Time in between transaction retries
const TXN_RETRY: Duration = Duration::from_millis(100);
/// Maximum times a query ran by this database adapter is retried.
const MAX_QUERY_RETRIES: usize = 16;

pub struct DatabasePgNats {
	pool: PgPool,
	nats: NatsPool,
	sub: Mutex<Option<rivet_pools::prelude::nats::Subscriber>>,
}

impl DatabasePgNats {
	pub fn from_pools(pool: PgPool, nats: NatsPool) -> Arc<DatabasePgNats> {
		Arc::new(DatabasePgNats {
			pool,
			// Lazy load the nats sub
			sub: Mutex::new(None),
			nats,
		})
	}

	async fn conn(&self) -> WorkflowResult<PoolConnection<Postgres>> {
		// Attempt to use an existing connection
		if let Some(conn) = self.pool.try_acquire() {
			Ok(conn)
		} else {
			// Create a new connection
			self.pool.acquire().await.map_err(WorkflowError::Sqlx)
		}
	}

	/// Spawns a new thread and publishes a worker wake message to nats.
	fn wake_worker(&self) {
		let nats = self.nats.clone();

		let spawn_res = tokio::task::Builder::new().name("wake").spawn(
			async move {
				// Fail gracefully
				if let Err(err) = nats
					.publish(message::WORKER_WAKE_SUBJECT, Vec::new().into())
					.await
				{
					tracing::warn!(?err, "failed to publish wake message");
				}
			}
			.in_current_span(),
		);
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn wake task");
		}
	}

	/// Executes queries and explicitly handles retry errors.
	async fn query<'a, F, Fut, T>(&self, mut cb: F) -> WorkflowResult<T>
	where
		F: FnMut() -> Fut,
		Fut: std::future::Future<Output = WorkflowResult<T>> + 'a,
		T: 'a,
	{
		let mut backoff = rivet_util::Backoff::new(3, None, QUERY_RETRY_MS, 50);
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
							tracing::info!(message=%db_err.message(), "transaction retry");
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
impl Database for DatabasePgNats {
	async fn wake(&self) -> WorkflowResult<()> {
		let mut sub = self.sub.try_lock().map_err(WorkflowError::WakeLock)?;

		// Initialize sub
		if sub.is_none() {
			*sub = Some(
				self.nats
					.subscribe(message::WORKER_WAKE_SUBJECT)
					.await
					.map_err(|x| WorkflowError::CreateSubscription(x.into()))?,
			);
		}

		match sub.as_mut().expect("unreachable").next().await {
			Some(_) => Ok(()),
			None => Err(WorkflowError::SubscriptionUnsubscribed),
		}
	}

	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflows (
					workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
				)
				VALUES ($1, $2, $3, $4, $5, $6, true)
				",
			))
			.bind(workflow_id)
			.bind(workflow_name)
			.bind(rivet_util::timestamp::now())
			.bind(ray_id)
			.bind(&tags)
			.bind(sqlx::types::Json(input))
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	async fn get_workflow(&self, workflow_id: Uuid) -> WorkflowResult<Option<WorkflowData>> {
		sqlx::query_as::<_, WorkflowRow>(indoc!(
			"
			SELECT workflow_id, input, output
			FROM db_workflow.workflows
			WHERE workflow_id = $1
			",
		))
		.bind(workflow_id)
		.fetch_optional(&mut *self.conn().await?)
		.await
		.map(|row| row.map(Into::into))
		.map_err(WorkflowError::Sqlx)
	}

	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflow>> {
		// Select all workflows that have a wake condition
		let workflow_rows = self
			.query(|| async {
				sqlx::query_as::<_, PulledWorkflowRow>(indoc!(
					"
					WITH
						pull_workflows AS (
							WITH select_pending_workflows AS (
								SELECT workflow_id
								FROM db_workflow.workflows
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
								FROM db_workflow.workflows AS w
								WHERE
									-- Filter
									workflow_name = ANY($2) AND
									-- Not already complete
									output IS NULL AND
									-- No assigned node (not running)
									worker_instance_id IS NULL AND
									-- Not silenced
									silence_ts IS NULL AND
									-- Signal exists
									(
										SELECT true
										FROM db_workflow.signals AS s
										WHERE
											s.workflow_id = w.workflow_id AND
											s.signal_name = ANY(w.wake_signals) AND
											s.ack_ts IS NULL AND
											silence_ts IS NULL
										LIMIT 1
									)
								UNION
								SELECT workflow_id
								FROM db_workflow.workflows AS w
								WHERE
									-- Filter
									workflow_name = ANY($2) AND
									-- Not already complete
									output IS NULL AND
									-- No assigned node (not running)
									worker_instance_id IS NULL AND
									-- Not silenced
									silence_ts IS NULL AND
									-- Tagged signal exists
									(
										SELECT true
										FROM db_workflow.tagged_signals AS s
										WHERE
											s.signal_name = ANY(w.wake_signals) AND
											s.tags <@ w.tags AND
											s.ack_ts IS NULL AND
											s.silence_ts IS NULL
										LIMIT 1
									)
								UNION
								SELECT workflow_id
								FROM db_workflow.workflows AS w
								WHERE
									-- Filter
									workflow_name = ANY($2) AND
									-- Not already complete
									output IS NULL AND
									-- No assigned node (not running)
									worker_instance_id IS NULL AND
									-- Not silenced
									silence_ts IS NULL AND
									-- Sub workflow completed
									(
										SELECT true
										FROM db_workflow.workflows AS w2
										WHERE
											w2.workflow_id = w.wake_sub_workflow_id AND
											output IS NOT NULL
									)
								LIMIT $5
							)
							UPDATE db_workflow.workflows AS w
							-- Assign current node to this workflow
							SET worker_instance_id = $1
							FROM select_pending_workflows AS pw
							WHERE w.workflow_id = pw.workflow_id
							RETURNING w.workflow_id, workflow_name, create_ts, ray_id, input, wake_deadline_ts
						),
						-- Update last ping
						worker_instance_update AS (
							UPSERT INTO db_workflow.worker_instances (worker_instance_id, last_ping_ts)
							VALUES ($1, $3)
							RETURNING 1
						)
					SELECT * FROM pull_workflows
					",
				))
				.bind(worker_instance_id)
				.bind(filter)
				.bind(rivet_util::timestamp::now())
				// Add padding to the tick interval so that the workflow deadline is never passed before its pulled.
				// The worker sleeps internally to handle this
				.bind(worker::TICK_INTERVAL.as_millis() as i64 + 1)
				.bind(MAX_PULLED_WORKFLOWS)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			})
			.await?;

		if workflow_rows.is_empty() {
			return Ok(Vec::new());
		}

		let workflow_ids = workflow_rows
			.iter()
			.map(|row| row.workflow_id)
			.collect::<Vec<_>>();

		// TODO: Convert into union query
		// Fetch all events for all fetched workflows
		let events = sqlx::query_as::<_, AmalgamEventRow>(indoc!(
			"
			-- Activity events
			SELECT
				ev.workflow_id,
				ev.location,
				ev.location2,
				0 AS event_type, -- EventType
				ev.activity_name AS name,
				NULL AS auxiliary_id,
				ev.input_hash AS hash,
				ev.output AS output,
				ev.create_ts AS create_ts,
				COUNT(err.*) AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_activity_events AS ev
			LEFT JOIN db_workflow.workflow_activity_errors AS err
			ON
				ev.workflow_id = err.workflow_id AND
				ev.location2_hash = err.location2_hash
			WHERE ev.workflow_id = ANY($1) AND forgotten = FALSE
			GROUP BY ev.workflow_id, ev.location2
			UNION ALL
			-- Signal listen events
			SELECT
				workflow_id,
				location,
				location2,
				1 AS event_type, -- EventType
				signal_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
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
				2 AS event_type, -- EventType
				signal_name AS name,
				signal_id AS auxiliary_id,
				NULL AS hash,
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
				3 AS event_type, -- EventType
				message_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
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
				4 AS event_type, -- pg_nats::types::EventType
				w.workflow_name AS name,
				sw.sub_workflow_id AS auxiliary_id,
				NULL AS hash,
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
				5 AS event_type, -- pg_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
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
				6 AS event_type, -- pg_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				deadline_ts,
				state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_sleep_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			-- Branch events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				7 AS event_type, -- pg_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_sleep_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			-- Removed events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				8 AS event_type, -- pg_nats::types::EventType
				event_name AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				event_type AS inner_event_type
			FROM db_workflow.workflow_sleep_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			-- Version check events
			SELECT
				workflow_id,
				ARRAY[] AS location,
				location AS location2,
				9 AS event_type, -- pg_nats::types::EventType
				NULL AS name,
				NULL AS auxiliary_id,
				NULL AS hash,
				NULL AS output,
				NULL AS create_ts,
				NULL AS error_count,
				NULL AS iteration,
				NULL AS deadline_ts,
				NULL AS state,
				NULL AS inner_event_type
			FROM db_workflow.workflow_sleep_events
			WHERE workflow_id = ANY($1) AND forgotten = FALSE
			-- We don't order by location2 because it is a JSONB type (probably inefficient)
			ORDER BY workflow_id ASC;
			",
		))
		.bind(&workflow_ids)
		.fetch_all(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

		let workflows = build_histories(workflow_rows, events)?;

		Ok(workflows)
	}

	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
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

		Ok(())
	}

	async fn fail_workflow(
		&self,
		workflow_id: Uuid,
		immediate: bool,
		deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow_id: Option<Uuid>,
		error: &str,
	) -> WorkflowResult<()> {
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
			.bind(deadline_ts)
			.bind(wake_signals)
			.bind(wake_sub_workflow_id)
			.bind(error)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		// Wake worker again if the deadline is before the next tick
		if let Some(deadline_ts) = deadline_ts {
			if deadline_ts
				< rivet_util::timestamp::now() + worker::TICK_INTERVAL.as_millis() as i64 + 1
			{
				self.wake_worker();
			}
		}

		Ok(())
	}

	// TODO: Theres nothing preventing this from being able to be called from the workflow ctx also, but for
	// now its only in the activity ctx so it isn't called again during workflow retries
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
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
						ON CONFLICT (workflow_id, location2) DO UPDATE
						SET output = excluded.output
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
									activity_name,
									input_hash,
									input,
									create_ts,
									loop_location2
								)
								VALUES ($1, $2, $3, $4, $5, $7, $8)
								ON CONFLICT (workflow_id, location2) DO NOTHING
								RETURNING 1
							),
							err AS (
								INSERT INTO db_workflow.workflow_activity_errors (
									workflow_id, location2, activity_name, error, ts
								)
								VALUES ($1, $2, $3, $6, $9)
								RETURNING 1
							)
						SELECT 1
						",
					))
					.bind(workflow_id)
					.bind(location)
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

	async fn pull_next_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &Location,
		version: usize,
		loop_location: Option<&Location>,
	) -> WorkflowResult<Option<SignalData>> {
		let signal = self
			.query(|| async {
				sqlx::query_as::<_, SignalRow>(indoc!(
					"
					WITH
						-- Finds the oldest signal matching the signal name filter in either the normal signals table
						-- or tagged signals table
						next_signal AS (
							SELECT false AS tagged, signal_id, create_ts, signal_name, body
							FROM db_workflow.signals
							WHERE
								workflow_id = $1 AND
								signal_name = ANY($2) AND
								ack_ts IS NULL AND
								silence_ts IS NULL
							UNION ALL
							SELECT true AS tagged, signal_id, create_ts, signal_name, body
							FROM db_workflow.tagged_signals
							WHERE
								signal_name = ANY($2) AND
								tags <@ (SELECT tags FROM db_workflow.workflows WHERE workflow_id = $1) AND
								ack_ts IS NULL AND
								silence_ts IS NULL
							ORDER BY create_ts ASC
							LIMIT 1
						),
						-- If the next signal is not tagged, acknowledge it with this statement
						ack_signal AS (
							UPDATE db_workflow.signals
							SET ack_ts = $5
							WHERE signal_id = (
								SELECT signal_id FROM next_signal WHERE tagged = false
							)
							RETURNING 1
						),
						-- If the next signal is tagged, acknowledge it with this statement
						ack_tagged_signal AS (
							UPDATE db_workflow.tagged_signals
							SET ack_ts = $5
							WHERE signal_id = (
								SELECT signal_id FROM next_signal WHERE tagged = true
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
				))
				.bind(workflow_id)
				.bind(filter)
				.bind(location)
				.bind(version as i64)
				.bind(rivet_util::timestamp::now())
				.bind(loop_location)
				.fetch_optional(&mut *self.conn().await?)
				.await
				.map(|row| row.map(Into::into))
				.map_err(WorkflowError::Sqlx)
			})
			.await?;

		Ok(signal)
	}

	async fn publish_signal(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.signals (signal_id, workflow_id, signal_name, body, ray_id, create_ts)			
				VALUES ($1, $2, $3, $4, $5, $6)
				",
			))
			.bind(signal_id)
			.bind(workflow_id)
			.bind(signal_name)
			.bind(sqlx::types::Json(body))
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	async fn publish_tagged_signal(
		&self,
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.tagged_signals (signal_id, tags, signal_name, body, ray_id, create_ts)			
				VALUES ($1, $2, $3, $4, $5, $6)
				",
			))
			.bind(signal_id)
			.bind(tags)
			.bind(signal_name)
			.bind(sqlx::types::Json(body))
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

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
			sqlx::query(indoc!(
				"
				WITH
					signal AS (
						INSERT INTO db_workflow.signals (signal_id, workflow_id, signal_name, body, ray_id, create_ts)			
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
			))
			.bind(signal_id)
			.bind(to_workflow_id)
			.bind(signal_name)
			.bind(sqlx::types::Json(body))
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

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
			sqlx::query(indoc!(
				"
				WITH
					signal AS (
						INSERT INTO db_workflow.tagged_signals (signal_id, tags, signal_name, body, ray_id, create_ts)			
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
			))
			.bind(signal_id)
			.bind(tags)
			.bind(signal_name)
			.bind(sqlx::types::Json(body))
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.bind(from_workflow_id)
			.bind(location)
			.bind(version as i64)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

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
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				WITH
					workflow AS (
						INSERT INTO db_workflow.workflows (
							workflow_id, workflow_name, create_ts, ray_id, tags, input, wake_immediate
						)
						VALUES ($8, $2, $3, $4, $5, $6, true)
						RETURNING 1
					),
					sub_workflow AS (
						INSERT INTO db_workflow.workflow_sub_workflow_events(
							workflow_id, location2, version, sub_workflow_id, create_ts, loop_location2
						)
						VALUES($1, $7, $8, $9, $3, $10)
						RETURNING 1
					)
				SELECT 1
				",
			))
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
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	async fn poll_workflow(
		&self,
		workflow_name: &str,
		input: &serde_json::value::RawValue,
		after_ts: i64,
	) -> WorkflowResult<Option<(Uuid, i64)>> {
		sqlx::query_as::<_, (Uuid, i64)>(indoc!(
			"
			SELECT workflow_id, create_ts
			FROM db_workflow.workflows
			WHERE
				workflow_name = $1 AND
				-- Subset
				input @> $2 AND
				create_ts >= $3
			",
		))
		.bind(workflow_name)
		.bind(sqlx::types::Json(input))
		.bind(after_ts)
		.fetch_optional(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)
	}

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
				INSERT INTO db_workflow.workflow_message_send_events(
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

	async fn upsert_loop(
		&self,
		workflow_id: Uuid,
		location: &Location,
		version: usize,
		iteration: usize,
		output: Option<&serde_json::value::RawValue>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			let mut conn = self.conn().await?;
			let mut tx = conn.begin().await.map_err(WorkflowError::Sqlx)?;

			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_loop_events (
					workflow_id,
					location2,
					iteration,
					output,
					loop_location2
				)
				VALUES ($1, $2, $3, $4, $5, $6)
				ON CONFLICT (workflow_id, location2) DO UPDATE
				SET
					iteration = $3,
					output = $4
				RETURNING 1
				",
			))
			.bind(workflow_id)
			.bind(&location)
			.bind(version as i64)
			.bind(iteration as i64)
			.bind(sqlx::types::Json(output))
			.bind(loop_location)
			.execute(&mut *tx)
			.await
			.map_err(WorkflowError::Sqlx)?;

			sqlx::query(indoc!(
				"
				WITH
					forget_activity_events AS (
						UPDATE db_workflow.workflow_activity_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_signal_events AS (
						UPDATE db_workflow.workflow_signal_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_sub_workflow_events AS (
						UPDATE db_workflow.workflow_sub_workflow_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_signal_send_events AS (
						UPDATE db_workflow.workflow_signal_send_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_message_send_events AS (
						UPDATE db_workflow.workflow_message_send_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),	
					forget_loop_events AS (
						UPDATE db_workflow.workflow_loop_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location2_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_branch_events AS (
						UPDATE db_workflow.workflow_branch_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_removed_events AS (
						UPDATE db_workflow.workflow_removed_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_version_check_events AS (
						UPDATE db_workflow.workflow_version_check_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location_hash = $2 AND
							forgotten = FALSE
						RETURNING 1
					)
				SELECT 1
				",
			))
			.bind(workflow_id)
			.bind(hash_location(&location)?)
			.execute(&mut *tx)
			.await
			.map_err(WorkflowError::Sqlx)?;

			tx.commit().await.map_err(WorkflowError::Sqlx)?;

			Ok(())
		})
		.await?;

		Ok(())
	}

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
				INSERT INTO db_workflow.workflow_sleep_events(
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
				INSERT INTO db_workflow.workflow_branch_events(
					workflow_id, location, version, loop_location
				)
				VALUES($1, $2, $3)
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
				INSERT INTO db_workflow.workflow_removed_events(
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

	async fn commit_workflow_version_check_event(
		&self,
		from_workflow_id: Uuid,
		location: &Location,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_version_check_events(
					workflow_id, location, loop_location
				)
				VALUES($1, $2, $3)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location)
			.bind(loop_location)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}
}

mod types {
	use std::collections::HashMap;

	use uuid::Uuid;

	use crate::{
		db::{PulledWorkflow, SignalData, WorkflowData},
		error::{WorkflowError, WorkflowResult},
		history::{
			event::{
				ActivityEvent, Event, EventData, EventId, EventType, LoopEvent, MessageSendEvent,
				RemovedEvent, SignalEvent, SignalSendEvent, SleepEvent, SleepState,
				SubWorkflowEvent,
			},
			location::{Coordinate, Location},
		},
	};

	type RawJson = sqlx::types::Json<Box<serde_json::value::RawValue>>;

	#[derive(sqlx::FromRow)]
	pub struct WorkflowRow {
		workflow_id: Uuid,
		input: RawJson,
		output: Option<RawJson>,
	}

	impl From<WorkflowRow> for WorkflowData {
		fn from(value: WorkflowRow) -> Self {
			WorkflowData {
				workflow_id: value.workflow_id,
				input: value.input.0,
				output: value.output.map(|x| x.0),
			}
		}
	}

	#[derive(sqlx::FromRow)]
	pub struct PulledWorkflowRow {
		pub workflow_id: Uuid,
		workflow_name: String,
		create_ts: i64,
		ray_id: Uuid,
		input: RawJson,
		wake_deadline_ts: Option<i64>,
	}

	#[derive(sqlx::FromRow)]
	pub struct SignalRow {
		signal_id: Uuid,
		signal_name: String,
		body: RawJson,
		create_ts: i64,
	}

	impl From<SignalRow> for SignalData {
		fn from(value: SignalRow) -> Self {
			SignalData {
				signal_id: value.signal_id,
				signal_name: value.signal_name,
				body: value.body.0,
				create_ts: value.create_ts,
			}
		}
	}

	/// Stores data for all event types in one.
	#[derive(sqlx::FromRow)]
	pub struct AmalgamEventRow {
		workflow_id: Uuid,
		location: Vec<i64>,
		location2: Option<Location>,
		version: i64,
		event_type: i64,
		name: Option<String>,
		auxiliary_id: Option<Uuid>,
		hash: Option<Vec<u8>>,
		output: Option<RawJson>,
		create_ts: Option<i64>,
		error_count: Option<i64>,
		iteration: Option<i64>,
		deadline_ts: Option<i64>,
		state: Option<i64>,
		inner_event_type: Option<i64>,
	}

	impl TryFrom<AmalgamEventRow> for Event {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			// Backwards compatibility
			let location_tail = value
				.location2
				.as_ref()
				.map(|x| x.tail().cloned().expect("empty location"))
				.unwrap_or_else(|| {
					Coordinate::new(Box::new([
						// NOTE: Add 1 because we switched from 0-based to 1-based
						*value.location.last().expect("empty location") as usize + 1,
					]))
				});

			let event_type = value
				.event_type
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?;
			let event_type = EventType::from_repr(event_type)
				.ok_or_else(|| WorkflowError::InvalidEventType(value.event_type))?;

			Ok(Event {
				coordinate: location_tail,
				version: value
					.version
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
				data: match event_type {
					EventType::Activity => EventData::Activity(value.try_into()?),
					EventType::Signal => EventData::Signal(value.try_into()?),
					EventType::SignalSend => EventData::SignalSend(value.try_into()?),
					EventType::MessageSend => EventData::MessageSend(value.try_into()?),
					EventType::SubWorkflow => EventData::SubWorkflow(value.try_into()?),
					EventType::Loop => EventData::Loop(value.try_into()?),
					EventType::Sleep => EventData::Sleep(value.try_into()?),
					EventType::Branch => EventData::Branch,
					EventType::Removed => EventData::Removed(value.try_into()?),
					EventType::VersionCheck => EventData::VersionCheck,
				},
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for ActivityEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(ActivityEvent {
				event_id: EventId::from_bytes(
					value.name.ok_or(WorkflowError::MissingEventData)?,
					value.hash.ok_or(WorkflowError::MissingEventData)?,
				)?,
				create_ts: value.create_ts.ok_or(WorkflowError::MissingEventData)?,
				output: value.output.map(|x| x.0),
				error_count: value
					.error_count
					.ok_or(WorkflowError::MissingEventData)?
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for SignalEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(SignalEvent {
				name: value.name.ok_or(WorkflowError::MissingEventData)?,
				body: value
					.output
					.map(|x| x.0)
					.ok_or(WorkflowError::MissingEventData)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for SignalSendEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(SignalSendEvent {
				signal_id: value.auxiliary_id.ok_or(WorkflowError::MissingEventData)?,
				name: value.name.ok_or(WorkflowError::MissingEventData)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for MessageSendEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(MessageSendEvent {
				name: value.name.ok_or(WorkflowError::MissingEventData)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for SubWorkflowEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(SubWorkflowEvent {
				sub_workflow_id: value.auxiliary_id.ok_or(WorkflowError::MissingEventData)?,
				name: value.name.ok_or(WorkflowError::MissingEventData)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for LoopEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			Ok(LoopEvent {
				output: value.output.map(|x| x.0),
				iteration: value
					.iteration
					.ok_or(WorkflowError::MissingEventData)?
					.try_into()
					.map_err(|_| WorkflowError::IntegerConversion)?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for SleepEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			let state = value.state.ok_or(WorkflowError::MissingEventData)?;

			Ok(SleepEvent {
				deadline_ts: value.deadline_ts.ok_or(WorkflowError::MissingEventData)?,
				state: SleepState::from_repr(state.try_into()?)
					.ok_or_else(|| WorkflowError::InvalidSleepState(state))?,
			})
		}
	}

	impl TryFrom<AmalgamEventRow> for RemovedEvent {
		type Error = WorkflowError;

		fn try_from(value: AmalgamEventRow) -> WorkflowResult<Self> {
			let event_type = value
				.inner_event_type
				.ok_or(WorkflowError::MissingEventData)?;

			Ok(RemovedEvent {
				name: value.name,
				event_type: EventType::from_repr(event_type.try_into()?)
					.ok_or_else(|| WorkflowError::InvalidSleepState(event_type))?,
			})
		}
	}

	// Implements sqlx postgres types for `Location`
	impl sqlx::Type<sqlx::Postgres> for Location {
		fn type_info() -> sqlx::postgres::PgTypeInfo {
			<i64 as sqlx::postgres::PgHasArrayType>::array_type_info()
		}

		fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
			<i64 as sqlx::postgres::PgHasArrayType>::array_compatible(ty)
		}
	}

	impl<'q> sqlx::Encode<'q, sqlx::Postgres> for Location {
		fn encode_by_ref(
			&self,
			buf: &mut sqlx::postgres::PgArgumentBuffer,
		) -> sqlx::encode::IsNull {
			<serde_json::Value as sqlx::Encode<'q, sqlx::Postgres>>::encode(
				serialize_location(self),
				buf,
			)
		}
	}

	impl sqlx::Decode<'_, sqlx::Postgres> for Location {
		fn decode(value: sqlx::postgres::PgValueRef) -> Result<Self, sqlx::error::BoxDynError> {
			let value =
				<sqlx::types::Json<Vec<Vec<usize>>> as sqlx::Decode<sqlx::Postgres>>::decode(
					value,
				)?;

			Ok(value.0.into_iter().collect())
		}
	}

	/// Convert location to json as `number[][]`.
	fn serialize_location(location: &Location) -> serde_json::Value {
		serde_json::Value::Array(
			location
				.as_ref()
				.iter()
				.map(|coord| {
					serde_json::Value::Array(
						coord
							.iter()
							.map(|x| serde_json::Value::Number((*x).into()))
							.collect(),
					)
				})
				.collect(),
		)
	}

	// IMPORTANT: Must match the hashing algorithm used in the `db-workflow` `loop_location2_hash` generated
	/// column expression.
	pub fn hash_location(location: &Location) -> WorkflowResult<Vec<u8>> {
		Ok(md5::compute(
			serde_json::to_vec(&serialize_location(location))
				.map_err(WorkflowError::SerializeLocation)?,
		)
		.to_vec())
	}

	/// Takes all workflow events (each with their own location) and combines them via enum into a hashmap of the
	/// following structure:
	///
	/// Given the location [1, 2, 3], 3 is the index and [1, 2] is the root location
	///
	/// HashMap {
	/// 	[1, 2]: [
	/// 		example signal event,
	/// 		example activity event,
	/// 		example sub workflow event,
	/// 		example activity event (this is [1, 2, 3])
	/// 	],
	/// }
	pub fn build_histories(
		workflow_rows: Vec<PulledWorkflowRow>,
		event_rows: Vec<AmalgamEventRow>,
	) -> WorkflowResult<Vec<PulledWorkflow>> {
		// Map workflow rows by workflow id
		let mut workflows_by_id = workflow_rows
			.into_iter()
			.map(|row| {
				let events_by_location: HashMap<Location, Vec<Event>> = HashMap::new();

				(row.workflow_id, (row, events_by_location))
			})
			.collect::<HashMap<_, _>>();

		for event_row in event_rows {
			// Backwards compatibility
			let location_root = event_row
				.location2
				.as_ref()
				.map(|x| x.root())
				.unwrap_or_else(|| {
					event_row
						.location
						.iter()
						.take(event_row.location.len().saturating_sub(1))
						// NOTE: Add 1 because we switched from 0-based to 1-based
						.map(|x| Coordinate::new(Box::new([(*x) as usize + 1])))
						.collect()
				});

			// Get workflow entry
			let (_, ref mut events_by_location) = workflows_by_id
				.get_mut(&event_row.workflow_id)
				.expect("unreachable, workflow for event not found");

			events_by_location
				.entry(location_root)
				.or_default()
				.push(event_row.try_into()?);
		}

		let workflows = workflows_by_id
			.into_values()
			.map(|(row, mut events_by_location)| {
				for events in events_by_location.values_mut() {
					// Events are already mostly sorted themselves so this should be fairly cheap
					events.sort_by_key(|event| event.coordinate().clone());
				}

				PulledWorkflow {
					workflow_id: row.workflow_id,
					workflow_name: row.workflow_name,
					create_ts: row.create_ts,
					ray_id: row.ray_id,
					input: row.input.0,
					wake_deadline_ts: row.wake_deadline_ts,
					events: events_by_location,
				}
			})
			.collect();

		Ok(workflows)
	}
}
use types::*;
