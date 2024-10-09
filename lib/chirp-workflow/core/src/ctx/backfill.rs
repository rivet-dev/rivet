use std::{collections::HashMap, sync::Arc};

use futures_util::FutureExt;
use global_error::unwrap_ref;
use global_error::GlobalResult;
use indoc::indoc;
use serde::Serialize;
use std::{
	collections::hash_map::DefaultHasher,
	future::Future,
	hash::{Hash, Hasher},
	pin::Pin,
};
use uuid::Uuid;

use crate::history::{cursor::Cursor, location::Location};

// Yes
type Query = Box<
	dyn for<'a> FnOnce(
		&'a mut sqlx::Transaction<'_, sqlx::Postgres>,
	) -> Pin<Box<dyn Future<Output = GlobalResult<()>> + Send + 'a>>,
>;

pub struct BackfillCtx {
	queries: Vec<Query>,
}

impl BackfillCtx {
	pub fn new() -> Self {
		BackfillCtx {
			queries: Vec::new(),
		}
	}

	pub async fn execute(self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> GlobalResult<()> {
		tracing::info!(queries=%self.queries.len(), "executing backfill queries");

		for query in self.queries {
			query(tx).await?;
		}

		Ok(())
	}
}

impl BackfillCtx {
	pub fn workflow<F>(&mut self, workflow_name: &str, builder: F) -> GlobalResult<Uuid>
	where
		F: Fn(&mut WorkflowBackfillCtx) -> GlobalResult<()>,
	{
		let mut wf_ctx = WorkflowBackfillCtx::new(workflow_name);
		let workflow_id = wf_ctx.workflow_id;

		builder(&mut wf_ctx)?;

		self.queries.extend(wf_ctx.queries);

		Ok(workflow_id)
	}

	pub fn existing_workflow<F>(&mut self, workflow_id: Uuid, builder: F) -> GlobalResult<()>
	where
		F: Fn(&mut WorkflowBackfillCtx) -> GlobalResult<()>,
	{
		let mut wf_ctx = WorkflowBackfillCtx::new("");
		wf_ctx.workflow_id = workflow_id;

		builder(&mut wf_ctx)?;

		self.queries.extend(wf_ctx.queries);

		Ok(())
	}
}

pub struct WorkflowBackfillCtx {
	workflow_id: Uuid,
	workflow_name: String,

	cursor: Cursor,

	tags: Option<serde_json::Value>,
	input: Option<serde_json::Value>,
	output: Option<serde_json::Value>,

	queries: Vec<Query>,
}

impl WorkflowBackfillCtx {
	fn new(workflow_name: &str) -> Self {
		WorkflowBackfillCtx {
			workflow_id: Uuid::new_v4(),
			workflow_name: workflow_name.to_string(),

			cursor: Cursor::new(Arc::new(HashMap::new()), Location::empty()),

			tags: None,
			input: None,
			output: None,

			queries: Vec::new(),
		}
	}

	fn branch(&mut self) -> Self {
		let branch = WorkflowBackfillCtx {
			workflow_id: self.workflow_id,
			workflow_name: self.workflow_name.clone(),

			cursor: Cursor::new(Arc::new(HashMap::new()), self.cursor.current_location()),

			tags: None,
			input: None,
			output: None,

			queries: Vec::new(),
		};

		self.cursor.inc();

		branch
	}

	// TODO:
	// pub fn set_location(&mut self, location: &Location) {
	// }

	pub fn finalize(&mut self) {
		let wake_immediate = true;

		let workflow_id = self.workflow_id;
		let workflow_name = self.workflow_name.clone();
		let tags = std::mem::replace(&mut self.tags, Default::default());
		let input = std::mem::replace(&mut self.input, Default::default());
		let output = std::mem::replace(&mut self.output, Default::default());

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
				INSERT INTO db_workflow.workflows (
					workflow_id, workflow_name, create_ts, ray_id, tags, input, output, wake_immediate
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
				"
				))
				.bind(workflow_id)
				.bind(workflow_name)
				.bind(rivet_util::timestamp::now())
				.bind(Uuid::new_v4())
				.bind(tags)
				.bind(unwrap_ref!(input, "workflow backfill must have input"))
				.bind(output)
				.bind(wake_immediate)
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));
	}
}

impl WorkflowBackfillCtx {
	pub fn tags<T: Serialize>(&mut self, tags: T) -> GlobalResult<()> {
		self.tags = Some(serde_json::to_value(&tags)?);

		Ok(())
	}

	pub fn input<T: Serialize>(&mut self, input: T) -> GlobalResult<()> {
		self.input = Some(serde_json::to_value(&input)?);

		Ok(())
	}

	pub fn output<T: Serialize>(&mut self, output: T) -> GlobalResult<()> {
		self.output = Some(serde_json::to_value(&output)?);

		Ok(())
	}

	pub fn activity<T: Serialize + Hash + Send + 'static, U: Serialize + Send + 'static>(
		&mut self,
		activity_name: &str,
		input: T,
		output: U,
	) -> GlobalResult<()> {
		let mut hasher = DefaultHasher::new();
		input.hash(&mut hasher);
		let input_hash = hasher.finish();

		let workflow_id = self.workflow_id;
		let location = self.cursor.current_location();
		let activity_name = activity_name.to_string();

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
				INSERT INTO db_workflow.workflow_activity_events (
					workflow_id, location, activity_name, input_hash, input, output, create_ts
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7)
				",
				))
				.bind(workflow_id)
				.bind(location)
				.bind(activity_name)
				.bind(input_hash.to_le_bytes())
				.bind(sqlx::types::Json(serde_json::value::to_raw_value(&input)?))
				.bind(sqlx::types::Json(serde_json::value::to_raw_value(&output)?))
				.bind(rivet_util::timestamp::now())
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));

		self.cursor.inc();

		Ok(())
	}

	pub fn message<T: Serialize + Send + 'static>(
		&mut self,
		message_name: &str,
		tags: serde_json::Value,
		body: T,
	) -> GlobalResult<()> {
		let workflow_id = self.workflow_id;
		let location = self.cursor.current_location();
		let message_name = message_name.to_string();

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
				INSERT INTO db_workflow.workflow_message_send_events (
					workflow_id, location, tags, message_name, body
				)
				VALUES ($1, $2, $3, $4, $5)
				"
				))
				.bind(workflow_id)
				.bind(location)
				.bind(&tags)
				.bind(message_name)
				.bind(serde_json::to_value(&body)?)
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));

		self.cursor.inc();

		Ok(())
	}

	pub fn signal<T: Serialize + Send + 'static>(
		&mut self,
		signal_name: &str,
		body: T,
	) -> GlobalResult<()> {
		let workflow_id = self.workflow_id;
		let location = self.cursor.current_location();
		let signal_name = signal_name.to_string();

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
					INSERT INTO db_workflow.workflow_signal_send_events (
						workflow_id, location, signal_id, signal_name, body
					)
					VALUES ($1, $2, $3, $4, $5)
					"
				))
				.bind(workflow_id)
				.bind(location)
				.bind(Uuid::new_v4())
				.bind(signal_name)
				.bind(serde_json::to_value(&body)?)
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));

		self.cursor.inc();

		Ok(())
	}

	pub fn sub_workflow<F>(&mut self, builder: F) -> GlobalResult<()>
	where
		F: Fn(&mut WorkflowBackfillCtx) -> GlobalResult<()>,
	{
		let mut swf_ctx = self.branch();

		builder(&mut swf_ctx)?;

		self.queries.extend(swf_ctx.queries);

		Ok(())
	}

	pub fn dispatch_sub_workflow(&mut self, sub_workflow_id: Uuid) -> GlobalResult<()> {
		let workflow_id = self.workflow_id;
		let location = self.cursor.current_location();

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
					INSERT INTO db_workflow.workflow_sub_workflow_events(
						workflow_id, location, sub_workflow_id, create_ts
					)
					VALUES($1, $2, $3, $4)
					"
				))
				.bind(workflow_id)
				.bind(location)
				.bind(sub_workflow_id)
				.bind(rivet_util::timestamp::now())
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));

		self.cursor.inc();

		Ok(())
	}

	pub fn listen<T: Serialize + Send + 'static>(
		&mut self,
		signal_name: &str,
		body: T,
	) -> GlobalResult<()> {
		let workflow_id = self.workflow_id;
		let location = self.cursor.current_location();
		let signal_name = signal_name.to_string();

		self.queries.push(Box::new(move |tx| {
			async move {
				sqlx::query(indoc!(
					"
					INSERT INTO db_workflow.workflow_signal_events (
						workflow_id, location, signal_id, signal_name, body, ack_ts
					)
					VALUES ($1, $2, $3, $4, $5, $6)
					"
				))
				.bind(workflow_id)
				.bind(location)
				.bind(Uuid::new_v4())
				.bind(signal_name)
				.bind(serde_json::to_value(&body)?)
				.bind(rivet_util::timestamp::now())
				.execute(&mut **tx)
				.await?;

				Ok(())
			}
			.boxed()
		}));

		self.cursor.inc();

		Ok(())
	}

	// TODO: Loop
}
