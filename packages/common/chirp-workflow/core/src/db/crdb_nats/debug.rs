use std::result::Result::{Err, Ok};

use anyhow::*;
use indoc::indoc;
use rivet_pools::prelude::*;
use uuid::Uuid;

use super::DatabaseCrdbNats;
use crate::{
	db::debug::{
		ActivityError, ActivityEvent, DatabaseDebug, Event, EventData, HistoryData, LoopEvent,
		MessageSendEvent, SignalData, SignalEvent, SignalSendEvent, SignalState, SubWorkflowEvent,
		WorkflowData, WorkflowState,
	},
	history::{
		event::{EventType, RemovedEvent, SleepEvent, SleepState},
		location::{Coordinate, Location},
	},
};

// HACK: We alias global error here because its hardcoded into the sql macros
type GlobalError = anyhow::Error;

#[async_trait::async_trait]
impl DatabaseDebug for DatabaseCrdbNats {
	#[tracing::instrument(skip_all)]
	async fn get_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<Vec<WorkflowData>> {
		let workflows = sql_fetch_all!(
			[self, WorkflowRow]
			"
			SELECT
				workflow_id,
				workflow_name,
				COALESCE(tags, '{}'::JSONB) AS tags,
				create_ts,
				silence_ts,
				input,
				output,
				error,
				worker_instance_id IS NOT NULL AS is_active,
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				) AS has_wake_condition
			FROM db_workflow.workflows
			WHERE
				workflow_id = ANY($1)
			",
			workflow_ids,
		)
		.await?;

		Ok(workflows.into_iter().map(Into::into).collect())
	}

	#[tracing::instrument(skip_all)]
	async fn find_workflows(
		&self,
		tags: &[(String, String)],
		name: Option<&str>,
		state: Option<WorkflowState>,
	) -> Result<Vec<WorkflowData>> {
		let mut query_str = indoc!(
			"
			SELECT
				workflow_id,
				workflow_name,
				COALESCE(tags, '{}'::JSONB) AS tags,
				create_ts,
				silence_ts,
				input,
				output,
				error,
				worker_instance_id IS NOT NULL AS is_active,
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				) AS has_wake_condition
			FROM db_workflow.workflows
			WHERE
				($1 IS NULL OR workflow_name = $1) AND
				-- Complete
				(NOT $2 OR output IS NOT NULL) AND
				-- Running
				(NOT $3 OR (
					output IS NULL AND
					worker_instance_id IS NOT NULL
				)) AND
				-- Sleeping
				(NOT $4 OR (
					output IS NULL AND
					worker_instance_id IS NULL AND
					(
						wake_immediate OR
						wake_deadline_ts IS NOT NULL OR
						cardinality(wake_signals) > 0 OR
						wake_sub_workflow_id IS NOT NULL
					)
				)) AND
				-- Dead
				(NOT $5 OR (
					output IS NULL AND
					worker_instance_id IS NULL AND
					wake_immediate = FALSE AND
					wake_deadline_ts IS NULL AND
					cardinality(wake_signals) = 0 AND
					wake_sub_workflow_id IS NULL
				)) AND
				(CASE WHEN $6 THEN silence_ts IS NOT NULL ELSE silence_ts IS NULL END)
			"
		)
		.to_string();

		// TODO: This should use some jsonb agg function instead
		// Procedurally add tags. We don't combine the tags into an object because we are comparing
		// strings with `->>` whereas with @> and `serde_json::Map` we would have to know the type of the input
		// given.
		for i in 0..tags.len() {
			let idx = i * 2 + 7;
			let idx2 = idx + 1;

			query_str.push_str(&format!(" AND tags->>${idx} = ${idx2}"));
		}

		query_str.push_str("LIMIT 100");

		let mut query = sqlx::query_as::<_, WorkflowRow>(&query_str)
			.bind(name)
			.bind(matches!(state, Some(WorkflowState::Complete)))
			.bind(matches!(state, Some(WorkflowState::Running)))
			.bind(matches!(state, Some(WorkflowState::Sleeping)))
			.bind(matches!(state, Some(WorkflowState::Dead)))
			.bind(matches!(state, Some(WorkflowState::Silenced)));

		for (key, value) in tags {
			query = query.bind(key);
			query = query.bind(value);
		}

		let mut conn = self.conn().await?;
		let workflows = query.fetch_all(&mut *conn).await?;

		Ok(workflows.into_iter().map(Into::into).collect())
	}

	#[tracing::instrument(skip_all)]
	async fn silence_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<()> {
		sql_execute!(
			[self]
			"
			UPDATE db_workflow.workflows
			SET silence_ts = $2
			WHERE workflow_id = ANY($1)
			",
			workflow_ids,
			rivet_util::timestamp::now(),
		)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn wake_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<()> {
		sql_execute!(
			[self]
			"
			UPDATE db_workflow.workflows
			SET wake_immediate = TRUE
			WHERE workflow_id = ANY($1)
			",
			workflow_ids,
		)
		.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn get_workflow_history(
		&self,
		workflow_id: Uuid,
		include_forgotten: bool,
	) -> Result<Option<HistoryData>> {
		let (wf_row, event_rows, error_rows) = tokio::try_join!(
			sql_fetch_optional!(
				[self, WorkflowRow]
				"
				SELECT
					workflow_id,
					workflow_name,
					COALESCE(tags, '{}'::JSONB) AS tags,
					create_ts,
					silence_ts,
					input,
					output,
					error,
					worker_instance_id IS NOT NULL AS is_active,
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
			),
			sql_fetch_all!(
				[self, AmalgamEventRow]
				"
				-- Activity events
				SELECT
					location,
					location2,
					NULL AS tags,
					0 AS event_type,
					version,
					create_ts,
					activity_name AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					input,
					output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_activity_events
				WHERE
					workflow_activity_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Signal listen events
				SELECT
					location,
					location2,
					NULL AS tags,
					1 AS event_type,
					version,
					ack_ts AS create_ts,
					signal_name AS name,
					signal_id::UUID AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					body AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_signal_events
				WHERE
					workflow_signal_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Signal send events
				SELECT
					location,
					location2,
					s.tags,
					2 AS event_type,
					version,
					NULL as create_ts,
					se.signal_name AS name,
					se.signal_id AS auxiliary_id,
					s2.workflow_id AS auxiliary_id2,
					se.body AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_signal_send_events AS se
				LEFT JOIN db_workflow.tagged_signals AS s
				ON se.signal_id = s.signal_id
				LEFT JOIN db_workflow.signals AS s2
				ON se.signal_id = s2.signal_id
				WHERE
					se.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Message send events
				SELECT
					location,
					location2,
					tags,
					3 AS event_type,
					version,
					NULL AS create_ts,
					message_name AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					body AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten				
				FROM db_workflow.workflow_message_send_events
				WHERE
					workflow_message_send_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Sub workflow events
				SELECT
					location,
					location2,
					COALESCE(w.tags, '{}'::JSONB) AS tags,
					4 AS event_type,
					version,
					sw.create_ts,
					w.workflow_name AS name,
					sw.sub_workflow_id AS auxiliary_id,
					NULL AS auxiliary_id2,
					w.input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_sub_workflow_events AS sw
				JOIN db_workflow.workflows AS w
				ON sw.sub_workflow_id = w.workflow_id
				WHERE
					sw.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Loop events
				SELECT
					location,
					location2,
					NULL AS tags,
					5 AS event_type,
					version,
					NULL as create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					state AS input,
					output,
					iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_loop_events
				WHERE
					workflow_loop_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					location,
					location2,
					NULL AS tags,
					6 AS event_type,
					version,
					NULL as create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					deadline_ts,
					state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_sleep_events
				WHERE
					workflow_sleep_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					7 AS event_type,
					version,
					NULL as create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_branch_events
				WHERE
					workflow_branch_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					8 AS event_type,
					1 AS version,
					NULL as create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					event_type AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_removed_events
				WHERE
					workflow_removed_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					9 AS event_type,
					version,
					NULL as create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_version_check_events
				WHERE
					workflow_version_check_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				ORDER BY location ASC, location2 ASC
				",
				workflow_id,
				include_forgotten,
			),
			sql_fetch_all!(
				[self, ActivityErrorRow]
				"
				SELECT location, location2, error, COUNT(error), MAX(ts) AS latest_ts
				FROM db_workflow.workflow_activity_errors
				WHERE workflow_id = $1
				GROUP BY location, location2, error
				ORDER BY latest_ts
				",
				workflow_id,
			),
		)?;

		let Some(wf_row) = wf_row else {
			return Ok(None);
		};

		Ok(Some(HistoryData {
			wf: wf_row.into(),
			events: build_history(event_rows, error_rows)?,
		}))
	}

	#[tracing::instrument(skip_all)]
	async fn get_signals(&self, signal_ids: Vec<Uuid>) -> Result<Vec<SignalData>> {
		let signals = sql_fetch_all!(
			[self, SignalRow]
			"
			SELECT
				signal_id,
				signal_name,
				NULL AS tags,
				workflow_id,
				create_ts,
				body,
				ack_ts
			FROM db_workflow.signals
			WHERE signal_id = $1
			UNION ALL
			SELECT
				signal_id,
				signal_name,
				tags,
				NULL AS workflow_id,
				create_ts,
				body,
				ack_ts
			FROM db_workflow.tagged_signals
			WHERE signal_id = ANY($1)
			",
			signal_ids,
		)
		.await?;

		Ok(signals.into_iter().map(Into::into).collect())
	}

	#[tracing::instrument(skip_all)]
	async fn find_signals(
		&self,
		tags: &[(String, String)],
		workflow_id: Option<Uuid>,
		name: Option<&str>,
		state: Option<SignalState>,
	) -> Result<Vec<SignalData>> {
		let mut query_str = indoc!(
			"
			SELECT
				signal_id,
				signal_name,
				NULL AS tags,
				workflow_id,
				create_ts,
				silence_ts,
				body,
				ack_ts
			FROM db_workflow.signals
			WHERE
				($1 IS NULL OR signal_name = $1) AND
				($2 IS NULL OR workflow_id = $2) AND
				silence_ts IS NULL AND
				-- Acked
				(NOT $3 OR ack_ts IS NOT NULL) AND
				-- Pending
				(NOT $4 OR ack_ts IS NULL)
			UNION ALL
			SELECT
				signal_id,
				signal_name,
				tags,
				NULL AS workflow_id,
				create_ts,
				body,
				ack_ts
			FROM db_workflow.tagged_signals
			WHERE
				($1 IS NULL OR signal_name = $1) AND
				-- Acked
				(NOT $3 OR ack_ts IS NOT NULL) AND
				-- Pending
				(NOT $4 OR ack_ts IS NULL) AND
				(CASE WHEN $5 THEN silence_ts IS NULL ELSE silence_ts IS NOT NULL END)
			"
		)
		.to_string();

		// Procedurally add tags. We don't combine the tags into an object because we are comparing
		// strings with `->>` whereas with @> and `serde_json::Map` we would have to know the type of the input
		// given.
		for i in 0..tags.len() {
			let idx = i * 2 + 6;
			let idx2 = idx + 1;

			query_str.push_str(&format!(" AND tags->>${idx} = ${idx2}"));
		}

		query_str.push_str("LIMIT 100");

		let mut query = sqlx::query_as::<_, SignalRow>(&query_str)
			.bind(name)
			.bind(workflow_id)
			.bind(matches!(state, Some(SignalState::Acked)))
			.bind(matches!(state, Some(SignalState::Pending)))
			.bind(matches!(state, Some(SignalState::Silenced)));

		for (key, value) in tags {
			query = query.bind(key);
			query = query.bind(value);
		}

		let mut conn = self.conn().await?;
		let signals = query.fetch_all(&mut *conn).await?;

		Ok(signals.into_iter().map(Into::into).collect())
	}

	#[tracing::instrument(skip_all)]
	async fn silence_signals(&self, signal_ids: Vec<Uuid>) -> Result<()> {
		sql_execute!(
			[self]
			"
			WITH
				update_signals AS (
					UPDATE db_workflow.signals
					SET silence_ts = $2
					WHERE signal_id = ANY($1)
					RETURNING 1
				),
				update_tagged_signals AS (
					UPDATE db_workflow.tagged_signals
					SET silence_ts = $2
					WHERE signal_id = ANY($1)
					RETURNING 1
				)
			SELECT 1
			",
			signal_ids,
			rivet_util::timestamp::now(),
		)
		.await?;

		Ok(())
	}
}

#[derive(Debug, sqlx::FromRow)]
struct WorkflowRow {
	workflow_id: Uuid,
	workflow_name: String,
	tags: serde_json::Value,
	create_ts: i64,
	silence_ts: Option<i64>,
	input: serde_json::Value,
	output: Option<serde_json::Value>,
	error: Option<String>,

	is_active: bool,
	has_wake_condition: bool,
}

impl From<WorkflowRow> for WorkflowData {
	fn from(row: WorkflowRow) -> Self {
		let state = if row.silence_ts.is_some() {
			WorkflowState::Silenced
		} else if row.output.is_some() {
			WorkflowState::Complete
		} else if row.is_active {
			WorkflowState::Running
		} else if row.has_wake_condition {
			WorkflowState::Sleeping
		} else {
			WorkflowState::Dead
		};

		WorkflowData {
			workflow_id: row.workflow_id,
			workflow_name: row.workflow_name,
			tags: row.tags,
			create_ts: row.create_ts,
			input: row.input,
			output: row.output,
			error: row.error,
			state,
		}
	}
}

#[derive(sqlx::FromRow)]
struct ActivityErrorRow {
	location: Vec<i64>,
	location2: Option<Location>,
	error: String,
	count: i64,
	latest_ts: i64,
}

#[derive(sqlx::FromRow)]
struct AmalgamEventRow {
	location: Vec<i64>,
	location2: Option<Location>,
	tags: Option<serde_json::Value>,
	version: i64,
	create_ts: Option<i64>,
	event_type: i64,
	name: Option<String>,
	auxiliary_id: Option<Uuid>,
	auxiliary_id2: Option<Uuid>,
	input: Option<serde_json::Value>,
	output: Option<serde_json::Value>,
	iteration: Option<i64>,
	deadline_ts: Option<i64>,
	state: Option<i64>,
	inner_event_type: Option<i64>,
	forgotten: bool,
}

impl TryFrom<AmalgamEventRow> for Event {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		// Backwards compatibility
		let location = value.location2.clone().unwrap_or_else(|| {
			// NOTE: Add 1 because we switched from 0-based to 1-based
			value
				.location
				.iter()
				.map(|x| Coordinate::simple(*x as usize + 1))
				.collect()
		});

		let event_type = value.event_type.try_into().context("integer conversion")?;
		let event_type = EventType::from_repr(event_type)
			.with_context(|| format!("invalid event type: {}", value.event_type))?;

		Ok(Event {
			location,
			version: value.version.try_into().context("integer conversion")?,
			create_ts: value.create_ts.unwrap_or_default(),
			forgotten: value.forgotten,
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
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(ActivityEvent {
			name: value.name.context("missing event data")?,
			input: value.input.context("missing event data")?,
			output: value.output,
			// Filled in later
			errors: Vec::new(),
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SignalEvent {
			signal_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			body: value.output.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SignalSendEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SignalSendEvent {
			signal_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			workflow_id: value.auxiliary_id2,
			tags: value.tags,
			body: value.input.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for MessageSendEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(MessageSendEvent {
			name: value.name.context("missing event data")?,
			tags: value.tags.context("missing event data")?,
			body: value.input.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SubWorkflowEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value.auxiliary_id.context("missing event data")?,
			name: value.name.context("missing event data")?,
			tags: value.tags.context("missing event data")?,
			input: value.input.context("missing event data")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for LoopEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		Ok(LoopEvent {
			state: value.input.context("missing event data")?,
			output: value.output,
			iteration: value
				.iteration
				.context("missing event data")?
				.try_into()
				.context("integer conversion")?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for SleepEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		let state = value.state.context("missing event data")?;

		Ok(SleepEvent {
			deadline_ts: value.deadline_ts.context("missing event data")?,
			state: SleepState::from_repr(state.try_into()?)
				.with_context(|| format!("invalid sleep state type: {}", state))?,
		})
	}
}

impl TryFrom<AmalgamEventRow> for RemovedEvent {
	type Error = anyhow::Error;

	fn try_from(value: AmalgamEventRow) -> Result<Self> {
		let event_type = value.inner_event_type.context("missing event data")?;

		Ok(RemovedEvent {
			name: value.name,
			event_type: EventType::from_repr(event_type.try_into()?)
				.with_context(|| format!("invalid event type: {}", event_type))?,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
pub struct SignalRow {
	signal_id: Uuid,
	signal_name: String,
	tags: Option<serde_json::Value>,
	workflow_id: Option<Uuid>,
	create_ts: i64,
	silence_ts: Option<i64>,
	body: serde_json::Value,
	ack_ts: Option<i64>,
}

impl From<SignalRow> for SignalData {
	fn from(row: SignalRow) -> Self {
		let state = if row.silence_ts.is_some() {
			SignalState::Silenced
		} else if row.ack_ts.is_some() {
			SignalState::Acked
		} else {
			SignalState::Pending
		};

		SignalData {
			signal_id: row.signal_id,
			signal_name: row.signal_name,
			tags: row.tags,
			workflow_id: row.workflow_id,
			create_ts: row.create_ts,
			body: row.body,
			state,
		}
	}
}

fn build_history(
	event_rows: Vec<AmalgamEventRow>,
	error_rows: Vec<ActivityErrorRow>,
) -> Result<Vec<Event>> {
	let mut events = event_rows
		.into_iter()
		.map(|row| {
			let mut event = TryInto::<Event>::try_into(row)?;

			// Add errors to activity events
			if let EventData::Activity(data) = &mut event.data {
				data.errors = error_rows
					.iter()
					.filter(|row| {
						// Backwards compatibility
						// NOTE: Add 1 because we switched from 0-based to 1-based
						let location = row.location2.clone().unwrap_or_else(|| {
							row.location
								.iter()
								.map(|x| Coordinate::simple(*x as usize + 1))
								.collect()
						});

						location == event.location
					})
					.map(|row| ActivityError {
						error: row.error.clone(),
						count: row.count as usize,
						latest_ts: row.latest_ts,
					})
					.collect();
			}

			Ok(event)
		})
		.collect::<Result<Vec<_>>>()?;

	// Events are already mostly sorted themselves so this should be fairly cheap
	events.sort_by_key(|event| event.location.clone());

	// NOTE: The following is only for the purpose of backwards compatibility for workflows
	// that were created before the branch event was formally added.

	// Check for missing indexes and insert a `Empty` placeholder event for each missing
	// spot.
	let mut last_location = Location::new(Box::new([Coordinate::simple(0)]));
	let mut skip = false;
	events = events
		.drain(..)
		.flat_map(|event| {
			let last_coord_head = if last_location.root().len() < event.location.root().len() {
				0
			} else {
				// Get common root via cardinality
				last_location
					.iter()
					.take(last_location.len().min(event.location.len()))
					.cloned()
					.collect::<Location>()
					.tail()
					.expect("empty location")
					.head()
			};
			let curr_coord_head = event.location.tail().expect("empty location").head();

			let offset = if skip {
				0
			} else {
				(curr_coord_head - last_coord_head).saturating_sub(1)
			};

			last_location = event.location.clone();
			// Skip the next empty section for a loop
			skip = matches!(event.data, EventData::Loop(_));

			let root = event.location.root();

			(1..=offset)
				.map(move |i| Event {
					location: root.join(Coordinate::simple(last_coord_head + i)),
					version: 0,
					create_ts: 0,
					forgotten: false,
					data: EventData::Empty,
				})
				.chain(std::iter::once(event))
		})
		.collect();

	Ok(events)
}
