use std::collections::HashMap;

use fdb_util::keys::*;
use include_dir::{include_dir, Dir, File};
use indoc::indoc;
use rivet_pools::prelude::*;
use sqlx::Acquire;
use uuid::Uuid;

use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{
			ActivityEvent, Event, EventData, EventId, EventType, LoopEvent, MessageSendEvent,
			RemovedEvent, SignalEvent, SignalSendEvent, SleepEvent, SleepState, SubWorkflowEvent,
		},
		location::Location,
	},
};

type RawJson = sqlx::types::Json<Box<serde_json::value::RawValue>>;

// HACK: We alias global error here because its hardcoded into the sql macros
type GlobalError = WorkflowError;

const MIGRATIONS_DIR: Dir =
	include_dir!("$CARGO_MANIFEST_DIR/src/db/fdb_sqlite_nats/sqlite/migrations");

lazy_static::lazy_static! {
	// We use a lazy static because this only needs to be processed once
	static ref MIGRATIONS: Migrations = Migrations::new();
}

struct Migrations {
	last_migration_index: i64,
	files: Vec<(i64, &'static File<'static>)>,
}

impl Migrations {
	fn new() -> Self {
		let files = MIGRATIONS_DIR
			.files()
			.map(|file| (parse_migration_index(file), file))
			.collect::<Vec<_>>();

		Migrations {
			last_migration_index: files
				.iter()
				.fold(0, |max_index, (index, _)| max_index.max(*index)),
			files,
		}
	}
}

// TODO: Used to stub the sql macros, find better solution
pub(crate) struct SqlStub {}

impl SqlStub {
	// For sql macro
	pub fn name(&self) -> &str {
		super::CONTEXT_NAME
	}
}

/// Runs migrations that have not been run yet.
#[tracing::instrument(skip_all)]
pub async fn init(workflow_id: Uuid, pool: &SqlitePool) -> WorkflowResult<()> {
	// Create migrations table
	sql_execute!(
		[SqlStub {}, pool]
		"
		CREATE TABLE IF NOT EXISTS _migrations(
			last_index INTEGER NOT NULL,
			locked INTEGER NOT NULL,
			tainted INTEGER NOT NULL
		)
		",
	)
	.await
	.map_err(WorkflowError::into_migration_err)?;

	// Initialize state row if not exists
	sql_execute!(
		[SqlStub {}, pool]
		"
		INSERT INTO _migrations (last_index, locked, tainted)
		SELECT 0, 0, 0
		WHERE NOT EXISTS (SELECT 1 FROM _migrations)
		",
	)
	.await
	.map_err(WorkflowError::into_migration_err)?;

	// Attempt to get lock on migrations table if migrations should be ran
	let migrations_row = sql_fetch_optional!(
		[SqlStub {}, (i64,), pool]
		"
		UPDATE _migrations
		SET locked = TRUE
		WHERE
			last_index < ? AND
			NOT locked AND
			NOT tainted
		RETURNING last_index
		",
		MIGRATIONS.last_migration_index,
	)
	.await
	.map_err(WorkflowError::into_migration_err)?;

	// Could not get lock
	let Some((last_index,)) = migrations_row else {
		let (locked, tainted) = sql_fetch_one!(
			[SqlStub {}, (bool, bool), pool]
			"
			SELECT locked, tainted FROM _migrations
			",
		)
		.await
		.map_err(WorkflowError::into_migration_err)?;

		let msg = if tainted {
			"tainted"
		} else if locked {
			"already locked"
		} else {
			// Already on latest migration
			return Ok(());
		};

		return Err(WorkflowError::MigrationLock(workflow_id, msg.to_string()));
	};

	tracing::debug!(?workflow_id, "running sqlite migrations");

	// Run migrations
	if let Err(err) = run_migrations(&pool, last_index).await {
		tracing::debug!(?workflow_id, "sqlite migrations failed");

		// Mark as tainted
		sql_execute!(
			[SqlStub {}, pool]
			"
			UPDATE _migrations
			SET
				locked = FALSE,
				tainted = TRUE
			",
		)
		.await
		.map_err(WorkflowError::into_migration_err)?;

		return Err(err);
	} else {
		tracing::debug!(?workflow_id, "sqlite migrations succeeded");

		// Ack latest migration
		sql_execute!(
			[SqlStub {}, pool]
			"
			UPDATE _migrations
			SET
				locked = FALSE,
				last_index = ?
			",
			MIGRATIONS.last_migration_index,
		)
		.await
		.map_err(WorkflowError::into_migration_err)?;
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_migrations(pool: &SqlitePool, last_index: i64) -> WorkflowResult<()> {
	let mut conn = pool.conn().await?;

	for (idx, file) in &*MIGRATIONS.files {
		// Skip already applied migrations
		if *idx <= last_index {
			continue;
		}

		let mut tx = conn.begin().await?;

		sql_execute!(
			[SqlStub {}, @tx &mut tx]
			file.contents_utf8().unwrap(),
		)
		.await
		.map_err(WorkflowError::into_migration_err)?;

		tx.commit().await?;
	}

	Ok(())
}

fn parse_migration_index(file: &File) -> i64 {
	file.path()
		.file_name()
		.unwrap()
		.to_str()
		.unwrap()
		.split_once('_')
		.expect("invalid migration name")
		.0
		.parse()
		.expect("invalid migration index")
}

/// Stores data for all event types in one.
#[derive(Debug, sqlx::FromRow)]
pub struct AmalgamEventRow {
	location: Location,
	version: i64,
	event_type: i64,
	name: Option<String>,
	auxiliary_id: Option<Uuid>,
	hash: Option<Vec<u8>>,
	input: Option<RawJson>,
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
		let event_type = value
			.event_type
			.try_into()
			.map_err(|_| WorkflowError::IntegerConversion)?;
		let event_type = EventType::from_repr(event_type)
			.ok_or_else(|| WorkflowError::InvalidEventType(value.event_type))?;

		Ok(Event {
			coordinate: value.location.tail().cloned().expect("empty location"),
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
			event_id: EventId::from_be_bytes(
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
			state: value.input.ok_or(WorkflowError::MissingEventData)?.0,
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
				.ok_or_else(|| WorkflowError::InvalidEventType(event_type))?,
		})
	}
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
pub fn build_history(
	event_rows: Vec<AmalgamEventRow>,
) -> WorkflowResult<HashMap<Location, Vec<Event>>> {
	let mut events_by_location: HashMap<Location, Vec<Event>> = HashMap::new();

	for event_row in event_rows {
		events_by_location
			.entry(event_row.location.root())
			.or_default()
			.push(event_row.try_into()?);
	}

	for events in events_by_location.values_mut() {
		// Events are already mostly sorted themselves so this should be fairly cheap
		events.sort_by_key(|event| event.coordinate().clone());
	}

	Ok(events_by_location)
}

/// Database name for the workflow internal state.
pub fn db_name_internal(workflow_id: Uuid) -> (usize, Uuid, usize) {
	(WORKFLOW, workflow_id, INTERNAL)
}
