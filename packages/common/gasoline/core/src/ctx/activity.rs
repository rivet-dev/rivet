use std::ops::Deref;

use anyhow::Result;
use rivet_util::Id;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use tracing::Instrument;

use crate::{
	ctx::{
		common,
		message::{MessageCtx, SubscriptionHandle},
	},
	db::DatabaseHandle,
	error::{WorkflowError, WorkflowResult},
	message::Message,
	operation::{Operation, OperationInput},
	utils::tags::AsTags,
	workflow::StateGuard,
};

pub struct ActivityCtx {
	workflow_id: Id,
	workflow_name: String,
	ray_id: Id,
	name: &'static str,
	create_ts: i64,
	ts: i64,
	parallelized: bool,

	db: DatabaseHandle,

	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	msg_ctx: MessageCtx,

	workflow_state: Mutex<(Box<serde_json::value::RawValue>, bool)>,
}

impl ActivityCtx {
	#[tracing::instrument(skip_all, fields(activity_name=%name))]
	pub fn new(
		workflow_id: Id,
		workflow_name: String,
		workflow_state: Box<serde_json::value::RawValue>,
		db: DatabaseHandle,
		config: &rivet_config::Config,
		pools: &rivet_pools::Pools,
		cache: &rivet_cache::Cache,
		activity_create_ts: i64,
		ray_id: Id,
		name: &'static str,
		parallelized: bool,
	) -> WorkflowResult<Self> {
		let msg_ctx = MessageCtx::new(config, pools, cache, ray_id)?;

		Ok(ActivityCtx {
			workflow_id,
			workflow_name,
			ray_id,
			name,
			create_ts: activity_create_ts,
			ts: rivet_util::timestamp::now(),
			parallelized,

			db,

			config: config.clone(),
			pools: pools.clone(),
			cache: cache.clone(),
			msg_ctx,

			workflow_state: Mutex::new((workflow_state, false)),
		})
	}
}

impl ActivityCtx {
	#[tracing::instrument(skip_all)]
	pub fn state<T: Serialize + DeserializeOwned>(&self) -> Result<StateGuard<'_, T>> {
		if self.parallelized {
			return Err(
				WorkflowError::WorkflowStateInaccessible("activity running in parallel").into(),
			);
		} else {
			StateGuard::new(
				self.workflow_state.try_lock().map_err(|_| {
					WorkflowError::WorkflowStateInaccessible("should not be locked")
				})?,
			)
		}
	}

	#[tracing::instrument(skip_all, fields(operation_name=I::Operation::NAME))]
	pub async fn op<I>(
		&self,
		input: I,
	) -> Result<<<I as OperationInput>::Operation as Operation>::Output>
	where
		I: OperationInput,
		<I as OperationInput>::Operation: Operation<Input = I>,
	{
		common::op(
			&self.db,
			&self.config,
			&self.pools,
			&self.cache,
			self.ray_id,
			true,
			input,
		)
		.in_current_span()
		.await
	}

	#[tracing::instrument(skip_all)]
	pub async fn update_workflow_tags(&self, tags: &serde_json::Value) -> Result<()> {
		self.db
			.update_workflow_tags(self.workflow_id, &self.workflow_name, tags)
			.await
			.map_err(Into::into)
	}

	/// IMPORTANT: This is intended for ephemeral realtime events and should be used carefully. Use
	/// signals if you need this to be durable.
	#[tracing::instrument(skip_all, fields(message=M::NAME))]
	pub async fn subscribe<M>(&self, tags: impl AsTags) -> Result<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(tags)
			.in_current_span()
			.await
			.map_err(Into::into)
	}

	pub(crate) fn into_new_workflow_state(self) -> Option<Box<serde_json::value::RawValue>> {
		let guard = self.workflow_state.into_inner();

		guard.1.then(|| guard.0)
	}
}

impl ActivityCtx {
	pub fn name(&self) -> &str {
		self.name
	}

	pub fn workflow_id(&self) -> Id {
		self.workflow_id
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	/// Timestamp at which the activity was first created.
	pub fn create_ts(&self) -> i64 {
		self.create_ts
	}

	/// Timestamp at which this current activity run started.
	pub fn ts(&self) -> i64 {
		self.ts
	}

	pub fn pools(&self) -> &rivet_pools::Pools {
		&self.pools
	}

	pub fn cache(&self) -> &rivet_cache::Cache {
		&self.cache
	}

	pub fn config(&self) -> &rivet_config::Config {
		&self.config
	}
}

impl Deref for ActivityCtx {
	type Target = rivet_pools::Pools;

	fn deref(&self) -> &Self::Target {
		&self.pools
	}
}
