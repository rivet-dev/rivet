//! Implementation of a workflow database driver with SQLite and FoundationDB.
// TODO: Move code to smaller functions for readability

use std::{
	collections::{HashMap, HashSet},
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Instant,
};

use fdb_util::{end_of_key_range, keys::*, FormalChunkedKey, FormalKey, SERIALIZABLE, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
};
use futures_util::{stream::BoxStream, StreamExt, TryStreamExt};
use indoc::indoc;
use rivet_pools::prelude::*;
use rivet_util::future::CustomInstrumentExt;
use serde_json::json;
use sqlx::Acquire;
use tokio::sync::mpsc;
use tracing::Instrument;
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
mod keys;
mod sqlite;

// HACK: We alias global error here because its hardcoded into the sql macros
type GlobalError = WorkflowError;

/// Base retry for query retry backoff.
const QUERY_RETRY_MS: usize = 500;
/// Maximum times a query ran by this database adapter is retried.
const MAX_QUERY_RETRIES: usize = 4;
/// How long before considering the leases of a given worker instance expired.
/// IMPORTANT: Must always be greater than worker::INTERNAL_INTERVAL or else all workers will end up lost.
const WORKER_INSTANCE_LOST_THRESHOLD_MS: i64 = rivet_util::duration::seconds(30);
/// How long before overwriting an existing metrics lock.
const METRICS_LOCK_TIMEOUT_MS: i64 = rivet_util::duration::seconds(30);
/// For SQL macros.
const CONTEXT_NAME: &str = "chirp_workflow_fdb_sqlite_nats_engine";
/// For NATS wake mechanism.
const WORKER_WAKE_SUBJECT: &str = "chirp.workflow.fdb_sqlite_nats.worker.wake";

pub struct DatabaseFdbSqliteNats {
	pools: rivet_pools::Pools,
	subspace: fdb_util::Subspace,
	flush_tx: mpsc::UnboundedSender<Uuid>,
}

impl DatabaseFdbSqliteNats {
	// For SQL macros
	fn name(&self) -> &str {
		CONTEXT_NAME
	}

	/// Spawns a new thread and publishes a worker wake message to nats.
	fn wake_worker(&self) {
		let Ok(nats) = self.pools.nats() else {
			tracing::debug!("failed to acquire nats pool");
			return;
		};

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
}

// MARK: Sqlite
impl DatabaseFdbSqliteNats {
	/// Executes queries while explicitly handling txn retry errors.
	#[tracing::instrument(skip_all)]
	async fn txn<'a, F, Fut, T>(&self, mut cb: F) -> WorkflowResult<T>
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
						return Err(WorkflowError::Sqlx(sqlx::Error::Io(std::io::Error::new(
							std::io::ErrorKind::Other,
							rivet_pools::utils::sql_query_macros::Error::MaxSqlRetries(err),
						))));
					}

					use sqlx::Error::*;
					match &err {
						// Retry all errors with a backoff
						Database(_) | Io(_) | Tls(_) | Protocol(_) | PoolTimedOut | PoolClosed
						| WorkerCrashed => {
							tracing::warn!(?err, "txn retry");
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

	/// Evicts all SQLite databases related to this workflow from this node.
	///
	/// This must be done before releasing the lease on the workflow in order to prevent a race
	/// condition with other workflow workers picking it up.
	#[tracing::instrument(skip_all)]
	async fn evict_wf_sqlite(&self, workflow_id: Uuid) -> WorkflowResult<()> {
		tracing::debug!(?workflow_id, "evicting workflow");

		self.pools
			.sqlite_manager()
			.evict(vec![
				crate::db::sqlite_db_name_internal(workflow_id),
				crate::db::sqlite_db_name_data(workflow_id),
			])
			.await?;

		Ok(())
	}

	/// Notifies a background thread to flush the workflow's databases. Non-blocking.
	/// Only needed for events that have side effects: activities, signals, messages, and workflows.
	#[tracing::instrument(skip_all)]
	fn flush_wf_sqlite(&self, workflow_id: Uuid) -> WorkflowResult<()> {
		self.flush_tx
			.send(workflow_id)
			.map_err(|_| WorkflowError::FlushChannelClosed)
	}
}

// MARK: FDB
impl DatabaseFdbSqliteNats {
	fn write_signal_wake_idxs(
		&self,
		workflow_id: Uuid,
		wake_signals: &[&str],
		tx: &fdb::RetryableTransaction,
	) -> Result<(), fdb::FdbBindingError> {
		for signal_name in wake_signals {
			// Write to wake signals list
			let wake_signal_key =
				keys::workflow::WakeSignalKey::new(workflow_id, signal_name.to_string());
			tx.set(
				&self.subspace.pack(&wake_signal_key),
				&wake_signal_key
					.serialize(())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);
		}

		Ok(())
	}

	fn write_sub_workflow_wake_idx(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		sub_workflow_id: Uuid,
		tx: &fdb::RetryableTransaction,
	) -> Result<(), fdb::FdbBindingError> {
		let sub_workflow_wake_key =
			keys::wake::SubWorkflowWakeKey::new(sub_workflow_id, workflow_id);

		tx.set(
			&self.subspace.pack(&sub_workflow_wake_key),
			&sub_workflow_wake_key
				.serialize(workflow_name.to_string())
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
		);

		Ok(())
	}
}

#[async_trait::async_trait]
impl Database for DatabaseFdbSqliteNats {
	fn from_pools(pools: rivet_pools::Pools) -> Result<Arc<Self>, rivet_pools::Error> {
		// Start background flush handler task
		let (flush_tx, flush_rx) = mpsc::unbounded_channel();
		tokio::spawn(flush_handler(pools.clone(), flush_rx));

		Ok(Arc::new(DatabaseFdbSqliteNats {
			pools,
			subspace: fdb_util::Subspace::new(&(RIVET, CHIRP_WORKFLOW, FDB_SQLITE_NATS)),
			flush_tx,
		}))
	}

	#[tracing::instrument(skip_all)]
	async fn wake_sub<'a, 'b>(&'a self) -> WorkflowResult<BoxStream<'b, ()>> {
		let stream = self
			.pools
			.nats()?
			.subscribe(WORKER_WAKE_SUBJECT)
			.await
			.map_err(|x| WorkflowError::CreateSubscription(x.into()))?
			.map(|_| ());

		Ok(stream.boxed())
	}

	#[tracing::instrument(skip_all)]
	async fn clear_expired_leases(&self, _worker_instance_id: Uuid) -> WorkflowResult<()> {
		let (lost_worker_instance_ids, expired_workflow_count) = self
			.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let now = rivet_util::timestamp::now();

					let mut last_ping_cache: Vec<(Uuid, i64)> = Vec::new();
					let mut lost_worker_instance_ids = HashSet::new();
					let mut expired_workflow_count = 0;

					let lease_subspace = self
						.subspace
						.subspace(&keys::workflow::LeaseKey::subspace());

					// List all active leases
					let mut stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&lease_subspace).into()
						},
						// Not SERIALIZABLE because we don't want this to conflict with other queries which write
						// leases
						SNAPSHOT,
					);

					while let Some(lease_key_entry) = stream.try_next().await? {
						let lease_key = self
							.subspace
							.unpack::<keys::workflow::LeaseKey>(lease_key_entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
						let (workflow_name, worker_instance_id) = lease_key
							.deserialize(lease_key_entry.value())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
						let last_ping_ts_key =
							keys::worker_instance::LastPingTsKey::new(worker_instance_id);

						// Get last ping of worker instance for this lease
						let last_ping_ts = if let Some((_, last_ping_ts)) = last_ping_cache
							.iter()
							.find(|(k, _)| k == &worker_instance_id)
						{
							*last_ping_ts
						} else if let Some(last_ping_entry) = tx
							.get(
								&self.subspace.pack(&last_ping_ts_key),
								// Not SERIALIZABLE because we don't want this to conflict
								SNAPSHOT,
							)
							.await?
						{
							// Deserialize last ping value
							let last_ping_ts = last_ping_ts_key
								.deserialize(&last_ping_entry)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

							// Update cache
							last_ping_cache.push((worker_instance_id, last_ping_ts));

							last_ping_ts
						} else {
							// Update cache
							last_ping_cache.push((worker_instance_id, 0));

							0
						};

						// Worker has not pinged within the threshold, meaning the lease is expired
						if last_ping_ts < now - WORKER_INSTANCE_LOST_THRESHOLD_MS {
							// Check if the workflow is silenced and ignore
							let silence_ts_key =
								keys::workflow::SilenceTsKey::new(lease_key.workflow_id);
							if tx
								.get(&self.subspace.pack(&silence_ts_key), SERIALIZABLE)
								.await?
								.is_some()
							{
								continue;
							}

							// NOTE: We add a read conflict here so this query conflicts with any other
							// `clear_expired_leases` queries running at the same time (will conflict with the
							// following `tx.clear`).
							tx.add_conflict_range(
								lease_key_entry.key(),
								&end_of_key_range(lease_key_entry.key()),
								ConflictRangeType::Read,
							)?;

							// Clear lease
							tx.clear(lease_key_entry.key());
							let worker_instance_id_key =
								keys::workflow::WorkerInstanceIdKey::new(lease_key.workflow_id);
							tx.clear(&self.subspace.pack(&worker_instance_id_key));

							// Add immediate wake for workflow
							let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
								workflow_name.to_string(),
								lease_key.workflow_id,
								keys::wake::WakeCondition::Immediate,
							);
							tx.set(
								&self.subspace.pack(&wake_condition_key),
								&wake_condition_key
									.serialize(())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							);

							expired_workflow_count += 1;
							lost_worker_instance_ids.insert(worker_instance_id);

							tracing::debug!(?lease_key.workflow_id, "failed over wf");
						}
					}

					Ok((lost_worker_instance_ids, expired_workflow_count))
				}
			})
			.custom_instrument(tracing::info_span!("clear_expired_leases_tx"))
			.await?;

		if expired_workflow_count != 0 {
			tracing::info!(
				worker_instance_ids=?lost_worker_instance_ids,
				total_workflows=%expired_workflow_count,
				"handled failover",
			);

			self.wake_worker();
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_metrics(&self, _worker_instance_id: Uuid) -> WorkflowResult<()> {
		// Attempt to be the only worker publishing metrics by writing to the lock key
		let acquired_lock = self
			.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let metrics_lock_key = keys::worker_instance::MetricsLockKey::new();

					// Read existing lock
					let lock_expired = if let Some(entry) = tx
						.get(&self.subspace.pack(&metrics_lock_key), SERIALIZABLE)
						.await?
					{
						let lock_ts = metrics_lock_key
							.deserialize(&entry)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						lock_ts < rivet_util::timestamp::now() - METRICS_LOCK_TIMEOUT_MS
					} else {
						true
					};

					if lock_expired {
						// Write to lock key. FDB transactions guarantee that if multiple workers are running this
						// query at the same time only one will succeed which means only one will have the lock.
						tx.set(
							&self.subspace.pack(&metrics_lock_key),
							&metrics_lock_key
								.serialize(rivet_util::timestamp::now())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					Ok(lock_expired)
				}
			})
			.custom_instrument(tracing::info_span!("acquire_lock_tx"))
			.await?;

		if acquired_lock {
			let (other_workflow_counts, dead_workflow_count, pending_signal_count) = self
				.pools
				.fdb()?
				.run(|tx, _mc| {
					async move {
						// Get rid of metrics that don't exist in the db anymore (declarative)
						metrics::WORKFLOW_TOTAL.reset();
						metrics::WORKFLOW_ACTIVE.reset();
						metrics::WORKFLOW_DEAD.reset();
						metrics::WORKFLOW_SLEEPING.reset();
						metrics::SIGNAL_PENDING.reset();

						let wf_data_subspace = self
							.subspace
							.subspace(&keys::workflow::DataSubspaceKey::new());
						let pending_signal_subspace = self
							.subspace
							.subspace(&keys::workflow::EntirePendingSignalSubspaceKey::new());

						// Not SERIALIZABLE because we don't want these to conflict with other queries, they're
						// just for metrics
						let mut wf_stream = tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::Iterator,
								..(&wf_data_subspace).into()
							},
							SNAPSHOT,
						);
						let mut signal_stream = tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::Iterator,
								..(&pending_signal_subspace).into()
							},
							SNAPSHOT,
						);

						let mut other_workflow_counts = HashMap::<String, WorkflowMetrics>::new();
						let mut dead_workflow_count = HashMap::<(String, String), i64>::new();
						let mut pending_signal_count = HashMap::<String, i64>::new();

						tokio::try_join!(
							async {
								let mut current_workflow_id = None;
								let mut current_workflow_name: Option<String> = None;
								let mut current_error: Option<String> = None;
								let mut current_state = WorkflowState::Dead;

								while let Some(entry) = wf_stream.try_next().await? {
									let workflow_id = *self
										.subspace
										.unpack::<debug::JustUuid>(entry.key())
										.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

									if let Some(curr) = current_workflow_id {
										if workflow_id != curr {
											if let Some(workflow_name) = &current_workflow_name {
												let entry = other_workflow_counts
													.entry(workflow_name.clone())
													.or_default();

												match current_state {
													WorkflowState::Complete => entry.complete += 1,
													WorkflowState::Running => entry.running += 1,
													WorkflowState::Sleeping => entry.sleeping += 1,
													WorkflowState::Dead => {
														entry.dead += 1;

														if let Some(error) = current_error {
															let entry = dead_workflow_count
																.entry((
																	workflow_name.clone(),
																	error.clone(),
																))
																.or_default();

															*entry += 1;
														}
													}
													// Ignore
													WorkflowState::Silenced => {}
												}

												current_workflow_name = None;
												current_error = None;
												current_state = WorkflowState::Dead;
											}
										}
									}

									current_workflow_id = Some(workflow_id);

									if let Ok(workflow_name_key) =
										self.subspace.unpack::<keys::workflow::NameKey>(entry.key())
									{
										let workflow_name =
											workflow_name_key.deserialize(entry.value()).map_err(
												|x| fdb::FdbBindingError::CustomError(x.into()),
											)?;

										current_workflow_name = Some(workflow_name);
									} else if let Ok(error_key) =
										self.subspace
											.unpack::<keys::workflow::ErrorKey>(entry.key())
									{
										let error =
											error_key.deserialize(entry.value()).map_err(|x| {
												fdb::FdbBindingError::CustomError(x.into())
											})?;

										current_error = Some(error);
									} else if !matches!(
										current_state,
										WorkflowState::Silenced | WorkflowState::Complete
									) {
										if let Ok(_) = self
											.subspace
											.unpack::<keys::workflow::OutputChunkKey>(entry.key())
										{
											current_state = WorkflowState::Complete;
										} else if let Ok(_) = self
											.subspace
											.unpack::<keys::workflow::HasWakeConditionKey>(
											entry.key(),
										) {
											current_state = WorkflowState::Sleeping;
										} else if let Ok(_) = self
											.subspace
											.unpack::<keys::workflow::WorkerInstanceIdKey>(
											entry.key(),
										) {
											current_state = WorkflowState::Running;
										}
									} else if let Ok(_) = self
										.subspace
										.unpack::<keys::workflow::SilenceTsKey>(entry.key())
									{
										current_state = WorkflowState::Silenced;
									}
								}

								if let Some(workflow_name) = current_workflow_name {
									let entry = other_workflow_counts
										.entry(workflow_name.clone())
										.or_default();

									match current_state {
										WorkflowState::Complete => entry.complete += 1,
										WorkflowState::Running => entry.running += 1,
										WorkflowState::Sleeping => entry.sleeping += 1,
										WorkflowState::Dead => {
											entry.dead += 1;

											if let Some(error) = current_error {
												let entry = dead_workflow_count
													.entry((workflow_name, error.clone()))
													.or_default();

												*entry += 1;
											}
										}
										// Ignore
										WorkflowState::Silenced => {}
									}
								}

								Result::<_, fdb::FdbBindingError>::Ok(())
							},
							async {
								// TODO: Parallelize
								while let Some(entry) = signal_stream.try_next().await? {
									if let Ok(pending_signal_key) =
										self.subspace
											.unpack::<keys::workflow::PendingSignalKey>(entry.key())
									{
										let silence_ts_key = keys::signal::SilenceTsKey::new(
											pending_signal_key.signal_id,
										);

										// Not silenced
										if tx
											.get(&self.subspace.pack(&silence_ts_key), SNAPSHOT)
											.await?
											.is_none()
										{
											let entry = pending_signal_count
												.entry(pending_signal_key.signal_name)
												.or_default();
											*entry += 1;
										}
									}
								}

								Ok(())
							},
						)?;

						Ok((
							other_workflow_counts,
							dead_workflow_count,
							pending_signal_count,
						))
					}
				})
				.custom_instrument(tracing::info_span!("publish_metrics_tx"))
				.await?;

			for (workflow_name, counts) in other_workflow_counts {
				metrics::WORKFLOW_TOTAL
					.with_label_values(&[&workflow_name])
					.set(counts.complete + counts.running + counts.sleeping + counts.dead);
				metrics::WORKFLOW_ACTIVE
					.with_label_values(&[&workflow_name])
					.set(counts.running);
				metrics::WORKFLOW_SLEEPING
					.with_label_values(&[&workflow_name])
					.set(counts.sleeping);
			}

			for ((workflow_name, error), count) in dead_workflow_count {
				metrics::WORKFLOW_DEAD
					.with_label_values(&[&workflow_name, &error])
					.set(count);
			}

			for (signal_name, count) in pending_signal_count {
				metrics::SIGNAL_PENDING
					.with_label_values(&[&signal_name])
					.set(count);
			}

			// Clear lock
			self.pools
				.fdb()?
				.run(|tx, _mc| async move {
					let metrics_lock_key = keys::worker_instance::MetricsLockKey::new();
					tx.clear(&self.subspace.pack(&metrics_lock_key));

					Ok(())
				})
				.custom_instrument(tracing::info_span!("clear_lock_tx"))
				.await?;
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn update_worker_ping(&self, worker_instance_id: Uuid) -> WorkflowResult<()> {
		metrics::WORKER_LAST_PING
			.with_label_values(&[&worker_instance_id.to_string()])
			.set(rivet_util::timestamp::now());

		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					// Update worker instance ping
					let last_ping_ts_key =
						keys::worker_instance::LastPingTsKey::new(worker_instance_id);
					tx.set(
						&self.subspace.pack(&last_ping_ts_key),
						&last_ping_ts_key
							.serialize(rivet_util::timestamp::now())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("update_worker_ping_tx"))
			.await?;

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
		// TODO: Race condition if two unique dispatch_workflow calls are made at the same time. The txn
		// inside of find_workflow should be split into find_workflow_inner and run in the same txn in this
		// function
		if unique {
			let empty_tags = json!({});

			if let Some(existing_workflow_id) = self
				.find_workflow(workflow_name, tags.unwrap_or(&empty_tags))
				.await?
			{
				return Ok(existing_workflow_id);
			}
		}

		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					// Write create ts
					let create_ts_key = keys::workflow::CreateTsKey::new(workflow_id);
					tx.set(
						&self.subspace.pack(&create_ts_key),
						&create_ts_key
							.serialize(rivet_util::timestamp::now())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write name
					let name_key = keys::workflow::NameKey::new(workflow_id);
					tx.set(
						&self.subspace.pack(&name_key),
						&name_key
							.serialize(workflow_name.to_string())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write ray id
					let ray_id_key = keys::workflow::RayIdKey::new(workflow_id);
					tx.set(
						&self.subspace.pack(&ray_id_key),
						&ray_id_key
							.serialize(ray_id)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write tags
					let tags = tags
						.map(|x| {
							x.as_object().ok_or_else(|| {
								WorkflowError::InvalidTags("must be an object".to_string())
							})
						})
						.transpose()
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.flatten()
						.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
						.collect::<WorkflowResult<Vec<_>>>()
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					for (k, v) in &tags {
						// Write tag key
						let tag_key =
							keys::workflow::TagKey::new(workflow_id, k.clone(), v.clone());
						tx.set(
							&self.subspace.pack(&tag_key),
							&tag_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);

						// Write "by name and first tag" secondary index
						let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::new(
							workflow_name.to_string(),
							k.clone(),
							v.clone(),
							workflow_id,
						);
						let rest_of_tags = tags
							.iter()
							.filter(|(k2, _)| k2 != k)
							.map(|(k, v)| (k.clone(), v.clone()))
							.collect();
						tx.set(
							&self.subspace.pack(&by_name_and_tag_key),
							&by_name_and_tag_key
								.serialize(rest_of_tags)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					// Write null key for the "by name and first tag" secondary index (all workflows have this)
					{
						// Write secondary index by name and first tag
						let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::null(
							workflow_name.to_string(),
							workflow_id,
						);
						tx.set(
							&self.subspace.pack(&by_name_and_tag_key),
							&by_name_and_tag_key
								.serialize(tags)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					// Wrote "has wake condition"
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(workflow_id);
					tx.set(
						&self.subspace.pack(&has_wake_condition_key),
						&has_wake_condition_key
							.serialize(())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write input
					let input_key = keys::workflow::InputKey::new(workflow_id);

					for (i, chunk) in input_key
						.split_ref(input)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.enumerate()
					{
						let chunk_key = input_key.chunk(i);

						tx.set(&self.subspace.pack(&chunk_key), &chunk);
					}

					// Write immediate wake condition
					let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
						workflow_name.to_string(),
						workflow_id,
						keys::wake::WakeCondition::Immediate,
					);

					tx.set(
						&self.subspace.pack(&wake_condition_key),
						&wake_condition_key
							.serialize(())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("dispatch_workflow_tx"))
			.await?;

		self.wake_worker();

		Ok(workflow_id)
	}

	#[tracing::instrument(skip_all, fields(%workflow_id))]
	async fn get_workflow(&self, workflow_id: Uuid) -> WorkflowResult<Option<WorkflowData>> {
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let input_key = keys::workflow::InputKey::new(workflow_id);
					let input_subspace = self.subspace.subspace(&input_key);
					let output_key = keys::workflow::OutputKey::new(workflow_id);
					let output_subspace = self.subspace.subspace(&output_key);
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(workflow_id);

					// Read input and output
					let (input_chunks, output_chunks, has_wake_condition_entry) = tokio::try_join!(
						tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&input_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&output_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get(&self.subspace.pack(&has_wake_condition_key), SERIALIZABLE),
					)?;

					if input_chunks.is_empty() {
						Ok(None)
					} else {
						let input = input_key
							.combine(input_chunks)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let output = if output_chunks.is_empty() {
							None
						} else {
							Some(
								output_key
									.combine(output_chunks)
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							)
						};

						Ok(Some(WorkflowData {
							workflow_id,
							input,
							output,
							has_wake_condition: has_wake_condition_entry.is_some(),
						}))
					}
				}
			})
			.custom_instrument(tracing::info_span!("get_workflow_tx"))
			.await
			.map_err(Into::into)
	}

	/// Returns the first incomplete workflow with the given name and tags, first meaning the one with the
	/// lowest uuid value (interpreted as u128) because its in a KV store. There is no way to get any other
	/// workflow besides the first.
	#[tracing::instrument(skip_all, fields(%workflow_name))]
	async fn find_workflow(
		&self,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<Option<Uuid>> {
		// Convert to flat vec of strings
		let mut tag_iter = tags
			.as_object()
			.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))?
			.iter()
			.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)));
		let first_tag = tag_iter.next().transpose()?;
		let rest_of_tags = tag_iter.collect::<WorkflowResult<Vec<_>>>()?;

		let start_instant = Instant::now();

		let workflow_id = self
			.pools
			.fdb()?
			.run(|tx, _mc| {
				let first_tag = first_tag.clone();
				let rest_of_tags = rest_of_tags.clone();
				async move {
					let workflow_by_name_and_tag_subspace =
						if let Some((first_tag_key, first_tag_value)) = first_tag {
							self.subspace
								.subspace(&keys::workflow::ByNameAndTagKey::subspace(
									workflow_name.to_string(),
									first_tag_key,
									first_tag_value,
								))
						} else {
							// No tags provided, use null subspace. Every workflow has a null key for its tags
							// under the `ByNameAndTagKey` subspace
							self.subspace
								.subspace(&keys::workflow::ByNameAndTagKey::null_subspace(
									workflow_name.to_string(),
								))
						};

					let mut stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::Iterator,
							..(&workflow_by_name_and_tag_subspace).into()
						},
						SERIALIZABLE,
					);

					loop {
						let Some(entry) = stream.try_next().await? else {
							return Ok(None);
						};

						// Unpack key
						let workflow_by_name_and_tag_key = self
							.subspace
							.unpack::<keys::workflow::ByNameAndTagKey>(&entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						// Deserialize value
						let wf_rest_of_tags = workflow_by_name_and_tag_key
							.deserialize(entry.value())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						// Compute intersection between wf tags and input
						let tags_match = rest_of_tags.iter().all(|(k, v)| {
							wf_rest_of_tags
								.iter()
								.any(|(wf_k, wf_v)| k == wf_k && v == wf_v)
						});

						// Return first signal that matches the tags
						if tags_match {
							break Ok(Some(workflow_by_name_and_tag_key.workflow_id));
						}
					}
				}
			})
			.custom_instrument(tracing::info_span!("find_workflow_tx"))
			.await?;

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::FIND_WORKFLOWS_DURATION
			.with_label_values(&[workflow_name])
			.observe(dt);

		Ok(workflow_id)
	}

	#[tracing::instrument(skip_all)]
	async fn pull_workflows(
		&self,
		worker_instance_id: Uuid,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflowData>> {
		let start_instant = Instant::now();
		let owned_filter = filter
			.into_iter()
			.map(|x| x.to_string())
			.collect::<Vec<_>>();

		let partial_workflows = self
			.pools
			.fdb()?
			.run(|tx, _mc| {
				let owned_filter = owned_filter.clone();

				async move {
					let now = rivet_util::timestamp::now();

					// All wake conditions with a timestamp after this timestamp will be pulled
					let pull_before = now
						+ i64::try_from(self.worker_poll_interval().as_millis())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					// Pull all available wake conditions from all registered wf names
					let entries = futures_util::stream::iter(owned_filter)
						.map(|wf_name| {
							let wake_subspace_start = self
								.subspace
								.subspace(
									&keys::wake::WorkflowWakeConditionKey::subspace_without_ts(
										wf_name.clone(),
									),
								)
								.bytes()
								.iter()
								.map(|x| *x)
								// https://github.com/apple/foundationdb/blob/main/design/tuple.md
								.chain(std::iter::once(0x00))
								.collect::<Vec<_>>();
							let wake_subspace_end = self
								.subspace
								.subspace(&keys::wake::WorkflowWakeConditionKey::subspace(
									wf_name,
									pull_before,
								))
								.bytes()
								.to_vec();

							tx.get_ranges_keyvalues(
								fdb::RangeOption {
									mode: StreamingMode::WantAll,
									..(wake_subspace_start, wake_subspace_end).into()
								},
								// Must be a snapshot to not conflict with any new wake conditions being
								// inserted
								SNAPSHOT,
							)
						})
						.flatten()
						.map(|res| match res {
							Ok(entry) => Ok((
								entry.key().to_vec(),
								self.subspace
									.unpack::<keys::wake::WorkflowWakeConditionKey>(entry.key())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							)),
							Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
						})
						.try_collect::<Vec<_>>()
						.await?;

					// Collect name and deadline ts for each wf id
					let mut dedup_workflows: Vec<(Uuid, String, Option<i64>)> = Vec::new();
					for (_, key) in &entries {
						if let Some((_, _, last_wake_deadline_ts)) = dedup_workflows
							.iter_mut()
							.find(|(wf_id, _, _)| wf_id == &key.workflow_id)
						{
							let wake_deadline_ts = key.condition.deadline_ts();

							// Update wake deadline ts
							if last_wake_deadline_ts.is_none()
								|| wake_deadline_ts < *last_wake_deadline_ts
							{
								*last_wake_deadline_ts = wake_deadline_ts;
							}

							continue;
						}

						dedup_workflows.push((
							key.workflow_id,
							key.workflow_name.clone(),
							key.condition.deadline_ts(),
						));
					}

					// Check leases
					let leased_workflows = futures_util::stream::iter(dedup_workflows)
						.map(|(workflow_id, workflow_name, wake_deadline_ts)| {
							let tx = tx.clone();
							async move {
								let lease_key = keys::workflow::LeaseKey::new(workflow_id);
								let lease_key_buf = self.subspace.pack(&lease_key);

								// Check lease
								if tx.get(&lease_key_buf, SERIALIZABLE).await?.is_some() {
									Result::<_, fdb::FdbBindingError>::Ok(None)
								} else {
									// Write lease
									tx.set(
										&lease_key_buf,
										&lease_key
											.serialize((workflow_name.clone(), worker_instance_id))
											.map_err(|x| {
												fdb::FdbBindingError::CustomError(x.into())
											})?,
									);

									// Write worker instance id
									let worker_instance_id_key =
										keys::workflow::WorkerInstanceIdKey::new(workflow_id);
									tx.set(
										&self.subspace.pack(&worker_instance_id_key),
										&worker_instance_id_key
											.serialize(worker_instance_id)
											.map_err(|x| {
												fdb::FdbBindingError::CustomError(x.into())
											})?,
									);

									Ok(Some((workflow_id, workflow_name, wake_deadline_ts)))
								}
							}
						})
						// TODO: How to get rid of this buffer?
						.buffer_unordered(1024)
						.try_filter_map(|x| std::future::ready(Ok(x)))
						.try_collect::<Vec<_>>()
						.instrument(tracing::trace_span!("map_to_leased_workflows"))
						.await?;

					// TODO: Split this txn into two after checking leases here?

					for (raw_key, key) in &entries {
						// Filter unleased entries
						if !leased_workflows
							.iter()
							.any(|(wf_id, _, _)| wf_id == &key.workflow_id)
						{
							continue;
						}

						// Clear fetched wake conditions
						tx.clear(raw_key);
					}

					// TODO: Parallelize
					// Clear secondary indexes so that we don't get any new wake conditions inserted while
					// the workflow is running
					for (workflow_id, _, _) in &leased_workflows {
						// Clear sub workflow secondary idx
						let wake_sub_workflow_key =
							keys::workflow::WakeSubWorkflowKey::new(*workflow_id);
						if let Some(entry) = tx
							.get(&self.subspace.pack(&wake_sub_workflow_key), SERIALIZABLE)
							.await?
						{
							let sub_workflow_id = wake_sub_workflow_key
								.deserialize(&entry)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

							let sub_workflow_wake_key =
								keys::wake::SubWorkflowWakeKey::new(sub_workflow_id, *workflow_id);

							tx.clear(&self.subspace.pack(&sub_workflow_wake_key));
						}

						// Clear signals secondary index
						let wake_signals_subspace = self
							.subspace
							.subspace(&keys::workflow::WakeSignalKey::subspace(*workflow_id));
						tx.clear_subspace_range(&wake_signals_subspace);
					}

					// Read required data for each leased wf
					futures_util::stream::iter(leased_workflows)
						.map(|(workflow_id, workflow_name, wake_deadline_ts)| {
							let tx = tx.clone();
							async move {
								let create_ts_key = keys::workflow::CreateTsKey::new(workflow_id);
								let ray_id_key = keys::workflow::RayIdKey::new(workflow_id);
								let input_key = keys::workflow::InputKey::new(workflow_id);
								let input_subspace = self.subspace.subspace(&input_key);

								let (create_ts_entry, ray_id_entry, input_chunks) = tokio::try_join!(
									tx.get(&self.subspace.pack(&create_ts_key), SERIALIZABLE),
									tx.get(&self.subspace.pack(&ray_id_key), SERIALIZABLE),
									tx.get_ranges_keyvalues(
										fdb::RangeOption {
											mode: StreamingMode::WantAll,
											..(&input_subspace).into()
										},
										SERIALIZABLE,
									)
									.try_collect::<Vec<_>>(),
								)?;

								let create_ts = create_ts_key
									.deserialize(&create_ts_entry.ok_or(
										fdb::FdbBindingError::CustomError(
											format!("key should exist: {create_ts_key:?}").into(),
										),
									)?)
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
								let ray_id = ray_id_key
									.deserialize(&ray_id_entry.ok_or(
										fdb::FdbBindingError::CustomError(
											format!("key should exist: {ray_id_key:?}").into(),
										),
									)?)
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
								let input = input_key
									.combine(input_chunks)
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

								Ok(PartialWorkflow {
									workflow_id,
									workflow_name,
									create_ts,
									ray_id,
									input,
									wake_deadline_ts,
								})
							}
						})
						// TODO: How to get rid of this buffer?
						.buffer_unordered(512)
						.try_collect::<Vec<_>>()
						.instrument(tracing::trace_span!("map_to_partial_workflow"))
						.await
				}
			})
			.custom_instrument(tracing::info_span!("pull_workflows_tx"))
			.await?;

		let worker_instance_id_str = worker_instance_id.to_string();
		let dt = start_instant.elapsed().as_secs_f64();
		metrics::LAST_PULL_WORKFLOWS_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.set(dt);
		metrics::PULL_WORKFLOWS_DURATION
			.with_label_values(&[&worker_instance_id_str])
			.observe(dt);

		if partial_workflows.is_empty() {
			return Ok(Vec::new());
		}

		let start_instant2 = Instant::now();

		// Set up sqlite
		let pulled_workflows = futures_util::stream::iter(partial_workflows)
			.map(|partial| {
				async move {
					let pool = &self
						.pools
						.sqlite(crate::db::sqlite_db_name_internal(partial.workflow_id), false)
						.await?;

					// Handle error during sqlite init
					if let Err(err) = sqlite::init(partial.workflow_id, pool).await {
						self.commit_workflow(
							partial.workflow_id,
							&partial.workflow_name,
							false,
							None,
							&[],
							None,
							&err.to_string(),
						)
						.await?;

						return Ok(None);
					}

					// Fetch all events
					let events = sql_fetch_all!(
						[self, sqlite::AmalgamEventRow, pool]
						"
						-- Activity events
						SELECT
							json(location) AS location,
							version,
							0 AS event_type, -- EventType
							activity_name AS name,
							NULL AS auxiliary_id,
							input_hash AS hash,
							NULL AS input,
							json(output) AS output,
							create_ts AS create_ts,
							(
								SELECT COUNT(*)
								FROM workflow_activity_errors AS err
								WHERE ev.location = err.location
							) AS error_count,
							NULL AS iteration,
							NULL AS deadline_ts,
							NULL AS state,
							NULL AS inner_event_type
						FROM workflow_activity_events AS ev
						WHERE NOT forgotten
						GROUP BY ev.location
						UNION ALL
						-- Signal listen events
						SELECT
							json(location) AS location,
							version,
							1 AS event_type, -- EventType
							signal_name AS name,
							NULL AS auxiliary_id,
							NULL AS hash,
							NULL AS input,
							json(body) AS output,
							NULL AS create_ts,
							NULL AS error_count,
							NULL AS iteration,
							NULL AS deadline_ts,
							NULL AS state,
							NULL AS inner_event_type
						FROM workflow_signal_events
						WHERE NOT forgotten
						UNION ALL
						-- Signal send events
						SELECT
							json(location) AS location,
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
						FROM workflow_signal_send_events
						WHERE NOT forgotten
						UNION ALL
						-- Message send events
						SELECT
							json(location) AS location,
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
						FROM workflow_message_send_events
						WHERE NOT forgotten
						UNION ALL
						-- Sub workflow events
						SELECT
							json(location) AS location,
							version,
							4 AS event_type, -- crdb_nats::types::EventType
							sub_workflow_name AS name,
							sub_workflow_id AS auxiliary_id,
							NULL AS hash,
							NULL AS input,
							NULL AS output,
							NULL AS create_ts,
							NULL AS error_count,
							NULL AS iteration,
							NULL AS deadline_ts,
							NULL AS state,
							NULL AS inner_event_type
						FROM workflow_sub_workflow_events
						WHERE NOT forgotten
						UNION ALL
						-- Loop events
						SELECT
							json(location) AS location,
							version,
							5 AS event_type, -- crdb_nats::types::EventType
							NULL AS name,
							NULL AS auxiliary_id,
							NULL AS hash,
							json(state) AS input,
							json(output) AS output,
							NULL AS create_ts,
							NULL AS error_count,
							iteration,
							NULL AS deadline_ts,
							NULL AS state,
							NULL AS inner_event_type
						FROM workflow_loop_events
						WHERE NOT forgotten
						UNION ALL
						-- Sleep events
						SELECT
							json(location) AS location,
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
						FROM workflow_sleep_events
						WHERE NOT forgotten
						UNION ALL
						-- Branch events
						SELECT
							json(location) AS location,
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
						FROM workflow_branch_events
						WHERE NOT forgotten
						UNION ALL
						-- Removed events
						SELECT
							json(location) AS location,
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
						FROM workflow_removed_events
						WHERE NOT forgotten
						UNION ALL
						-- Version check events
						SELECT
							json(location) AS location,
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
						FROM workflow_version_check_events
						WHERE NOT forgotten
						",
					)
					.await?;

					WorkflowResult::Ok(Some(PulledWorkflowData {
						workflow_id: partial.workflow_id,
						workflow_name: partial.workflow_name,
						create_ts: partial.create_ts,
						ray_id: partial.ray_id,
						input: partial.input,
						wake_deadline_ts: partial.wake_deadline_ts,
						events: sqlite::build_history(events)?,
					}))
				}
			})
			.buffer_unordered(512)
			.try_filter_map(|x| std::future::ready(Ok(x)))
			.try_collect()
			.instrument(tracing::trace_span!("map_to_pulled_workflows"))
			.await?;

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

		Ok(pulled_workflows)
	}

	#[tracing::instrument(skip_all)]
	async fn complete_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		// Evict databases before releasing lease
		self.evict_wf_sqlite(workflow_id).await?;

		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let sub_workflow_wake_subspace = self
						.subspace
						.subspace(&keys::wake::SubWorkflowWakeKey::subspace(workflow_id));
					let tags_subspace = self
						.subspace
						.subspace(&keys::workflow::TagKey::subspace(workflow_id));
					let wake_deadline_key = keys::workflow::WakeDeadlineKey::new(workflow_id);

					let mut stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&sub_workflow_wake_subspace).into()
						},
						// NOTE: Must be serializable to conflict with `get_sub_workflow`
						SERIALIZABLE,
					);

					let (_, tag_keys, wake_deadline_entry) = tokio::try_join!(
						// Check for other workflows waiting on this one, wake all
						async {
							while let Some(entry) = stream.try_next().await? {
								let sub_workflow_wake_key = self
									.subspace
									.unpack::<keys::wake::SubWorkflowWakeKey>(&entry.key())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
								let workflow_name = sub_workflow_wake_key
									.deserialize(entry.value())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

								let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
									workflow_name,
									sub_workflow_wake_key.workflow_id,
									keys::wake::WakeCondition::SubWorkflow {
										sub_workflow_id: workflow_id,
									},
								);

								// Add wake condition for workflow
								tx.set(
									&self.subspace.pack(&wake_condition_key),
									&wake_condition_key
										.serialize(())
										.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
								);

								// Clear secondary index
								tx.clear(entry.key());
							}

							Result::<_, fdb::FdbBindingError>::Ok(())
						},
						// Read tags
						async {
							tx.get_ranges_keyvalues(
								fdb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&tags_subspace).into()
								},
								SERIALIZABLE,
							)
							.map(|res| match res {
								Ok(entry) => self
									.subspace
									.unpack::<keys::workflow::TagKey>(entry.key())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into())),
								Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
							})
							.try_collect::<Vec<_>>()
							.await
							.map_err(Into::into)
						},
						async {
							tx.get(&self.subspace.pack(&wake_deadline_key), SERIALIZABLE)
								.await
								.map_err(Into::into)
						},
					)?;

					for key in tag_keys {
						let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::new(
							workflow_name.to_string(),
							key.k,
							key.v,
							workflow_id,
						);
						tx.clear(&self.subspace.pack(&by_name_and_tag_key));
					}

					// Clear null key
					{
						let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::null(
							workflow_name.to_string(),
							workflow_id,
						);
						tx.clear(&self.subspace.pack(&by_name_and_tag_key));
					}

					// Get and clear the pending deadline wake condition, if any. This could be put in the
					// `pull_workflows` function (where we clear secondary indexes) but we chose to clear it
					// here and in `commit_workflow` because its not a secondary index so theres no worry of
					// it inserting more wake conditions. This reduces the load on `pull_workflows`. The
					// reason this isn't immediately cleared in `pull_workflows` along with the rest of the
					// wake conditions is because it might be in the future.
					if let Some(raw) = wake_deadline_entry {
						let deadline_ts = wake_deadline_key
							.deserialize(&raw)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name.to_string(),
							workflow_id,
							keys::wake::WakeCondition::Deadline { deadline_ts },
						);

						tx.clear(&self.subspace.pack(&wake_condition_key));
					}

					// Clear "has wake condition"
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&has_wake_condition_key));

					// Write output
					let output_key = keys::workflow::OutputKey::new(workflow_id);

					for (i, chunk) in output_key
						.split_ref(output)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.enumerate()
					{
						let chunk_key = output_key.chunk(i);

						tx.set(&self.subspace.pack(&chunk_key), &chunk);
					}

					// Clear lease
					let lease_key = keys::workflow::LeaseKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&lease_key));
					let worker_instance_id_key =
						keys::workflow::WorkerInstanceIdKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&worker_instance_id_key));

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("complete_workflows_tx"))
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
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow_id: Option<Uuid>,
		error: &str,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		// Evict databases before releasing lease
		self.evict_wf_sqlite(workflow_id).await?;

		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let wake_deadline_key = keys::workflow::WakeDeadlineKey::new(workflow_id);

					let wake_deadline_entry = tx
						.get(&self.subspace.pack(&wake_deadline_key), SERIALIZABLE)
						.await?;

					// Add immediate wake for workflow
					if wake_immediate {
						let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name.to_string(),
							workflow_id,
							keys::wake::WakeCondition::Immediate,
						);
						tx.set(
							&self.subspace.pack(&wake_condition_key),
							&wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					// Get and clear the pending deadline wake condition, if any. This could be put in the
					// `pull_workflows` function (where we clear secondary indexes) but we chose to clear it
					// here and in `complete_workflow` because its not a secondary index so theres no worry of
					// it inserting more wake conditions. This reduces the load on `pull_workflows`. The
					// reason this isn't immediately cleared in `pull_workflows` along with the rest of the
					// wake conditions is because it might be in the future.
					if let Some(raw) = wake_deadline_entry {
						let deadline_ts = wake_deadline_key
							.deserialize(&raw)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name.to_string(),
							workflow_id,
							keys::wake::WakeCondition::Deadline { deadline_ts },
						);

						tx.clear(&self.subspace.pack(&wake_condition_key));
					}

					// Write deadline wake index
					if let Some(deadline_ts) = wake_deadline_ts {
						let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name.to_string(),
							workflow_id,
							keys::wake::WakeCondition::Deadline { deadline_ts },
						);

						// Add wake condition for workflow
						tx.set(
							&self.subspace.pack(&wake_condition_key),
							&wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);

						// Write to wake deadline
						tx.set(
							&self.subspace.pack(&wake_deadline_key),
							&wake_deadline_key
								.serialize(deadline_ts)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					self.write_signal_wake_idxs(workflow_id, wake_signals, &tx)?;

					// Write sub workflow wake index
					if let Some(sub_workflow_id) = wake_sub_workflow_id {
						self.write_sub_workflow_wake_idx(
							workflow_id,
							workflow_name,
							sub_workflow_id,
							&tx,
						)?;
					}

					// Update "has wake condition"
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(workflow_id);
					if wake_immediate
						|| wake_deadline_ts.is_some()
						|| !wake_signals.is_empty()
						|| wake_sub_workflow_id.is_some()
					{
						tx.set(
							&self.subspace.pack(&has_wake_condition_key),
							&has_wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					} else {
						tx.clear(&self.subspace.pack(&has_wake_condition_key));
					}

					// Write error
					let error_key = keys::workflow::ErrorKey::new(workflow_id);
					tx.set(
						&self.subspace.pack(&error_key),
						&error_key
							.serialize(error.to_string())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Clear lease
					let lease_key = keys::workflow::LeaseKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&lease_key));
					let worker_instance_id_key =
						keys::workflow::WorkerInstanceIdKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&worker_instance_id_key));

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("commit_workflow_tx"))
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
		last_try: bool,
	) -> WorkflowResult<Option<SignalData>> {
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(workflow_id), false)
			.await?;

		let owned_filter = filter
			.into_iter()
			.map(|x| x.to_string())
			.collect::<Vec<_>>();
		let is_retrying = Arc::new(AtomicBool::new(false));

		// Fetch signal from FDB
		let signal =
			self.pools
				.fdb()?
				.run(|tx, _mc| {
					let owned_filter = owned_filter.clone();
					let is_retrying = is_retrying.clone();

					async move {
						let signal = {
							// Create a stream for each signal name subspace
							let streams = owned_filter
								.iter()
								.map(|signal_name| {
									let pending_signal_subspace = self.subspace.subspace(
										&keys::workflow::PendingSignalKey::subspace(
											workflow_id,
											signal_name.to_string(),
										),
									);

									tx.get_ranges_keyvalues(
										fdb::RangeOption {
											mode: StreamingMode::WantAll,
											limit: Some(1),
											..(&pending_signal_subspace).into()
										},
										// NOTE: This does not have to be SERIALIZABLE because the conflict occurs
										// with acking which is a separate row. See below
										SNAPSHOT,
									)
								})
								.collect::<Vec<_>>();

							// Fetch the next entry from all streams at the same time
							let mut results = futures_util::future::try_join_all(
								streams.into_iter().map(|mut stream| async move {
									if let Some(entry) = stream.try_next().await? {
										Result::<_, fdb::FdbBindingError>::Ok(Some((
											entry.key().to_vec(),
											self.subspace
												.unpack::<keys::workflow::PendingSignalKey>(
													&entry.key(),
												)
												.map_err(|x| {
													fdb::FdbBindingError::CustomError(x.into())
												})?,
										)))
									} else {
										Ok(None)
									}
								}),
							)
							.instrument(tracing::trace_span!("map_signals"))
							.await?;

							// Sort by ts
							results.sort_by_key(|res| res.as_ref().map(|(_, key)| key.ts));

							results.into_iter().flatten().next().map(
								|(raw_key, pending_signal_key)| {
									(
										raw_key,
										pending_signal_key.signal_name,
										pending_signal_key.ts,
										pending_signal_key.signal_id,
									)
								},
							)
						};

						// Signal found
						if let Some((raw_key, signal_name, ts, signal_id)) = signal {
							let ack_ts_key = keys::signal::AckTsKey::new(signal_id);

							// Ack signal
							tx.add_conflict_range(
								&raw_key,
								&end_of_key_range(&raw_key),
								ConflictRangeType::Read,
							)?;
							tx.set(
								&self.subspace.pack(&ack_ts_key),
								&ack_ts_key
									.serialize(rivet_util::timestamp::now())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							);

							// TODO: Split txn into two after acking here?

							// Clear pending signal key
							tx.clear(&raw_key);

							// Read signal body
							let body_key = keys::signal::BodyKey::new(signal_id);
							let body_subspace = self.subspace.subspace(&body_key);

							let chunks = tx
								.get_ranges_keyvalues(
									fdb::RangeOption {
										mode: StreamingMode::WantAll,
										..(&body_subspace).into()
									},
									SERIALIZABLE,
								)
								.try_collect::<Vec<_>>()
								.await?;

							let body = body_key
								.combine(chunks)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

							// In the event of an FDB txn retry, we have to delete the previously inserted row
							if is_retrying.load(Ordering::Relaxed) {
								sql_execute!(
									[self, &pool]
									"
									DELETE FROM workflow_signal_events
									WHERE location = jsonb(?1)
									",
									location,
								)
								.await
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
							}

							// Insert history event
							sql_execute!(
								[self, &pool]
								"
								INSERT INTO workflow_signal_events (
									location,
									version,
									signal_id,
									signal_name,
									body,
									create_ts,
									loop_location
								)
								VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5), ?6, jsonb(?7))
								",
								location,
								version as i64,
								signal_id,
								&signal_name,
								sqlx::types::Json(&body),
								rivet_util::timestamp::now(),
								loop_location,
							)
							.await
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

							is_retrying.store(true, Ordering::Relaxed);

							Ok(Some(SignalData {
								signal_id,
								signal_name,
								create_ts: ts,
								body,
							}))
						}
						// No signal found
						else {
							// Write signal wake index if no signal was received. Normally this is done in
							// `commit_workflow` but without this code there would be a race condition if the
							// signal is published between after this transaction and before `commit_workflow`.
							// There is a possibility of `commit_workflow` NOT writing a signal secondary index
							// after this in which case there might be an unnecessary wake condition inserted
							// causing the workflow to wake up again, but this is not as big of an issue because
							// workflow wakes should be idempotent if no events happen.
							// It is important that this is only written on the last try to pull workflows
							// (the workflow engine internally retries a few times) because it should only
							// write signal wake indexes before going to sleep (with err `NoSignalFound`) and
							// not during a retry.
							if last_try {
								self.write_signal_wake_idxs(
									workflow_id,
									&owned_filter.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
									&tx,
								)?;
							}

							Ok(None)
						}
					}
				})
				.custom_instrument(tracing::info_span!("pull_next_signal_tx"))
				.await?;

		if signal.is_some() {
			self.flush_wf_sqlite(workflow_id)?;
		}

		Ok(signal)
	}

	#[tracing::instrument(skip_all)]
	async fn get_sub_workflow(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		sub_workflow_id: Uuid,
	) -> WorkflowResult<Option<WorkflowData>> {
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let input_key = keys::workflow::InputKey::new(sub_workflow_id);
					let input_subspace = self.subspace.subspace(&input_key);
					let output_key = keys::workflow::OutputKey::new(sub_workflow_id);
					let output_subspace = self.subspace.subspace(&output_key);
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(sub_workflow_id);

					// Read input and output
					let (input_chunks, output_chunks, has_wake_condition_entry) = tokio::try_join!(
						tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&input_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&output_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get(&self.subspace.pack(&has_wake_condition_key), SERIALIZABLE),
					)?;

					if input_chunks.is_empty() {
						Ok(None)
					} else {
						let input = input_key
							.combine(input_chunks)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let output = if output_chunks.is_empty() {
							// Write sub workflow wake index if the sub workflow is not complete yet. Normally
							// this is done in `commit_workflow` but without this code there would be a race
							// condition if the sub workflow completes between after this transaction and
							// before `commit_workflow`. There is a possibility of `commit_workflow` NOT writing a
							// sub workflow secondary index after this in which case there might be an
							// unnecessary wake condition inserted causing the workflow to wake up again, but this
							// is not as big of an issue because workflow wakes should be idempotent if no events
							// happen.
							self.write_sub_workflow_wake_idx(
								workflow_id,
								workflow_name,
								sub_workflow_id,
								&tx,
							)?;

							None
						} else {
							Some(
								output_key
									.combine(output_chunks)
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							)
						};

						Ok(Some(WorkflowData {
							workflow_id: sub_workflow_id,
							input,
							output,
							has_wake_condition: has_wake_condition_entry.is_some(),
						}))
					}
				}
			})
			.custom_instrument(tracing::info_span!("get_sub_workflow_tx"))
			.await
			.map_err(Into::into)
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
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let workflow_name_key = keys::workflow::NameKey::new(workflow_id);

					// NOTE: This does not have to be serializable because wf name doesn't change
					// Check if the workflow exists
					let Some(workflow_name_entry) = tx
						.get(&self.subspace.pack(&workflow_name_key), SNAPSHOT)
						.await?
					else {
						return Err(fdb::FdbBindingError::CustomError(
							WorkflowError::WorkflowNotFound.into(),
						));
					};

					let workflow_name = workflow_name_key
						.deserialize(&workflow_name_entry)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					// Write name
					let name_key = keys::signal::NameKey::new(signal_id);
					tx.set(
						&self.subspace.pack(&name_key),
						&name_key
							.serialize(signal_name.to_string())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					let signal_body_key = keys::signal::BodyKey::new(signal_id);

					// Write signal body
					for (i, chunk) in signal_body_key
						.split_ref(body)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.enumerate()
					{
						let chunk_key = signal_body_key.chunk(i);

						tx.set(&self.subspace.pack(&chunk_key), &chunk);
					}

					// Write pending key
					let pending_signal_key = keys::workflow::PendingSignalKey::new(
						workflow_id,
						signal_name.to_string(),
						signal_id,
					);

					tx.set(
						&self.subspace.pack(&pending_signal_key),
						&pending_signal_key
							.serialize(())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write create ts
					let create_ts_key = keys::signal::CreateTsKey::new(signal_id);
					tx.set(
						&self.subspace.pack(&create_ts_key),
						&create_ts_key
							.serialize(pending_signal_key.ts)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write ray id ts
					let ray_id_key = keys::signal::RayIdKey::new(signal_id);
					tx.set(
						&self.subspace.pack(&ray_id_key),
						&ray_id_key
							.serialize(ray_id)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Write workflow id
					let workflow_id_key = keys::signal::WorkflowIdKey::new(signal_id);
					tx.set(
						&self.subspace.pack(&workflow_id_key),
						&workflow_id_key
							.serialize(workflow_id)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					let wake_signal_key =
						keys::workflow::WakeSignalKey::new(workflow_id, signal_name.to_string());

					// If the workflow is currently listening for this signal, wake it
					if tx
						.get(&self.subspace.pack(&wake_signal_key), SERIALIZABLE)
						.await?
						.is_some()
					{
						let mut wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name,
							workflow_id,
							keys::wake::WakeCondition::Signal { signal_id },
						);
						wake_condition_key.ts = pending_signal_key.ts;

						// Add wake condition for workflow
						tx.set(
							&self.subspace.pack(&wake_condition_key),
							&wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("publish_signal_tx"))
			.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_tagged_signal(
		&self,
		_ray_id: Uuid,
		_tags: &serde_json::Value,
		_signal_id: Uuid,
		_signal_name: &str,
		_body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		Err(WorkflowError::TaggedSignalsDisabled)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		// Insert history event
		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_signal_send_events (
				location, version, signal_id, signal_name, body, workflow_id, create_ts, loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5), ?6, ?7, jsonb(?8))
			",
			location,
			version as i64,
			signal_id,
			signal_name,
			sqlx::types::Json(body),
			to_workflow_id,
			rivet_util::timestamp::now(),
			loop_location,
		)
		.await?;

		// Block while flushing databases in order ensure listeners have the latest data
		self.pools
			.sqlite_manager()
			.flush(
				vec![
					crate::db::sqlite_db_name_internal(from_workflow_id),
					crate::db::sqlite_db_name_data(from_workflow_id),
				],
				false,
			)
			.await?;

		if let Err(err) = self
			.publish_signal(ray_id, to_workflow_id, signal_id, signal_name, body)
			.await
		{
			// Undo history if FDB failed
			sql_execute!(
				[self, pool]
				"
				DELETE FROM workflow_signal_send_events
				WHERE location = jsonb(?1)
				",
				location,
			)
			.await?;

			self.flush_wf_sqlite(from_workflow_id)?;

			Err(err)
		} else {
			Ok(())
		}
	}

	#[tracing::instrument(skip_all)]
	async fn publish_tagged_signal_from_workflow(
		&self,
		_from_workflow_id: Uuid,
		_location: &Location,
		_version: usize,
		_ray_id: Uuid,
		_tags: &serde_json::Value,
		_signal_id: Uuid,
		_signal_name: &str,
		_body: &serde_json::value::RawValue,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		Err(WorkflowError::TaggedSignalsDisabled)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(workflow_id), false)
			.await?;

		// Insert history event
		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_sub_workflow_events (
				location,
				version,
				sub_workflow_id,
				sub_workflow_name,
				tags,
				input,
				create_ts,
				loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5), jsonb(?6), ?7, jsonb(?8))				
			",
			location,
			version as i64,
			sub_workflow_id,
			sub_workflow_name,
			tags,
			sqlx::types::Json(input),
			rivet_util::timestamp::now(),
			loop_location,
		)
		.await?;

		// Block while flushing databases in order ensure sub workflow have the latest data
		self.pools
			.sqlite_manager()
			.flush(
				vec![
					crate::db::sqlite_db_name_internal(workflow_id),
					crate::db::sqlite_db_name_data(workflow_id),
				],
				false,
			)
			.await?;

		match self
			.dispatch_workflow(
				ray_id,
				sub_workflow_id,
				sub_workflow_name,
				tags,
				input,
				unique,
			)
			.await
		{
			Ok(workflow_id) => Ok(workflow_id),
			Err(err) => {
				// Undo history if FDB failed
				sql_execute!(
					[self, pool]
					"
					DELETE FROM workflow_sub_workflow_events
					WHERE location = jsonb(?1)
					",
					location,
				)
				.await?;

				self.flush_wf_sqlite(workflow_id)?;

				Err(err)
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn update_workflow_tags(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<()> {
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				async move {
					let tags_subspace = self
						.subspace
						.subspace(&keys::workflow::TagKey::subspace(workflow_id));

					// Read old tags
					let tag_keys = tx
						.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&tags_subspace).into()
							},
							SERIALIZABLE,
						)
						.map(|res| match res {
							Ok(entry) => self
								.subspace
								.unpack::<keys::workflow::TagKey>(entry.key())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into())),
							Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
						})
						.try_collect::<Vec<_>>()
						.await?;

					// Clear old tags
					tx.clear_subspace_range(&tags_subspace);

					// Clear old "by name and first tag" secondary index
					for key in tag_keys {
						keys::workflow::ByNameAndTagKey::new(
							workflow_name.to_string(),
							key.k,
							key.v,
							workflow_id,
						);
					}

					// Write new tags
					let tags = tags
						.as_object()
						.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
						.collect::<WorkflowResult<Vec<_>>>()
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					for (k, v) in &tags {
						let tag_key =
							keys::workflow::TagKey::new(workflow_id, k.clone(), v.clone());
						tx.set(
							&self.subspace.pack(&tag_key),
							&tag_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);

						// Write new "by name and first tag" secondary index
						let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::new(
							workflow_name.to_string(),
							k.clone(),
							v.clone(),
							workflow_id,
						);
						let rest_of_tags = tags
							.iter()
							.filter(|(k2, _)| k2 != k)
							.map(|(k, v)| (k.clone(), v.clone()))
							.collect();
						tx.set(
							&self.subspace.pack(&by_name_and_tag_key),
							&by_name_and_tag_key
								.serialize(rest_of_tags)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("update_workflow_tags_tx"))
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(workflow_id), false)
			.await?;
		let input_hash = event_id.input_hash.to_be_bytes();

		match res {
			Ok(output) => {
				sql_execute!(
					[self, pool]
					"
					INSERT INTO workflow_activity_events (
						location,
						version,
						activity_name,
						input_hash,
						input,
						output,
						create_ts,
						loop_location
					)
					VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5), jsonb(?6), ?7, jsonb(?8))
					ON CONFLICT (location) DO UPDATE
					SET output = EXCLUDED.output
					",
					location,
					version as i64,
					&event_id.name,
					input_hash.as_slice(),
					sqlx::types::Json(input),
					sqlx::types::Json(output),
					create_ts,
					loop_location,
				)
				.await?;
			}
			Err(err) => {
				self.txn(|| async {
					let mut conn = pool.conn().await?;
					let mut tx = conn.begin().await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						INSERT INTO workflow_activity_events (
							location,
							version,
							activity_name,
							input_hash,
							input,
							create_ts,
							loop_location
						)
						VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5), ?6, jsonb(?7))
						ON CONFLICT (location) DO NOTHING
						",
						location,
						version as i64,
						&event_id.name,
						input_hash.as_slice(),
						sqlx::types::Json(input),
						create_ts,
						loop_location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						INSERT INTO workflow_activity_errors (
							location, activity_name, error, ts
						)
						VALUES (jsonb(?1), ?2, ?3, ?4)
						",
						location,
						&event_id.name,
						err,
						rivet_util::timestamp::now(),
					)
					.await?;

					tx.commit().await.map_err(Into::into)
				})
				.await?;
			}
		}

		self.flush_wf_sqlite(workflow_id)?;

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
		// Block while flushing databases in order ensure subscribers will have the latest data
		self.pools
			.sqlite_manager()
			.flush(
				vec![
					crate::db::sqlite_db_name_internal(from_workflow_id),
					crate::db::sqlite_db_name_data(from_workflow_id),
				],
				false,
			)
			.await?;

		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_message_send_events (
				location, version, tags, message_name, body, create_ts, loop_location
			)
			VALUES (jsonb(?1), ?2, jsonb(?3), ?4, jsonb(?5), ?6, jsonb(?7))
			",
			location,
			version as i64,
			tags,
			message_name,
			sqlx::types::Json(body),
			rivet_util::timestamp::now(),
			loop_location,
		)
		.await?;

		self.flush_wf_sqlite(from_workflow_id)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn upsert_workflow_loop_event(
		&self,
		workflow_id: Uuid,
		workflow_name: &str,
		location: &Location,
		version: usize,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
		loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(workflow_id), false)
			.await?;

		self.txn(|| async {
			let mut conn = pool.conn().await?;
			let mut tx = conn.begin().await?;

			sql_execute!(
				[self, @tx &mut tx]
				"
				INSERT INTO workflow_loop_events (
					location,
					version,
					iteration,
					state,
					output,
					create_ts,
					loop_location
				)
				VALUES (jsonb(?1), ?2, ?3, jsonb(?4), jsonb(?5), ?6, jsonb(?7))
				ON CONFLICT (location) DO UPDATE
				SET
					iteration = ?3,
					state = jsonb(?4),
					output = jsonb(?5)
				",
				location,
				version as i64,
				iteration as i64,
				sqlx::types::Json(state),
				output.map(sqlx::types::Json),
				rivet_util::timestamp::now(),
				loop_location,
			)
			.await?;

			// 0-th iteration is the initial insertion
			if iteration != 0 {
				// TODO: Add config parameter in either fdb or sqlite to toggle this per wf
				let delete_instead_of_forget =
					workflow_name == "pegboard_client" || workflow_name == "pegboard_actor";

				if delete_instead_of_forget {
					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_activity_errors
						WHERE location IN (
							SELECT location
							FROM workflow_activity_events
							WHERE loop_location = jsonb(?1)
						)
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_activity_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_signal_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_sub_workflow_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_signal_send_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_message_send_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_loop_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_sleep_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_branch_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_removed_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						DELETE FROM workflow_version_check_events
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;
				} else {
					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_activity_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_signal_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_sub_workflow_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_signal_send_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_message_send_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_loop_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_sleep_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_branch_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_removed_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;

					sql_execute!(
						[self, @tx &mut tx]
						"
						UPDATE workflow_version_check_events
						SET forgotten = TRUE
						WHERE loop_location = jsonb(?1) AND NOT forgotten
						",
						location,
					)
					.await?;
				}
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_sleep_events (
				location, version, deadline_ts, create_ts, state, loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, ?4, ?5, jsonb(?6))
			",
			location,
			version as i64,
			deadline_ts,
			rivet_util::timestamp::now(),
			SleepState::Normal as i64,
			loop_location,
		)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			UPDATE workflow_sleep_events
			SET state = ?1
			WHERE location = jsonb(?2)
			",
			state as i64,
			location,
		)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_branch_events (
				location, version, create_ts, loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, jsonb(?4))
			",
			location,
			version as i64,
			rivet_util::timestamp::now(),
			loop_location,
		)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_removed_events (
				location, event_type, event_name, create_ts, loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, ?4, jsonb(?5))
			",
			location,
			event_type as i64,
			event_name,
			rivet_util::timestamp::now(),
			loop_location,
		)
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
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(from_workflow_id), false)
			.await?;

		sql_execute!(
			[self, pool]
			"
			INSERT INTO workflow_version_check_events (
				location, version, create_ts, loop_location
			)
			VALUES (jsonb(?1), ?2, ?3, jsonb(?4))
			",
			location,
			version as i64,
			rivet_util::timestamp::now(),
			loop_location,
		)
		.await?;

		Ok(())
	}
}

struct PartialWorkflow {
	pub workflow_id: Uuid,
	pub workflow_name: String,
	pub create_ts: i64,
	pub ray_id: Uuid,
	pub input: Box<serde_json::value::RawValue>,
	pub wake_deadline_ts: Option<i64>,
}

#[derive(Debug, Default)]
struct WorkflowMetrics {
	complete: i64,
	running: i64,
	sleeping: i64,
	dead: i64,
}

#[derive(Debug)]
enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
	Silenced,
}

async fn flush_handler(pools: rivet_pools::Pools, mut flush_rx: mpsc::UnboundedReceiver<Uuid>) {
	while let Some(workflow_id) = flush_rx.recv().await {
		tracing::debug!(?workflow_id, "flushing workflow");

		if let Err(err) = pools
			.sqlite_manager()
			.flush(
				vec![
					crate::db::sqlite_db_name_internal(workflow_id),
					crate::db::sqlite_db_name_data(workflow_id),
				],
				true,
			)
			.await
		{
			// TODO: Somehow forward the error to the workflow so it can die
			tracing::error!(?workflow_id, ?err, "failed to flush workflow databases");
		}
	}

	// If the channel is closed that means the db driver instance was dropped which is not an error
}

fn value_to_str(v: &serde_json::Value) -> WorkflowResult<String> {
	match v {
		serde_json::Value::String(s) => Ok(s.clone()),
		_ => cjson::to_string(&v).map_err(WorkflowError::CjsonSerializeTags),
	}
}
