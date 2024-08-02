use std::{collections::HashMap, sync::Arc, time::Duration};

use indoc::indoc;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use uuid::Uuid;

use super::{
	ActivityEventRow, Database, PulledWorkflow, PulledWorkflowRow, SignalEventRow, SignalRow,
	SubWorkflowEventRow, WorkflowRow,
};
use crate::{schema::ActivityId, WorkflowError, WorkflowResult};

pub struct DatabasePostgres {
	pool: PgPool,
}

impl DatabasePostgres {
	pub async fn new(url: &str) -> WorkflowResult<Arc<DatabasePostgres>> {
		let pool = sqlx::postgres::PgPoolOptions::new()
			// The default connection timeout is too high
			.acquire_timeout(Duration::from_secs(15))
			// Increase lifetime to mitigate: https://github.com/launchbadge/sqlx/issues/2854
			//
			// See max lifetime https://www.cockroachlabs.com/docs/stable/connection-pooling#set-the-maximum-lifetime-of-connections
			.max_lifetime(Duration::from_secs(30 * 60))
			// Remove connections after a while in order to reduce load
			// on CRDB after bursts
			.idle_timeout(Some(Duration::from_secs(3 * 60)))
			// Open connections immediately on startup
			.min_connections(1)
			// Raise the cap, since this is effectively the amount of
			// simultaneous requests we can handle. See
			// https://www.cockroachlabs.com/docs/stable/connection-pooling.html
			.max_connections(4096)
			.connect(url)
			.await
			.map_err(WorkflowError::BuildSqlx)?;

		Ok(Arc::new(DatabasePostgres { pool }))
	}

	pub fn from_pool(pool: PgPool) -> Arc<DatabasePostgres> {
		Arc::new(DatabasePostgres { pool })
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
}

#[async_trait::async_trait]
impl Database for DatabasePostgres {
	async fn dispatch_workflow(
		&self,
		ray_id: Uuid,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: serde_json::Value,
	) -> WorkflowResult<()> {
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
		.bind(tags)
		.bind(input)
		.execute(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

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
		// TODO(RVT-3753): include limit on query to allow better workflow spread between nodes?
		// Select all workflows that haven't started or that have a wake condition
		let rows = sqlx::query_as::<_, PulledWorkflowRow>(indoc!(
			"
			WITH
				pull_workflows AS (
					UPDATE db_workflow.workflows AS w
						-- Assign this node to this workflow
					SET worker_instance_id = $1
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
							wake_deadline_ts IS NOT NULL OR
							-- Signal exists
							(
								SELECT true
								FROM db_workflow.signals AS s
								WHERE s.signal_name = ANY(wake_signals)
								LIMIT 1
							) OR
							-- Tagged signal exists
							(
								SELECT true
								FROM db_workflow.tagged_signals AS s
								WHERE w.tags @> s.tags
								LIMIT 1
							) OR
							-- Sub workflow completed
							(
								SELECT true
								FROM db_workflow.workflows AS w2
								WHERE
									w2.workflow_id = w.wake_sub_workflow_id AND
									output IS NOT NULL
							)
						)
					RETURNING workflow_id, workflow_name, create_ts, ray_id, input, wake_deadline_ts
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
		.fetch_all(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

		if rows.is_empty() {
			return Ok(Vec::new());
		}

		// Turn rows into hashmap
		let workflow_ids = rows.iter().map(|row| row.workflow_id).collect::<Vec<_>>();
		let mut workflows_by_id = rows
			.into_iter()
			.map(|row| {
				(
					row.workflow_id,
					PulledWorkflow {
						workflow_id: row.workflow_id,
						workflow_name: row.workflow_name,
						create_ts: row.create_ts,
						ray_id: row.ray_id,
						input: row.input,
						wake_deadline_ts: row.wake_deadline_ts,
						activity_events: Vec::new(),
						signal_events: Vec::new(),
						sub_workflow_events: Vec::new(),
					},
				)
			})
			.collect::<HashMap<_, _>>();

		// Fetch all events for all fetched workflows
		let (activity_events, signal_events, sub_workflow_events) = tokio::try_join!(
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
					WHERE ev.workflow_id = ANY($1)
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
					WHERE workflow_id = ANY($1)
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
					WHERE sw.workflow_id = ANY($1)
					ORDER BY sw.workflow_id, sw.location ASC
					",
				))
				.bind(&workflow_ids)
				.fetch_all(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)
			}
		)?;

		// Insert events into hashmap
		for event in activity_events {
			workflows_by_id
				.get_mut(&event.workflow_id)
				.expect("unreachable, workflow for event not found")
				.activity_events
				.push(event);
		}
		for event in signal_events {
			workflows_by_id
				.get_mut(&event.workflow_id)
				.expect("unreachable, workflow for event not found")
				.signal_events
				.push(event);
		}
		for event in sub_workflow_events {
			workflows_by_id
				.get_mut(&event.workflow_id)
				.expect("unreachable, workflow for event not found")
				.sub_workflow_events
				.push(event);
		}

		Ok(workflows_by_id.into_values().collect())
	}

	async fn commit_workflow(
		&self,
		workflow_id: Uuid,
		output: &serde_json::Value,
	) -> WorkflowResult<()> {
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
		.map_err(WorkflowError::Sqlx)?;

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
		// TODO(RVT-3762): Should this compare `wake_deadline_ts` before setting it?
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
		.map_err(WorkflowError::Sqlx)?;

		Ok(())
	}

	// TODO: Theres nothing preventing this from being able to be called from the workflow ctx also, but for
	// now its only in the activity ctx so it isn't called again during workflow retries
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		tags: &serde_json::Value,
	) -> WorkflowResult<()> {
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
		.map_err(WorkflowError::Sqlx)?;

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
	) -> WorkflowResult<()> {
		match res {
			Ok(output) => {
				sqlx::query(indoc!(
					"
					INSERT INTO db_workflow.workflow_activity_events (
						workflow_id, location, activity_name, input_hash, input, output, create_ts
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7)
					ON CONFLICT (workflow_id, location) DO UPDATE
					SET output = excluded.output
					",
				))
				.bind(workflow_id)
				.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
				.bind(&activity_id.name)
				.bind(activity_id.input_hash.to_le_bytes())
				.bind(input)
				.bind(output)
				.bind(rivet_util::timestamp::now())
				.bind(create_ts)
				.execute(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)?;
			}
			Err(err) => {
				sqlx::query(indoc!(
					"
					WITH
						event AS (
							INSERT INTO db_workflow.workflow_activity_events (
								workflow_id, location, activity_name, input_hash, input, create_ts
							)
							VALUES ($1, $2, $3, $4, $5, $7)
							ON CONFLICT (workflow_id, location) DO NOTHING
							RETURNING 1
						),
						err AS (
							INSERT INTO db_workflow.workflow_activity_errors (
								workflow_id, location, activity_name, error, ts
							)
							VALUES ($1, $2, $3, $6, $7)
							RETURNING 1
						)
					SELECT 1
					",
				))
				.bind(workflow_id)
				.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
				.bind(&activity_id.name)
				.bind(activity_id.input_hash.to_le_bytes())
				.bind(input)
				.bind(err)
				.bind(create_ts)
				.execute(&mut *self.conn().await?)
				.await
				.map_err(WorkflowError::Sqlx)?;
			}
		}

		Ok(())
	}

	async fn pull_next_signal(
		&self,
		workflow_id: Uuid,
		filter: &[&str],
		location: &[usize],
	) -> WorkflowResult<Option<SignalRow>> {
		let signal = sqlx::query_as::<_, SignalRow>(indoc!(
			"
			WITH
				-- Finds the oldest signal matching the signal name filter in either the normal signals table
				-- or tagged signals table
				next_signal AS (
					SELECT false AS tagged, signal_id, create_ts, signal_name, body
					FROM db_workflow.signals
					WHERE
						workflow_id = $1 AND
						signal_name = ANY($2)
					UNION ALL
					SELECT true AS tagged, signal_id, create_ts, signal_name, body
					FROM db_workflow.tagged_signals
					WHERE
						signal_name = ANY($2) AND
						tags <@ (SELECT tags FROM db_workflow.workflows WHERE workflow_id = $1)
					ORDER BY create_ts ASC
					LIMIT 1
				),
				-- If the next signal is not tagged, delete it with this statement
				delete_signal AS (
					DELETE FROM db_workflow.signals
					WHERE signal_id = (
						SELECT signal_id FROM next_signal WHERE tagged = false
					)
					RETURNING 1
				),
				-- If the next signal is tagged, delete it with this statement
				delete_tagged_signal AS (
					DELETE FROM db_workflow.tagged_signals
					WHERE signal_id = (
						SELECT signal_id FROM next_signal WHERE tagged = true
					)
					RETURNING 1
				),
				-- After deleting the signal, add it to the events table (i.e. acknowledge it)
				insert_event AS (
					INSERT INTO db_workflow.workflow_signal_events (
						workflow_id, location, signal_id, signal_name, body, ack_ts
					)
					SELECT $1 AS workflow_id, $3 AS location, signal_id, signal_name, body, $4 AS ack_ts
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
		.fetch_optional(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

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
		sqlx::query(indoc!(
			"
			INSERT INTO db_workflow.signals (signal_id, workflow_id, signal_name, body, ray_id, create_ts)			
			VALUES ($1, $2, $3, $4, $5, $6)
			",
		))
		.bind(signal_id)
		.bind(workflow_id)
		.bind(signal_name)
		.bind(body)
		.bind(ray_id)
		.bind(rivet_util::timestamp::now())
		.execute(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

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
		sqlx::query(indoc!(
			"
			INSERT INTO db_workflow.tagged_signals (signal_id, tags, signal_name, body, ray_id, create_ts)			
			VALUES ($1, $2, $3, $4, $5, $6)
			",
		))
		.bind(signal_id)
		.bind(tags)
		.bind(signal_name)
		.bind(body)
		.bind(ray_id)
		.bind(rivet_util::timestamp::now())
		.execute(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

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
	) -> WorkflowResult<()> {
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
						workflow_id, location, sub_workflow_id, create_ts
					)
					VALUES($1, $7, $8, $3)
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
		.bind(input)
		.bind(location.iter().map(|x| *x as i64).collect::<Vec<_>>())
		.bind(sub_workflow_id)
		.execute(&mut *self.conn().await?)
		.await
		.map_err(WorkflowError::Sqlx)?;

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
}
