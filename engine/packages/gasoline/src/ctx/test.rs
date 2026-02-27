use std::{ops::Deref, sync::Arc, time::Duration};
use tokio::{sync::watch, task::JoinHandle};

use anyhow::Result;
use rivet_util::Id;
use serde::Serialize;
use tracing::Instrument;

use crate::{
	builder::{WorkflowRepr, common as builder},
	ctx::{MessageCtx, common, message::SubscriptionHandle},
	db::{Database, DatabaseHandle, WorkflowData, debug::DatabaseDebug},
	message::Message,
	operation::{Operation, OperationInput},
	prelude::*,
	signal::Signal,
	utils::{tags::AsTags, topic::AsTopic},
	workflow::{Workflow, WorkflowInput},
};

pub struct TestCtx {
	name: String,
	ray_id: Id,
	ts: i64,

	db: DatabaseHandle,
	debug_db: Arc<dyn DatabaseDebug>,
	shutdown_tx: watch::Sender<()>,
	pub test_deps: rivet_test_deps::TestDeps,
	worker_handle: Option<JoinHandle<Result<()>>>,

	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	msg_ctx: MessageCtx,
}

impl TestCtx {
	pub async fn new(reg: Registry) -> Result<TestCtx> {
		let test_deps = rivet_test_deps::TestDeps::new().await?;
		Self::new_with_deps(reg, test_deps).await
	}

	pub async fn new_with_deps(
		reg: Registry,
		test_deps: rivet_test_deps::TestDeps,
	) -> Result<Self> {
		setup_logging();

		tracing::info!("setting up gasoline test environment");

		let config = test_deps.config().clone();
		let pools = test_deps.pools().clone();

		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())
			.expect("failed to create cache");

		let db = db::DatabaseKv::new(config.clone(), pools.clone()).await?;
		let debug_db =
			db::DatabaseKv::new(config.clone(), pools.clone()).await? as Arc<dyn DatabaseDebug>;

		let service_name = format!("{}-test--{}", rivet_env::service_name(), "gasoline_test");
		let ray_id = Id::new_v1(config.dc_label());

		let msg_ctx = MessageCtx::new(&config, &pools, &cache, ray_id)?;

		let worker = Worker::new(reg.handle(), db.clone(), config.clone(), pools.clone());
		let (shutdown_tx, shutdown_rx) = watch::channel(());

		tracing::info!("starting workflow worker");
		let worker_handle = tokio::spawn(worker.start(Some(shutdown_rx)));

		// Give the worker time to start up
		tokio::time::sleep(Duration::from_millis(500)).await;

		tracing::info!("test environment setup complete");
		Ok(TestCtx {
			name: service_name,
			ray_id,
			ts: rivet_util::timestamp::now(),
			db,
			debug_db,
			shutdown_tx,
			test_deps,
			worker_handle: Some(worker_handle),
			config,
			pools,
			cache,
			msg_ctx,
		})
	}

	pub fn debug_db(&self) -> &dyn DatabaseDebug {
		&*self.debug_db
	}

	pub fn standalone(&self) -> Result<StandaloneCtx> {
		StandaloneCtx::new(
			self.db.clone(),
			self.config.clone(),
			self.pools.clone(),
			self.cache.clone(),
			&self.name,
			self.ray_id,
			Id::new_v1(self.config.dc_label()),
		)
		.map_err(Into::into)
	}

	pub async fn shutdown(&mut self) -> Result<()> {
		if let Some(worker_handle) = self.worker_handle.take() {
			tracing::info!("stopping workflow worker");

			// Trigger shutdown
			self.shutdown_tx.send(())?;

			// Wait for the worker to finish its shutdown sequence
			//
			// This ensures that `Worker::shutdown` has been called successfully
			tracing::info!("waiting for workflow worker handle to finish");
			match worker_handle.await {
				Ok(result) => {
					if let Err(err) = result {
						tracing::warn!(?err, "worker stopped with error");
					}
				}
				Err(err) => {
					tracing::warn!(?err, "worker task join error");
				}
			}

			tracing::info!("workflow worker stopped");
		}
		Ok(())
	}
}

impl TestCtx {
	/// Creates a workflow builder.
	pub fn workflow<I>(
		&self,
		input: impl WorkflowRepr<I>,
	) -> builder::workflow::WorkflowBuilder<impl WorkflowRepr<I>, I>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		builder::workflow::WorkflowBuilder::new(
			self.db.clone(),
			self.config.clone(),
			self.ray_id,
			input,
			false,
		)
	}

	/// Finds the first incomplete workflow with the given tags.
	#[tracing::instrument(skip_all, ret(Debug), fields(workflow_name=W::NAME))]
	pub async fn find_workflow<W: Workflow>(&self, tags: impl AsTags) -> Result<Option<Id>> {
		common::find_workflow::<W>(&self.db, tags)
			.in_current_span()
			.await
	}

	/// Finds the first incomplete workflow for each (name, tags) pair in a single batch transaction.
	#[tracing::instrument(skip_all)]
	pub async fn find_workflows(
		&self,
		queries: &[(&str, serde_json::Value)],
	) -> Result<Vec<Option<Id>>> {
		common::find_workflows(&self.db, queries)
			.in_current_span()
			.await
	}

	/// Finds the first incomplete workflow with the given tags.
	#[tracing::instrument(skip_all)]
	pub async fn get_workflows(&self, workflow_ids: Vec<Id>) -> Result<Vec<WorkflowData>> {
		common::get_workflows(&self.db, workflow_ids)
			.in_current_span()
			.await
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(
			self.db.clone(),
			self.config.clone(),
			self.ray_id,
			body,
			false,
		)
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
			false,
			input,
		)
		.in_current_span()
		.await
	}

	pub fn msg<M: Message>(&self, body: M) -> builder::message::MessageBuilder<M> {
		builder::message::MessageBuilder::new(self.msg_ctx.clone(), body)
	}

	#[tracing::instrument(skip_all, fields(message=M::NAME, topic=%topic.as_topic()))]
	pub async fn subscribe<M>(&self, topic: impl AsTopic) -> Result<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.msg_ctx
			.subscribe::<M>(&topic.as_topic())
			.in_current_span()
			.await
			.map_err(Into::into)
	}
}

impl TestCtx {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	/// Timestamp at which the request started.
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

impl Deref for TestCtx {
	type Target = rivet_pools::Pools;

	fn deref(&self) -> &Self::Target {
		&self.pools
	}
}

pub fn setup_logging() {
	// Set up logging
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_ansi(true)
		.with_test_writer()
		.try_init();
}
