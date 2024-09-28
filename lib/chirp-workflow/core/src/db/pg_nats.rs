use std::{sync::Arc, time::Duration};

use indoc::indoc;
use rivet_pools::prelude::NatsPool;
use sqlx::{pool::PoolConnection, Acquire, PgPool, Postgres};
use tracing::Instrument;
use uuid::Uuid;

use super::{
	ActivityEventRow, Database, LoopEventRow, MessageSendEventRow, PulledWorkflow,
	PulledWorkflowRow, SignalEventRow, SignalRow, SignalSendEventRow, SleepEventRow,
	SubWorkflowEventRow, WorkflowRow,
};
use crate::{
	activity::ActivityId,
	error::{WorkflowError, WorkflowResult},
	event::combine_events,
	message, worker,
};

/// Max amount of workflows pulled from the database with each call to `pull_workflows`.
const MAX_PULLED_WORKFLOWS: i64 = 50;
// Base retry for query retry backoff
const QUERY_RETRY_MS: usize = 750;
// Time in between transaction retries
const TXN_RETRY: Duration = Duration::from_millis(100);
/// Maximum times a query ran bu this database adapter is retried.
const MAX_QUERY_RETRIES: usize = 16;

pub struct DatabasePgNats {
	pool: PgPool,
	nats: NatsPool,
}

impl DatabasePgNats {
	pub fn from_pools(pool: PgPool, nats: NatsPool) -> Arc<DatabasePgNats> {
		Arc::new(DatabasePgNats { pool, nats })
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

		let spawn_res = tokio::task::Builder::new()
			.name("chirp_workflow::DatabasePgNats::wake")
			.spawn(
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
						// Retry transaction errors immediately
						Database(db_err)
							if db_err
								.message()
								.contains("TransactionRetryWithProtoRefreshError") =>
						{
							tracing::info!(message=%db_err.message(), "transaction retry");
							tokio::time::sleep(TXN_RETRY).await;
						}
						// Retry internal errors with a backoff
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
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: serde_json::Value,
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
			.bind(&input)
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		self.wake_worker();

		Ok(())
	}

	async fn get_workflow(&self, workflow_id: Uuid) -> WorkflowResult<Option<WorkflowRow>> {
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
									-- Signal exists
									(
										SELECT true
										FROM db_workflow.signals AS s
										WHERE
											s.workflow_id = w.workflow_id AND
											s.signal_name = ANY(w.wake_signals) AND
											s.ack_ts IS NULL
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
									-- Tagged signal exists
									(
										SELECT true
										FROM db_workflow.tagged_signals AS s
										WHERE
											s.signal_name = ANY(w.wake_signals) AND
											s.tags <@ w.tags AND
											s.ack_ts IS NULL
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

		// Turn rows into hashmap
		let workflow_ids = workflow_rows
			.iter()
			.map(|row| row.workflow_id)
			.collect::<Vec<_>>();

		// TODO: Convert into union query
		// Fetch all events for all fetched workflows
		let (
			activity_events,
			signal_events,
			signal_send_events,
			msg_send_events,
			sub_workflow_events,
			loop_events,
			sleep_events,
		) = tokio::try_join!(
			async {
				sqlx::query_as::<_, ActivityEventRow>(indoc!(
					"
					SELECT
						ev.workflow_id,
						ev.location,
						ev.activity_name,
						ev.input_hash,
						ev.output,
						ev.create_ts,
						COUNT(err.workflow_id) AS error_count
					FROM db_workflow.workflow_activity_events AS ev
					LEFT JOIN db_workflow.workflow_activity_errors AS err
					ON
						ev.workflow_id = err.workflow_id AND
						ev.location = err.location
					WHERE ev.workflow_id = ANY($1) AND forgotten = FALSE
					GROUP BY ev.workflow_id, ev.location, ev.activity_name, ev.input_hash, ev.output
					ORDER BY ev.workflow_id, ev.location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, SignalEventRow>(indoc!(
					"
					SELECT
						workflow_id, location, signal_name, body
					FROM db_workflow.workflow_signal_events
					WHERE workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY workflow_id, location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, SignalSendEventRow>(indoc!(
					"
					SELECT
						workflow_id, location, signal_id, signal_name
					FROM db_workflow.workflow_signal_send_events
					WHERE workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY workflow_id, location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, MessageSendEventRow>(indoc!(
					"
					SELECT
						workflow_id, location, message_name
					FROM db_workflow.workflow_message_send_events
					WHERE workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY workflow_id, location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, SubWorkflowEventRow>(indoc!(
					"
					SELECT
						sw.workflow_id,
						sw.location,
						sw.sub_workflow_id,
						w.workflow_name AS sub_workflow_name
					FROM db_workflow.workflow_sub_workflow_events AS sw
					JOIN db_workflow.workflows AS w
					ON sw.sub_workflow_id = w.workflow_id
					WHERE sw.workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY sw.workflow_id, sw.location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, LoopEventRow>(indoc!(
					"
					SELECT
						workflow_id, location, iteration, output
					FROM db_workflow.workflow_loop_events
					WHERE workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY workflow_id, location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
			async {
				sqlx::query_as::<_, SleepEventRow>(indoc!(
					"
					SELECT
						workflow_id, location, deadline_ts
					FROM db_workflow.workflow_sleep_events
					WHERE workflow_id = ANY($1) AND forgotten = FALSE
					ORDER BY workflow_id, location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			},
		)?;

		let workflows = combine_events(
			workflow_rows,
			activity_events,
			signal_events,
			signal_send_events,
			msg_send_events,
			sub_workflow_events,
			loop_events,
			sleep_events,
		)?;

		Ok(workflows)
	}

	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		output: &serde_json::Value,
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
			.bind(output)
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
		location: &[usize],
		activity_id: &ActivityId,
		create_ts: i64,
		input: serde_json::Value,
		res: Result<serde_json::Value, &str>,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()> {
		match res {
			Ok(output) => {
				self.query(|| async {
					sqlx::query(indoc!(
						"
						INSERT INTO db_workflow.workflow_activity_events (
							workflow_id,
							location,
							activity_name,
							input_hash,
							input,
							output,
							create_ts,
							loop_location
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
						ON CONFLICT (workflow_id, location) DO UPDATE
						SET output = excluded.output
						",
					))
					.bind(workflow_id)
					.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
					.bind(&activity_id.name)
					.bind(activity_id.input_hash.to_le_bytes())
					.bind(&input)
					.bind(&output)
					.bind(create_ts)
					.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
									location,
									activity_name,
									input_hash,
									input,
									create_ts,
									loop_location
								)
								VALUES ($1, $2, $3, $4, $5, $7, $8)
								ON CONFLICT (workflow_id, location) DO NOTHING
								RETURNING 1
							),
							err AS (
								INSERT INTO db_workflow.workflow_activity_errors (
									workflow_id, location, activity_name, error, ts
								)
								VALUES ($1, $2, $3, $6, $9)
								RETURNING 1
							)
						SELECT 1
						",
					))
					.bind(workflow_id)
					.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
					.bind(&activity_id.name)
					.bind(activity_id.input_hash.to_le_bytes())
					.bind(&input)
					.bind(err)
					.bind(create_ts)
					.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
		location: &[usize],
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<Option<SignalRow>> {
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
								ack_ts IS NULL
							UNION ALL
							SELECT true AS tagged, signal_id, create_ts, signal_name, body
							FROM db_workflow.tagged_signals
							WHERE
								signal_name = ANY($2) AND
								tags <@ (SELECT tags FROM db_workflow.workflows WHERE workflow_id = $1) AND
								ack_ts IS NULL
							ORDER BY create_ts ASC
							LIMIT 1
						),
						-- If the next signal is not tagged, acknowledge it with this statement
						ack_signal AS (
							UPDATE db_workflow.signals
							SET ack_ts = $4
							WHERE signal_id = (
								SELECT signal_id FROM next_signal WHERE tagged = false
							)
							RETURNING 1
						),
						-- If the next signal is tagged, acknowledge it with this statement
						ack_tagged_signal AS (
							UPDATE db_workflow.tagged_signals
							SET ack_ts = $4
							WHERE signal_id = (
								SELECT signal_id FROM next_signal WHERE tagged = true
							)
							RETURNING 1
						),
						-- After acking the signal, add it to the events table
						insert_event AS (
							INSERT INTO db_workflow.workflow_signal_events (
								workflow_id, location, signal_id, signal_name, body, ack_ts, loop_location
							)
							SELECT
								$1 AS workflow_id,
								$3 AS location,
								signal_id,
								signal_name,
								body,
								$4 AS ack_ts,
								$5 AS loop_location
							FROM next_signal
							RETURNING 1
						)
					SELECT * FROM next_signal
					",
				))
				.bind(workflow_id)
				.bind(filter)
				.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
				.bind(rivet_util::timestamp::now())
				.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
				.fetch_optional(&mut *self.conn().await?)
				.await
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
		body: serde_json::Value,
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
			.bind(&body)
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
		body: serde_json::Value,
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
			.bind(&body)
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
		location: &[usize],
		ray_id: Uuid,
		to_workflow_id: Uuid,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
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
							workflow_id, location, signal_id, signal_name, body, loop_location
						)
						VALUES($7, $8, $1, $3, $4, $9)
						RETURNING 1
					)
				SELECT 1
				",
			))
			.bind(signal_id)
			.bind(to_workflow_id)
			.bind(signal_name)
			.bind(&body)
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.bind(from_workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
		location: &[usize],
		ray_id: Uuid,
		tags: &serde_json::Value,
		signal_id: Uuid,
		signal_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
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
							workflow_id, location, signal_id, signal_name, body, loop_location
						)
						VALUES($7, $8, $1, $3, $4, $9)
						RETURNING 1
					)
				SELECT 1
				",
			))
			.bind(signal_id)
			.bind(tags)
			.bind(signal_name)
			.bind(&body)
			.bind(ray_id)
			.bind(rivet_util::timestamp::now())
			.bind(from_workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
		location: &[usize],
		sub_workflow_id: Uuid,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: serde_json::Value,
		loop_location: Option<&[usize]>,
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
							workflow_id, location, sub_workflow_id, create_ts, loop_location
						)
						VALUES($1, $7, $8, $3, $9)
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
			.bind(&input)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(sub_workflow_id)
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
		input: &serde_json::Value,
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
		.bind(input)
		.bind(after_ts)
		.fetch_optional(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)
	}

	async fn commit_workflow_message_send_event(
		&self,
		from_workflow_id: Uuid,
		location: &[usize],
		tags: &serde_json::Value,
		message_name: &str,
		body: serde_json::Value,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_message_send_events(
					workflow_id, location, tags, message_name, body, loop_location
				)
				VALUES($1, $2, $3, $4, $5, $6)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(tags)
			.bind(message_name)
			.bind(&body)
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}

	async fn update_loop(
		&self,
		workflow_id: Uuid,
		location: &[usize],
		iteration: usize,
		output: Option<serde_json::Value>,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			let mut conn = self.conn().await?;
			let mut tx = conn.begin().await.map_err(WorkflowError::Sqlx)?;

			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_loop_events (
					workflow_id,
					location,
					iteration,
					output,
					loop_location
				)
				VALUES ($1, $2, $3, $4, $5)
				ON CONFLICT (workflow_id, location) DO UPDATE
				SET
					iteration = $3,
					output = $4
				RETURNING 1
				",
			))
			.bind(workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(iteration as i64)
			.bind(&output)
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
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
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_signal_events AS (
						UPDATE db_workflow.workflow_signal_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_sub_workflow_events AS (
						UPDATE db_workflow.workflow_sub_workflow_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_signal_send_events AS (
						UPDATE db_workflow.workflow_signal_send_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					),
					forget_message_send_events AS (
						UPDATE db_workflow.workflow_message_send_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					),	
					forget_loop_events AS (
						UPDATE db_workflow.workflow_loop_events
						SET forgotten = TRUE
						WHERE
							workflow_id = $1 AND
							loop_location = $2 AND
							forgotten = FALSE
						RETURNING 1
					)
				SELECT 1
				",
			))
			.bind(workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
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
		location: &[usize],
		deadline_ts: i64,
		loop_location: Option<&[usize]>,
	) -> WorkflowResult<()> {
		self.query(|| async {
			sqlx::query(indoc!(
				"
				INSERT INTO db_workflow.workflow_sleep_events(
					workflow_id, location, deadline_ts, loop_location
				)
				VALUES($1, $2, $3, $4)
				RETURNING 1
				",
			))
			.bind(from_workflow_id)
			.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
			.bind(deadline_ts)
			.bind(loop_location.map(|l| l.iter().map(|x| *x as i64).collect::<Vec<_>>()))
			.execute(&mut *self.conn().await?)
			.await
			.map_err(WorkflowError::Sqlx)
		})
		.await?;

		Ok(())
	}
}
