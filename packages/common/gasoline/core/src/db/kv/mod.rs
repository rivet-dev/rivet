//! Implementation of a workflow database driver with UniversalDB and NATS.
// TODO: Move code to smaller functions for readability

use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Instant,
};

use futures_util::{StreamExt, TryStreamExt, stream::BoxStream};
use rivet_util::Id;
use rivet_util::future::CustomInstrumentExt;
use serde_json::json;
use tracing::Instrument;
use udb_util::{
	FormalChunkedKey, FormalKey, SERIALIZABLE, SNAPSHOT, TxnExt, end_of_key_range, keys::*,
};
use universaldb::{
	self as udb,
	future::FdbValue,
	options::{ConflictRangeType, MutationType, StreamingMode},
};

use rivet_metrics::KeyValue;

use super::{Database, PulledWorkflowData, SignalData, WorkflowData};
use crate::{
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{
			ActivityEvent, Event, EventData, EventId, EventType, LoopEvent, MessageSendEvent,
			RemovedEvent, SignalEvent, SignalSendEvent, SleepEvent, SleepState, SubWorkflowEvent,
		},
		location::Location,
	},
	metrics,
};

mod debug;
mod keys;

/// How long before considering the leases of a given worker instance expired.
const WORKER_INSTANCE_LOST_THRESHOLD_MS: i64 = rivet_util::duration::seconds(30);
/// How long before overwriting an existing metrics lock.
const METRICS_LOCK_TIMEOUT_MS: i64 = rivet_util::duration::seconds(30);
/// For NATS wake mechanism.
const WORKER_WAKE_SUBJECT: &str = "gasoline.worker.wake";

pub struct DatabaseKv {
	pools: rivet_pools::Pools,
	subspace: udb_util::Subspace,
}

impl DatabaseKv {
	/// Spawns a new thread and publishes a worker wake message to nats.
	fn wake_worker(&self) {
		let Ok(nats) = self.pools.ups() else {
			tracing::debug!("failed to acquire nats pool");
			return;
		};

		let spawn_res = tokio::task::Builder::new().name("wake").spawn(
			async move {
				// Fail gracefully
				if let Err(err) = nats.publish(WORKER_WAKE_SUBJECT, &Vec::new()).await {
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

// MARK: FDB Helpers
impl DatabaseKv {
	fn write_signal_wake_idxs(
		&self,
		workflow_id: Id,
		wake_signals: &[&str],
		tx: &udb::RetryableTransaction,
	) -> Result<(), udb::FdbBindingError> {
		for signal_name in wake_signals {
			// Write to wake signals list
			let wake_signal_key =
				keys::workflow::WakeSignalKey::new(workflow_id, signal_name.to_string());
			tx.set(
				&self.subspace.pack(&wake_signal_key),
				&wake_signal_key
					.serialize(())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
			);
		}

		Ok(())
	}

	fn write_sub_workflow_wake_idx(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		sub_workflow_id: Id,
		tx: &udb::RetryableTransaction,
	) -> Result<(), udb::FdbBindingError> {
		let sub_workflow_wake_key =
			keys::wake::SubWorkflowWakeKey::new(sub_workflow_id, workflow_id);

		tx.set(
			&self.subspace.pack(&sub_workflow_wake_key),
			&sub_workflow_wake_key
				.serialize(workflow_name.to_string())
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		Ok(())
	}

	async fn publish_signal_inner(
		&self,
		ray_id: Id,
		workflow_id: Id,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		tx: &udb::RetryableTransaction,
	) -> Result<(), udb::FdbBindingError> {
		tracing::debug!(
			?ray_id,
			?workflow_id,
			?signal_id,
			?signal_name,
			"publishing signal"
		);

		let workflow_name_key = keys::workflow::NameKey::new(workflow_id);

		// Check if the workflow exists
		let Some(workflow_name_entry) = tx
			.get(&self.subspace.pack(&workflow_name_key), SERIALIZABLE)
			.await?
		else {
			return Err(udb::FdbBindingError::CustomError(
				WorkflowError::WorkflowNotFound.into(),
			));
		};

		let workflow_name = workflow_name_key
			.deserialize(&workflow_name_entry)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

		// Write name
		let name_key = keys::signal::NameKey::new(signal_id);
		tx.set(
			&self.subspace.pack(&name_key),
			&name_key
				.serialize(signal_name.to_string())
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		let signal_body_key = keys::signal::BodyKey::new(signal_id);

		// Write signal body
		for (i, chunk) in signal_body_key
			.split_ref(body)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = signal_body_key.chunk(i);

			tx.set(&self.subspace.pack(&chunk_key), &chunk);
		}

		// Write pending key
		let pending_signal_key =
			keys::workflow::PendingSignalKey::new(workflow_id, signal_name.to_string(), signal_id);

		tx.set(
			&self.subspace.pack(&pending_signal_key),
			&pending_signal_key
				.serialize(())
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		// Write create ts
		let create_ts_key = keys::signal::CreateTsKey::new(signal_id);
		tx.set(
			&self.subspace.pack(&create_ts_key),
			&create_ts_key
				.serialize(pending_signal_key.ts)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		// Write ray id
		let ray_id_key = keys::signal::RayIdKey::new(signal_id);
		tx.set(
			&self.subspace.pack(&ray_id_key),
			&ray_id_key
				.serialize(ray_id)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		// Write workflow id
		let workflow_id_key = keys::signal::WorkflowIdKey::new(signal_id);
		tx.set(
			&self.subspace.pack(&workflow_id_key),
			&workflow_id_key
				.serialize(workflow_id)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
		);

		let wake_signal_key =
			keys::workflow::WakeSignalKey::new(workflow_id, signal_name.to_string());

		// If the workflow currently has a wake signal key for this signal, wake it
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
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
			);
		}

		update_metric(
			&tx.subspace(self.subspace.clone()),
			None,
			Some(keys::metric::GaugeMetric::SignalPending(
				signal_name.to_string(),
			)),
		);

		Ok(())
	}

	async fn dispatch_workflow_inner(
		&self,
		ray_id: Id,
		workflow_id: Id,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		unique: bool,
		tx: &udb::RetryableTransaction,
	) -> Result<Id, udb::FdbBindingError> {
		let txs = tx.subspace(self.subspace.clone());

		if unique {
			let empty_tags = json!({});

			if let Some(existing_workflow_id) = self
				.find_workflow_inner(workflow_name, tags.unwrap_or(&empty_tags), tx)
				.await?
			{
				tracing::debug!(?existing_workflow_id, "found existing workflow");
				return Ok(existing_workflow_id);
			}
		}

		txs.write(
			&keys::workflow::CreateTsKey::new(workflow_id),
			rivet_util::timestamp::now(),
		)?;

		txs.write(
			&keys::workflow::NameKey::new(workflow_id),
			workflow_name.to_string(),
		)?;

		txs.write(&keys::workflow::RayIdKey::new(workflow_id), ray_id)?;

		// Write tags
		let tags = tags
			.map(|x| {
				x.as_object()
					.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))
			})
			.transpose()
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.flatten()
			.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
			.collect::<WorkflowResult<Vec<_>>>()
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

		for (k, v) in &tags {
			// Write tag key
			txs.write(
				&keys::workflow::TagKey::new(workflow_id, k.clone(), v.clone()),
				(),
			)?;

			// Write "by name and first tag" secondary index
			let rest_of_tags = tags
				.iter()
				.filter(|(k2, _)| k2 != k)
				.map(|(k, v)| (k.clone(), v.clone()))
				.collect();

			txs.write(
				&keys::workflow::ByNameAndTagKey::new(
					workflow_name.to_string(),
					k.clone(),
					v.clone(),
					workflow_id,
				),
				rest_of_tags,
			)?;
		}

		// Write null key for the "by name and first tag" secondary index (all workflows have this)
		txs.write(
			&keys::workflow::ByNameAndTagKey::null(workflow_name.to_string(), workflow_id),
			tags,
		)?;

		// Write input
		let input_key = keys::workflow::InputKey::new(workflow_id);

		for (i, chunk) in input_key
			.split_ref(input)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = input_key.chunk(i);

			txs.set(&self.subspace.pack(&chunk_key), &chunk);
		}

		// Write immediate wake condition
		txs.write(
			&keys::wake::WorkflowWakeConditionKey::new(
				workflow_name.to_string(),
				workflow_id,
				keys::wake::WakeCondition::Immediate,
			),
			(),
		)?;

		txs.write(&keys::workflow::HasWakeConditionKey::new(workflow_id), ())?;

		// Write metric
		update_metric(
			&txs,
			None,
			Some(keys::metric::GaugeMetric::WorkflowSleeping(
				workflow_name.to_string(),
			)),
		);

		Ok(workflow_id)
	}

	async fn find_workflow_inner(
		&self,
		workflow_name: &str,
		tags: &serde_json::Value,
		tx: &udb::RetryableTransaction,
	) -> Result<Option<Id>, udb::FdbBindingError> {
		// Convert to flat vec of strings
		let mut tag_iter = tags
			.as_object()
			.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.iter()
			.map(|(k, v)| {
				Result::<_, udb::FdbBindingError>::Ok((
					k.clone(),
					value_to_str(v).map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				))
			});
		let first_tag = tag_iter.next().transpose()?;
		let rest_of_tags = tag_iter.collect::<Result<Vec<_>, _>>()?;

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
			udb::RangeOption {
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
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			// Deserialize value
			let wf_rest_of_tags = workflow_by_name_and_tag_key
				.deserialize(entry.value())
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
}

#[async_trait::async_trait]
impl Database for DatabaseKv {
	fn worker_poll_interval(&self) -> std::time::Duration {
		std::time::Duration::from_secs(4)
	}

	async fn from_pools(pools: rivet_pools::Pools) -> anyhow::Result<Arc<Self>> {
		Ok(Arc::new(DatabaseKv {
			pools,
			subspace: udb_util::Subspace::new(&(RIVET, GASOLINE, KV)),
		}))
	}

	#[tracing::instrument(skip_all)]
	async fn wake_sub<'a, 'b>(&'a self) -> WorkflowResult<BoxStream<'b, ()>> {
		let mut subscriber = self
			.pools
			.ups()
			.map_err(WorkflowError::PoolsGeneric)?
			.subscribe(WORKER_WAKE_SUBJECT)
			.await
			.map_err(|x| WorkflowError::CreateSubscription(x.into()))?;

		let stream = async_stream::stream! {
			loop {
				use universalpubsub::NextOutput;
				match subscriber.next().await {
					Ok(NextOutput::Message(_)) => yield (),
					Ok(NextOutput::Unsubscribed) => break,
					Err(err) => {
						tracing::warn!(?err, "error in worker wake stream");
						break;
					}
				}
			}
		};

		Ok(stream.boxed())
	}

	#[tracing::instrument(skip_all)]
	async fn clear_expired_leases(&self, _worker_instance_id: Id) -> WorkflowResult<()> {
		let (lost_worker_instance_ids, expired_workflow_count) = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					let now = rivet_util::timestamp::now();

					let mut last_ping_cache: Vec<(Id, i64)> = Vec::new();
					let mut lost_worker_instance_ids = HashSet::new();
					let mut expired_workflow_count = 0;

					let lease_subspace = self
						.subspace
						.subspace(&keys::workflow::LeaseKey::subspace());

					// List all active leases
					let mut stream = tx.get_ranges_keyvalues(
						udb::RangeOption {
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
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
						let (workflow_name, worker_instance_id) = lease_key
							.deserialize(lease_key_entry.value())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
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
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
							);

							update_metric(
								&tx.subspace(self.subspace.clone()),
								Some(keys::metric::GaugeMetric::WorkflowActive(
									workflow_name.to_string(),
								)),
								Some(keys::metric::GaugeMetric::WorkflowSleeping(
									workflow_name.to_string(),
								)),
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
	async fn publish_metrics(&self, _worker_instance_id: Id) -> WorkflowResult<()> {
		// Attempt to be the only worker publishing metrics by writing to the lock key
		let acquired_lock = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					let txs = tx.subspace(self.subspace.clone());

					// Read existing lock
					let lock_expired = if let Some(lock_ts) = txs
						.read_opt(&keys::worker_instance::MetricsLockKey::new(), SERIALIZABLE)
						.await?
					{
						lock_ts < rivet_util::timestamp::now() - METRICS_LOCK_TIMEOUT_MS
					} else {
						true
					};

					if lock_expired {
						// Write to lock key. FDB transactions guarantee that if multiple workers are running this
						// query at the same time only one will succeed which means only one will have the lock.
						txs.write(
							&keys::worker_instance::MetricsLockKey::new(),
							rivet_util::timestamp::now(),
						)?;
					}

					Ok(lock_expired)
				}
			})
			.custom_instrument(tracing::info_span!("acquire_lock_tx"))
			.await?;

		if acquired_lock {
			let entries = self
				.pools
				.udb()
				.map_err(WorkflowError::PoolsGeneric)?
				.run(|tx, _mc| async move {
					let txs = tx.subspace(self.subspace.clone());

					let metrics_subspace = txs.subspace(&keys::metric::GaugeMetricKey::subspace());
					txs.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&metrics_subspace).into()
						},
						SERIALIZABLE,
					)
					.map(|res| match res {
						Ok(entry) => txs.read_entry::<keys::metric::GaugeMetricKey>(&entry),
						Err(err) => Err(err.into()),
					})
					.try_collect::<Vec<_>>()
					.await
				})
				.custom_instrument(tracing::info_span!("read_metrics_tx"))
				.await?;

			let mut total_workflow_counts: Vec<(String, usize)> = Vec::new();

			for (key, count) in entries {
				match key.metric {
					keys::metric::GaugeMetric::WorkflowActive(workflow_name) => {
						metrics::WORKFLOW_ACTIVE.record(
							count as u64,
							&[KeyValue::new("workflow_name", workflow_name.clone())],
						);

						if let Some(entry) = total_workflow_counts
							.iter_mut()
							.find(|(name, _)| name == &workflow_name)
						{
							entry.1 += 1;
						} else {
							total_workflow_counts.push((workflow_name, 1));
						}
					}
					keys::metric::GaugeMetric::WorkflowSleeping(workflow_name) => {
						metrics::WORKFLOW_SLEEPING.record(
							count as u64,
							&[KeyValue::new("workflow_name", workflow_name.clone())],
						);

						if let Some(entry) = total_workflow_counts
							.iter_mut()
							.find(|(name, _)| name == &workflow_name)
						{
							entry.1 += 1;
						} else {
							total_workflow_counts.push((workflow_name, 1));
						}
					}
					keys::metric::GaugeMetric::WorkflowDead(workflow_name, error) => {
						metrics::WORKFLOW_DEAD.record(
							count as u64,
							&[
								KeyValue::new("workflow_name", workflow_name.clone()),
								KeyValue::new("error", error),
							],
						);

						if let Some(entry) = total_workflow_counts
							.iter_mut()
							.find(|(name, _)| name == &workflow_name)
						{
							entry.1 += 1;
						} else {
							total_workflow_counts.push((workflow_name, 1));
						}
					}
					keys::metric::GaugeMetric::WorkflowComplete(workflow_name) => {
						if let Some(entry) = total_workflow_counts
							.iter_mut()
							.find(|(name, _)| name == &workflow_name)
						{
							entry.1 += 1;
						} else {
							total_workflow_counts.push((workflow_name, 1));
						}
					}
					keys::metric::GaugeMetric::SignalPending(signal_name) => {
						metrics::SIGNAL_PENDING
							.record(count as u64, &[KeyValue::new("signal_name", signal_name)]);
					}
				}
			}

			for (workflow_name, count) in total_workflow_counts {
				metrics::WORKFLOW_TOTAL.record(
					count as u64,
					&[KeyValue::new("workflow_name", workflow_name.clone())],
				);
			}

			// Clear lock
			self.pools
				.udb()
				.map_err(WorkflowError::PoolsGeneric)?
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
	async fn update_worker_ping(&self, worker_instance_id: Id) -> WorkflowResult<()> {
		metrics::WORKER_LAST_PING.record(
			rivet_util::timestamp::now() as u64,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id.to_string(),
			)],
		);

		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					// Update worker instance ping
					let last_ping_ts_key =
						keys::worker_instance::LastPingTsKey::new(worker_instance_id);
					tx.set(
						&self.subspace.pack(&last_ping_ts_key),
						&last_ping_ts_key
							.serialize(rivet_util::timestamp::now())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
		ray_id: Id,
		workflow_id: Id,
		workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		unique: bool,
	) -> WorkflowResult<Id> {
		let workflow_id = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				self.dispatch_workflow_inner(
					ray_id,
					workflow_id,
					workflow_name,
					tags,
					input,
					unique,
					&tx,
				)
				.await
			})
			.custom_instrument(tracing::info_span!("dispatch_workflow_tx"))
			.await?;

		self.wake_worker();

		Ok(workflow_id)
	}

	#[tracing::instrument(skip_all, fields(?workflow_ids))]
	async fn get_workflows(&self, workflow_ids: Vec<Id>) -> WorkflowResult<Vec<WorkflowData>> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				let workflow_ids = workflow_ids.clone();
				async move {
					futures_util::stream::iter(workflow_ids)
						.map(|workflow_id| {
							let tx = tx.clone();
							async move {
								let input_key = keys::workflow::InputKey::new(workflow_id);
								let input_subspace = self.subspace.subspace(&input_key);
								let state_key = keys::workflow::StateKey::new(workflow_id);
								let state_subspace = self.subspace.subspace(&state_key);
								let output_key = keys::workflow::OutputKey::new(workflow_id);
								let output_subspace = self.subspace.subspace(&output_key);
								let has_wake_condition_key =
									keys::workflow::HasWakeConditionKey::new(workflow_id);

								// Read input and output
								let (
									input_chunks,
									state_chunks,
									output_chunks,
									has_wake_condition_entry,
								) = tokio::try_join!(
									tx.get_ranges_keyvalues(
										udb::RangeOption {
											mode: StreamingMode::WantAll,
											..(&input_subspace).into()
										},
										SERIALIZABLE,
									)
									.try_collect::<Vec<_>>(),
									tx.get_ranges_keyvalues(
										udb::RangeOption {
											mode: StreamingMode::WantAll,
											..(&state_subspace).into()
										},
										SERIALIZABLE,
									)
									.try_collect::<Vec<_>>(),
									tx.get_ranges_keyvalues(
										udb::RangeOption {
											mode: StreamingMode::WantAll,
											..(&output_subspace).into()
										},
										SERIALIZABLE,
									)
									.try_collect::<Vec<_>>(),
									tx.get(
										&self.subspace.pack(&has_wake_condition_key),
										SERIALIZABLE
									),
								)?;

								if input_chunks.is_empty() {
									Ok(None)
								} else {
									let input = input_key
										.combine(input_chunks)
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									let state = if state_chunks.is_empty() {
										serde_json::value::RawValue::NULL.to_owned()
									} else {
										state_key.combine(state_chunks).map_err(|x| {
											udb::FdbBindingError::CustomError(x.into())
										})?
									};

									let output = if output_chunks.is_empty() {
										None
									} else {
										Some(output_key.combine(output_chunks).map_err(|x| {
											udb::FdbBindingError::CustomError(x.into())
										})?)
									};

									Ok(Some(WorkflowData {
										workflow_id,
										input,
										state,
										output,
										has_wake_condition: has_wake_condition_entry.is_some(),
									}))
								}
							}
						})
						.buffered(256)
						.try_filter_map(|x| std::future::ready(Ok(x)))
						.try_collect::<Vec<_>>()
						.instrument(tracing::trace_span!("get_workflows"))
						.await
				}
			})
			.custom_instrument(tracing::info_span!("get_workflow_tx"))
			.await
			.map_err(Into::into)
	}

	/// Returns the first incomplete workflow with the given name and tags, first meaning the one with the
	/// lowest id value (interpreted as u128) because its in a KV store. There is no way to get any other
	/// workflow besides the first.
	#[tracing::instrument(skip_all, fields(%workflow_name))]
	async fn find_workflow(
		&self,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<Option<Id>> {
		let start_instant = Instant::now();

		let workflow_id = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move { self.find_workflow_inner(workflow_name, tags, &tx).await })
			.custom_instrument(tracing::info_span!("find_workflow_tx"))
			.await?;

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::FIND_WORKFLOWS_DURATION.record(
			dt,
			&[KeyValue::new("workflow_name", workflow_name.to_string())],
		);

		Ok(workflow_id)
	}

	#[tracing::instrument(skip_all)]
	async fn pull_workflows(
		&self,
		worker_instance_id: Id,
		filter: &[&str],
	) -> WorkflowResult<Vec<PulledWorkflowData>> {
		let start_instant = Instant::now();
		let owned_filter = filter
			.into_iter()
			.map(|x| x.to_string())
			.collect::<Vec<_>>();

		let leased_workflows = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				let owned_filter = owned_filter.clone();

				async move {
					let now = rivet_util::timestamp::now();

					// All wake conditions with a timestamp after this timestamp will be pulled
					let pull_before = now
						+ i64::try_from(self.worker_poll_interval().as_millis())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
								udb::RangeOption {
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
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
							)),
							Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
						})
						.try_collect::<Vec<_>>()
						.await?;

					// Collect name and deadline ts for each wf id
					let mut dedup_workflows: Vec<(Id, String, Option<i64>)> = Vec::new();
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
									Result::<_, udb::FdbBindingError>::Ok(None)
								} else {
									// Write lease
									tx.set(
										&lease_key_buf,
										&lease_key
											.serialize((workflow_name.clone(), worker_instance_id))
											.map_err(|x| {
												udb::FdbBindingError::CustomError(x.into())
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
												udb::FdbBindingError::CustomError(x.into())
											})?,
									);

									update_metric(
										&tx.subspace(self.subspace.clone()),
										Some(keys::metric::GaugeMetric::WorkflowSleeping(
											workflow_name.clone(),
										)),
										Some(keys::metric::GaugeMetric::WorkflowActive(
											workflow_name.clone(),
										)),
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
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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

					// NOTE: We don't read any workflow data in this txn since its only for acquiring leases.
					// The less operations we do in this txn the less contention there is with other workers.
					Ok(leased_workflows)
				}
			})
			.custom_instrument(tracing::info_span!("pull_workflows_tx"))
			.await?;

		let worker_instance_id_str = worker_instance_id.to_string();
		let dt = start_instant.elapsed().as_secs_f64();
		metrics::LAST_PULL_WORKFLOWS_DURATION.record(
			dt,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);
		metrics::PULL_WORKFLOWS_DURATION.record(
			dt,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);

		if leased_workflows.is_empty() {
			return Ok(Vec::new());
		}

		let start_instant2 = Instant::now();

		let pulled_workflows = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				let leased_workflows = leased_workflows.clone();

				async move {
					// Read required wf data for each leased wf
					futures_util::stream::iter(leased_workflows)
						.map(|(workflow_id, workflow_name, wake_deadline_ts)| {
							let tx = tx.clone();
							async move {
								let create_ts_key = keys::workflow::CreateTsKey::new(workflow_id);
								let ray_id_key = keys::workflow::RayIdKey::new(workflow_id);
								let input_key = keys::workflow::InputKey::new(workflow_id);
								let state_key = keys::workflow::StateKey::new(workflow_id);
								let input_subspace = self.subspace.subspace(&input_key);
								let state_subspace = self.subspace.subspace(&state_key);
								let active_history_subspace = self.subspace.subspace(
									&keys::history::HistorySubspaceKey::new(
										workflow_id,
										keys::history::HistorySubspaceVariant::Active,
									),
								);

								let (
									create_ts_entry,
									ray_id_entry,
									input_chunks,
									state_chunks,
									events,
								) = tokio::try_join!(
									async {
										tx.get(&self.subspace.pack(&create_ts_key), SERIALIZABLE)
											.await
											.map_err(|x| {
												udb::FdbBindingError::CustomError(x.into())
											})
									},
									async {
										tx.get(&self.subspace.pack(&ray_id_key), SERIALIZABLE)
											.await
											.map_err(|x| {
												udb::FdbBindingError::CustomError(x.into())
											})
									},
									async {
										tx.get_ranges_keyvalues(
											udb::RangeOption {
												mode: StreamingMode::WantAll,
												..(&input_subspace).into()
											},
											SERIALIZABLE,
										)
										.try_collect::<Vec<_>>()
										.await
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
									},
									async {
										tx.get_ranges_keyvalues(
											udb::RangeOption {
												mode: StreamingMode::WantAll,
												..(&state_subspace).into()
											},
											SERIALIZABLE,
										)
										.try_collect::<Vec<_>>()
										.await
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
									},
									async {
										let mut events_by_location: HashMap<Location, Vec<Event>> =
											HashMap::new();
										let mut current_event =
											WorkflowHistoryEventBuilder::new(Location::empty());

										let mut stream = tx.get_ranges_keyvalues(
											udb::RangeOption {
												mode: StreamingMode::WantAll,
												..(&active_history_subspace).into()
											},
											SERIALIZABLE,
										);

										loop {
											let Some(entry) = stream.try_next().await? else {
												break;
											};

											// Parse only the wf id and location of the current key
											let partial_key = self
												.subspace
												.unpack::<keys::history::PartialEventKey>(
													entry.key(),
												)
												.map_err(|x| {
													udb::FdbBindingError::CustomError(x.into())
												})?;

											if current_event.location != partial_key.location {
												if current_event.location.is_empty() {
													current_event =
														WorkflowHistoryEventBuilder::new(
															partial_key.location,
														);
												} else {
													// Insert current event builder to into wf events and
													// reset state
													let previous_event = std::mem::replace(
														&mut current_event,
														WorkflowHistoryEventBuilder::new(
															partial_key.location,
														),
													);
													events_by_location
														.entry(previous_event.location.root())
														.or_default()
														.push(
															Event::try_from(previous_event)
																.map_err(|x| {
																	udb::FdbBindingError::CustomError(x.into())
																})?,
														);
												}
											}

											// Parse current key as any event key
											if let Ok(key) =
												self.subspace.unpack::<keys::history::EventTypeKey>(
													entry.key(),
												) {
												let event_type = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.event_type = Some(event_type);
											} else if let Ok(key) =
												self.subspace.unpack::<keys::history::VersionKey>(
													entry.key(),
												) {
												let version = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.version = Some(version);
											} else if let Ok(key) =
												self.subspace.unpack::<keys::history::CreateTsKey>(
													entry.key(),
												) {
												let create_ts = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.create_ts = Some(create_ts);
											} else if let Ok(key) =
												self.subspace
													.unpack::<keys::history::NameKey>(entry.key())
											{
												let name = key.deserialize(entry.value()).map_err(
													|x| udb::FdbBindingError::CustomError(x.into()),
												)?;

												current_event.name = Some(name);
											} else if let Ok(key) =
												self.subspace.unpack::<keys::history::SignalIdKey>(
													entry.key(),
												) {
												let signal_id = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.signal_id = Some(signal_id);
											} else if let Ok(key) = self
												.subspace
												.unpack::<keys::history::SubWorkflowIdKey>(
												entry.key(),
											) {
												let sub_workflow_id = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.sub_workflow_id =
													Some(sub_workflow_id);
											} else if let Ok(_key) = self
												.subspace
												.unpack::<keys::history::InputChunkKey>(
												entry.key(),
											) {
												current_event.input_chunks.push(entry);
											} else if let Ok(_key) = self
												.subspace
												.unpack::<keys::history::OutputChunkKey>(
												entry.key(),
											) {
												current_event.output_chunks.push(entry);
											} else if let Ok(key) =
												self.subspace.unpack::<keys::history::InputHashKey>(
													entry.key(),
												) {
												let input_hash = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.input_hash = Some(input_hash);
											} else if let Ok(_key) =
												self.subspace
													.unpack::<keys::history::ErrorKey>(entry.key())
											{
												current_event.error_count += 1;
											} else if let Ok(key) =
												self.subspace.unpack::<keys::history::IterationKey>(
													entry.key(),
												) {
												let iteration = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.iteration = Some(iteration);
											} else if let Ok(key) = self
												.subspace
												.unpack::<keys::history::DeadlineTsKey>(
												entry.key(),
											) {
												let deadline_ts = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.deadline_ts = Some(deadline_ts);
											} else if let Ok(key) = self
												.subspace
												.unpack::<keys::history::SleepStateKey>(
												entry.key(),
											) {
												let sleep_state = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.sleep_state = Some(sleep_state);
											} else if let Ok(key) = self
												.subspace
												.unpack::<keys::history::InnerEventTypeKey>(
												entry.key(),
											) {
												let inner_event_type = key
													.deserialize(entry.value())
													.map_err(|x| {
														udb::FdbBindingError::CustomError(x.into())
													})?;

												current_event.inner_event_type =
													Some(inner_event_type);
											}

											// We ignore keys we don't need (like tags)
										}
										// Insert final event
										if !current_event.location.is_empty() {
											events_by_location
												.entry(current_event.location.root())
												.or_default()
												.push(Event::try_from(current_event).map_err(
													|x| udb::FdbBindingError::CustomError(x.into()),
												)?);
										}

										Ok(events_by_location)
									}
								)?;

								let create_ts = create_ts_key
									.deserialize(&create_ts_entry.ok_or(
										udb::FdbBindingError::CustomError(
											format!("key should exist: {create_ts_key:?}").into(),
										),
									)?)
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
								let ray_id = ray_id_key
									.deserialize(&ray_id_entry.ok_or(
										udb::FdbBindingError::CustomError(
											format!("key should exist: {ray_id_key:?}").into(),
										),
									)?)
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
								let input = input_key
									.combine(input_chunks)
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
								let state = if state_chunks.is_empty() {
									serde_json::value::RawValue::NULL.to_owned()
								} else {
									state_key
										.combine(state_chunks)
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
								};

								Result::<_, udb::FdbBindingError>::Ok(PulledWorkflowData {
									workflow_id,
									workflow_name,
									create_ts,
									ray_id,
									input,
									state,
									wake_deadline_ts,
									events,
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
			.custom_instrument(tracing::info_span!("pull_workflow_history_tx"))
			.await?;

		let dt2 = start_instant2.elapsed().as_secs_f64();
		let dt = start_instant.elapsed().as_secs_f64();
		metrics::LAST_PULL_WORKFLOWS_FULL_DURATION.record(
			dt,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);
		metrics::PULL_WORKFLOWS_FULL_DURATION.record(
			dt,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);
		metrics::LAST_PULL_WORKFLOWS_HISTORY_DURATION.record(
			dt2 as u64,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);
		metrics::PULL_WORKFLOWS_HISTORY_DURATION.record(
			dt2,
			&[KeyValue::new(
				"worker_instance_id",
				worker_instance_id_str.clone(),
			)],
		);

		Ok(pulled_workflows)
	}

	#[tracing::instrument(skip_all)]
	async fn complete_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		output: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		let wrote_to_wake_idx = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
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
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&sub_workflow_wake_subspace).into()
						},
						// NOTE: Must be serializable to conflict with `get_sub_workflow`
						SERIALIZABLE,
					);

					let (wrote_to_wake_idx, tag_keys, wake_deadline_entry) = tokio::try_join!(
						// Check for other workflows waiting on this one, wake all
						async {
							let mut wrote_to_wake_idx = false;

							while let Some(entry) = stream.try_next().await? {
								let sub_workflow_wake_key = self
									.subspace
									.unpack::<keys::wake::SubWorkflowWakeKey>(&entry.key())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
								let workflow_name = sub_workflow_wake_key
									.deserialize(entry.value())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
								);

								// Clear secondary index
								tx.clear(entry.key());

								wrote_to_wake_idx = true;
							}

							Result::<_, udb::FdbBindingError>::Ok(wrote_to_wake_idx)
						},
						// Read tags
						async {
							tx.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&tags_subspace).into()
								},
								SERIALIZABLE,
							)
							.map(|res| match res {
								Ok(entry) => self
									.subspace
									.unpack::<keys::workflow::TagKey>(entry.key())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into())),
								Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
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
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
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

					update_metric(
						&tx.subspace(self.subspace.clone()),
						Some(keys::metric::GaugeMetric::WorkflowActive(
							workflow_name.to_string(),
						)),
						Some(keys::metric::GaugeMetric::WorkflowComplete(
							workflow_name.to_string(),
						)),
					);

					Ok(wrote_to_wake_idx)
				}
			})
			.custom_instrument(tracing::info_span!("complete_workflows_tx"))
			.await?;

		// Wake worker again in case some other workflow was waiting for this one to complete
		if wrote_to_wake_idx {
			self.wake_worker();
		}

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::COMPLETE_WORKFLOW_DURATION.record(
			dt,
			&[KeyValue::new("workflow_name", workflow_name.to_string())],
		);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		wake_immediate: bool,
		wake_deadline_ts: Option<i64>,
		wake_signals: &[&str],
		wake_sub_workflow_id: Option<Id>,
		error: &str,
	) -> WorkflowResult<()> {
		let start_instant = Instant::now();

		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
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
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
						);

						// Write to wake deadline
						tx.set(
							&self.subspace.pack(&wake_deadline_key),
							&wake_deadline_key
								.serialize(deadline_ts)
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
					let has_wake_condition = wake_immediate
						|| wake_deadline_ts.is_some()
						|| !wake_signals.is_empty()
						|| wake_sub_workflow_id.is_some();
					if has_wake_condition {
						tx.set(
							&self.subspace.pack(&has_wake_condition_key),
							&has_wake_condition_key
								.serialize(())
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
					);

					// Clear lease
					let lease_key = keys::workflow::LeaseKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&lease_key));
					let worker_instance_id_key =
						keys::workflow::WorkerInstanceIdKey::new(workflow_id);
					tx.clear(&self.subspace.pack(&worker_instance_id_key));

					update_metric(
						&tx.subspace(self.subspace.clone()),
						Some(keys::metric::GaugeMetric::WorkflowActive(
							workflow_name.to_string(),
						)),
						Some(if has_wake_condition {
							keys::metric::GaugeMetric::WorkflowSleeping(workflow_name.to_string())
						} else {
							keys::metric::GaugeMetric::WorkflowDead(
								workflow_name.to_string(),
								error.to_string(),
							)
						}),
					);

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("commit_workflow_tx"))
			.await?;

		// Always wake the worker immediately again. This is an IMPORTANT implementation detail to prevent
		// race conditions with workflow sleep. Imagine the scenario:
		//
		// 1. workflow is between user code and commit
		// 2. worker reads wake condition for said workflow but cannot run it because it is already leased
		// 3. workflow commits
		//
		// This will result in the workflow sleeping instead of immediately running again.
		//
		// Adding this wake_worker call ensures that if the workflow has a valid wake condition before commit
		// then it will immediately wake up again.
		//
		// This is simpler than having this commit_workflow fn read wake conditions because:
		// - the wake conditions are not indexed by wf id
		// - would involve informing the worker to restart the workflow in memory instead of the usual
		//   workflow lifecycle
		// - the worker is already designed to pull wake conditions frequently
		self.wake_worker();

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::COMMIT_WORKFLOW_DURATION.record(
			dt,
			&[KeyValue::new("workflow_name", workflow_name.to_string())],
		);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn pull_next_signal(
		&self,
		workflow_id: Id,
		_workflow_name: &str,
		filter: &[&str],
		location: &Location,
		version: usize,
		_loop_location: Option<&Location>,
		last_try: bool,
	) -> WorkflowResult<Option<SignalData>> {
		let owned_filter = filter
			.into_iter()
			.map(|x| x.to_string())
			.collect::<Vec<_>>();

		// Fetch signal from FDB
		let signal =
			self.pools
				.udb()
				.map_err(WorkflowError::PoolsGeneric)?
				.run(|tx, _mc| {
					let owned_filter = owned_filter.clone();

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
										udb::RangeOption {
											mode: StreamingMode::WantAll,
											limit: Some(1),
											..(&pending_signal_subspace).into()
										},
										// NOTE: This is serializable because any insert into this subspace
										// should cause a conflict and retry of this txn
										SERIALIZABLE,
									)
								})
								.collect::<Vec<_>>();

							// Fetch the next entry from all streams at the same time
							let mut results = futures_util::future::try_join_all(
								streams.into_iter().map(|mut stream| async move {
									if let Some(entry) = stream.try_next().await? {
										Result::<_, udb::FdbBindingError>::Ok(Some((
											entry.key().to_vec(),
											self.subspace
												.unpack::<keys::workflow::PendingSignalKey>(
													&entry.key(),
												)
												.map_err(|x| {
													udb::FdbBindingError::CustomError(x.into())
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
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
							);

							update_metric(
								&tx.subspace(self.subspace.clone()),
								Some(keys::metric::GaugeMetric::SignalPending(
									signal_name.to_string(),
								)),
								None,
							);

							// TODO: Split txn into two after acking here?

							// Clear pending signal key
							tx.clear(&raw_key);

							// Read signal body
							let body_key = keys::signal::BodyKey::new(signal_id);
							let body_subspace = self.subspace.subspace(&body_key);

							let chunks = tx
								.get_ranges_keyvalues(
									udb::RangeOption {
										mode: StreamingMode::WantAll,
										..(&body_subspace).into()
									},
									SERIALIZABLE,
								)
								.try_collect::<Vec<_>>()
								.await?;

							let body = body_key
								.combine(chunks)
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

							// Insert history event
							keys::history::insert::signal_event(
								&self.subspace,
								&tx,
								workflow_id,
								&location,
								version,
								rivet_util::timestamp::now(),
								signal_id,
								&signal_name,
								&body,
							)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

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

		Ok(signal)
	}

	#[tracing::instrument(skip_all)]
	async fn get_sub_workflow(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		sub_workflow_id: Id,
	) -> WorkflowResult<Option<WorkflowData>> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					let input_key = keys::workflow::InputKey::new(sub_workflow_id);
					let input_subspace = self.subspace.subspace(&input_key);
					let state_key = keys::workflow::StateKey::new(sub_workflow_id);
					let state_subspace = self.subspace.subspace(&state_key);
					let output_key = keys::workflow::OutputKey::new(sub_workflow_id);
					let output_subspace = self.subspace.subspace(&output_key);
					let has_wake_condition_key =
						keys::workflow::HasWakeConditionKey::new(sub_workflow_id);

					// Read input and output
					let (input_chunks, state_chunks, output_chunks, has_wake_condition_entry) = tokio::try_join!(
						tx.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&input_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&state_subspace).into()
							},
							SERIALIZABLE,
						)
						.try_collect::<Vec<_>>(),
						tx.get_ranges_keyvalues(
							udb::RangeOption {
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
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						let state = if state_chunks.is_empty() {
							serde_json::value::RawValue::NULL.to_owned()
						} else {
							state_key
								.combine(state_chunks)
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
						};

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
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
							)
						};

						Ok(Some(WorkflowData {
							workflow_id: sub_workflow_id,
							input,
							state,
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
		ray_id: Id,
		workflow_id: Id,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				self.publish_signal_inner(ray_id, workflow_id, signal_id, signal_name, body, &tx)
					.await
			})
			.custom_instrument(tracing::info_span!("publish_signal_tx"))
			.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn publish_signal_from_workflow(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		ray_id: Id,
		to_workflow_id: Id,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				self.publish_signal_inner(
					ray_id,
					to_workflow_id,
					signal_id,
					signal_name,
					body,
					&tx,
				)
				.await?;

				// Insert history event
				keys::history::insert::signal_send_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					&location,
					version,
					rivet_util::timestamp::now(),
					signal_id,
					&signal_name,
					&body,
					to_workflow_id,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("publish_signal_from_workflow_tx"))
			.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(%sub_workflow_id, %sub_workflow_name, unique))]
	async fn dispatch_sub_workflow(
		&self,
		ray_id: Id,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		sub_workflow_id: Id,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
		_loop_location: Option<&Location>,
		unique: bool,
	) -> WorkflowResult<Id> {
		let sub_workflow_id = self
			.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				let sub_workflow_id = self
					.dispatch_workflow_inner(
						ray_id,
						sub_workflow_id,
						sub_workflow_name,
						tags,
						input,
						unique,
						&tx,
					)
					.await?;

				// Insert history event
				keys::history::insert::sub_workflow_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					&location,
					version,
					rivet_util::timestamp::now(),
					sub_workflow_id,
					sub_workflow_name,
					tags,
					input,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(sub_workflow_id)
			})
			.custom_instrument(tracing::info_span!("dispatch_sub_workflow_tx"))
			.await?;

		self.wake_worker();

		Ok(sub_workflow_id)
	}

	#[tracing::instrument(skip_all)]
	async fn update_workflow_tags(
		&self,
		workflow_id: Id,
		workflow_name: &str,
		tags: &serde_json::Value,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					let tags_subspace = self
						.subspace
						.subspace(&keys::workflow::TagKey::subspace(workflow_id));

					// Read old tags
					let tag_keys = tx
						.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&tags_subspace).into()
							},
							SERIALIZABLE,
						)
						.map(|res| match res {
							Ok(entry) => self
								.subspace
								.unpack::<keys::workflow::TagKey>(entry.key())
								.map_err(|x| udb::FdbBindingError::CustomError(x.into())),
							Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
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
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
						.collect::<WorkflowResult<Vec<_>>>()
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

					for (k, v) in &tags {
						let tag_key =
							keys::workflow::TagKey::new(workflow_id, k.clone(), v.clone());
						tx.set(
							&self.subspace.pack(&tag_key),
							&tag_key
								.serialize(())
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
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
	async fn update_workflow_state(
		&self,
		workflow_id: Id,
		state: &serde_json::value::RawValue,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| {
				async move {
					let state_key = keys::workflow::StateKey::new(workflow_id);

					// Write state
					for (i, chunk) in state_key
						.split_ref(&state)
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
						.into_iter()
						.enumerate()
					{
						let chunk_key = state_key.chunk(i);

						tx.set(&self.subspace.pack(&chunk_key), &chunk);
					}

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("update_workflow_state_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_activity_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		event_id: &EventId,
		create_ts: i64,
		input: &serde_json::value::RawValue,
		res: Result<&serde_json::value::RawValue, &str>,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::activity_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					version,
					create_ts,
					&event_id.name,
					&event_id.input_hash.to_be_bytes(),
					input,
					res,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_activity_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_message_send_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		tags: &serde_json::Value,
		message_name: &str,
		body: &serde_json::value::RawValue,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::message_send_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					version,
					rivet_util::timestamp::now(),
					tags,
					message_name,
					body,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_message_send_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn upsert_workflow_loop_event(
		&self,
		from_workflow_id: Id,
		_workflow_name: &str,
		location: &Location,
		version: usize,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				if iteration == 0 {
					keys::history::insert::loop_event(
						&self.subspace,
						&tx,
						from_workflow_id,
						location,
						version,
						rivet_util::timestamp::now(),
						iteration,
						state,
						output,
					)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
				} else {
					keys::history::insert::update_loop_event(
						&self.subspace,
						&tx,
						from_workflow_id,
						location,
						iteration,
						state,
						output,
					)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

					let active_history_subspace =
						self.subspace
							.subspace(&keys::history::HistorySubspaceKey::new(
								from_workflow_id,
								keys::history::HistorySubspaceVariant::Active,
							));

					let forgotten_history_subspace =
						self.subspace
							.subspace(&keys::history::HistorySubspaceKey::new(
								from_workflow_id,
								keys::history::HistorySubspaceVariant::Forgotten,
							));

					let loop_events_subspace =
						self.subspace
							.subspace(&keys::history::EventHistorySubspaceKey::entire(
								from_workflow_id,
								location.clone(),
								false,
							));

					let mut stream = tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&loop_events_subspace).into()
						},
						SERIALIZABLE,
					);

					// Move all current events under this loop to the forgotten history
					loop {
						let Some(entry) = stream.try_next().await? else {
							break;
						};

						if !active_history_subspace.is_start_of(entry.key()) {
							return Err(udb::FdbBindingError::CustomError(
								udb::tuple::PackError::BadPrefix.into(),
							));
						}

						// Truncate tuple up to ACTIVE and replace it with FORGOTTEN
						let truncated_key = &entry.key()[active_history_subspace.bytes().len()..];
						let forgotten_key =
							[forgotten_history_subspace.bytes(), truncated_key].concat();

						tx.set(&forgotten_key, entry.value());
					}

					tx.clear_subspace_range(&loop_events_subspace);

					// Only retain last 100 events in forgotten history
					if iteration > 100 {
						let old_forgotten_subspace_start =
							self.subspace
								.pack(&keys::history::EventHistorySubspaceKey::new(
									from_workflow_id,
									location.clone(),
									0,
									true,
								));
						let old_forgotten_subspace_end =
							self.subspace
								.pack(&keys::history::EventHistorySubspaceKey::new(
									from_workflow_id,
									location.clone(),
									iteration - 100,
									true,
								));

						tx.clear_range(&old_forgotten_subspace_start, &old_forgotten_subspace_end);
					}
				}

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_sleep_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_sleep_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		deadline_ts: i64,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::sleep_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					version,
					rivet_util::timestamp::now(),
					deadline_ts,
					SleepState::Normal,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_sleep_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn update_workflow_sleep_event_state(
		&self,
		from_workflow_id: Id,
		location: &Location,
		state: SleepState,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::update_sleep_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					state,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("update_workflow_sleep_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_branch_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::branch_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					version,
					rivet_util::timestamp::now(),
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_branch_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_removed_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		event_type: EventType,
		event_name: Option<&str>,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::removed_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					1, // Default
					rivet_util::timestamp::now(),
					event_type,
					event_name,
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!("commit_workflow_removed_event_tx"))
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn commit_workflow_version_check_event(
		&self,
		from_workflow_id: Id,
		location: &Location,
		version: usize,
		_loop_location: Option<&Location>,
	) -> WorkflowResult<()> {
		self.pools
			.udb()
			.map_err(WorkflowError::PoolsGeneric)?
			.run(|tx, _mc| async move {
				keys::history::insert::version_check_event(
					&self.subspace,
					&tx,
					from_workflow_id,
					location,
					version,
					rivet_util::timestamp::now(),
				)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				Ok(())
			})
			.custom_instrument(tracing::info_span!(
				"commit_workflow_version_check_event_tx"
			))
			.await?;

		Ok(())
	}
}

fn update_metric(
	txs: &udb_util::TxnSubspace,
	previous: Option<keys::metric::GaugeMetric>,
	current: Option<keys::metric::GaugeMetric>,
) {
	if &previous == &current {
		return;
	}

	if let Some(previous) = previous {
		txs.atomic_op(
			&keys::metric::GaugeMetricKey::new(previous),
			&(-1isize).to_le_bytes(),
			MutationType::Add,
		);
	}

	if let Some(current) = current {
		txs.atomic_op(
			&keys::metric::GaugeMetricKey::new(current),
			&1usize.to_le_bytes(),
			MutationType::Add,
		);
	}
}

struct WorkflowHistoryEventBuilder {
	location: Location,
	event_type: Option<EventType>,
	version: Option<usize>,
	create_ts: Option<i64>,
	name: Option<String>,
	signal_id: Option<Id>,
	sub_workflow_id: Option<Id>,
	input_chunks: Vec<FdbValue>,
	output_chunks: Vec<FdbValue>,
	input_hash: Option<Vec<u8>>,
	error_count: usize,
	iteration: Option<usize>,
	deadline_ts: Option<i64>,
	sleep_state: Option<SleepState>,
	inner_event_type: Option<EventType>,
}

impl WorkflowHistoryEventBuilder {
	fn new(location: Location) -> Self {
		WorkflowHistoryEventBuilder {
			location,
			event_type: None,
			version: None,
			create_ts: None,
			name: None,
			signal_id: None,
			sub_workflow_id: None,
			input_chunks: Vec::new(),
			output_chunks: Vec::new(),
			input_hash: None,
			error_count: 0,
			iteration: None,
			deadline_ts: None,
			sleep_state: None,
			inner_event_type: None,
		}
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for Event {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		let event_type = value
			.event_type
			.ok_or(WorkflowError::MissingEventData("event_type"))?;

		Ok(Event {
			coordinate: value
				.location
				.tail()
				.cloned()
				.ok_or(WorkflowError::MissingEventData("location"))?,
			version: value
				.version
				.ok_or(WorkflowError::MissingEventData("version"))?,
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

impl TryFrom<WorkflowHistoryEventBuilder> for ActivityEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(ActivityEvent {
			event_id: EventId::from_be_bytes(
				value.name.ok_or(WorkflowError::MissingEventData("name"))?,
				value
					.input_hash
					.ok_or(WorkflowError::MissingEventData("hash"))?,
			)?,
			create_ts: value
				.create_ts
				.ok_or(WorkflowError::MissingEventData("create_ts"))?,
			output: {
				if value.output_chunks.is_empty() {
					None
				} else {
					// workflow_id not needed
					let output_key = keys::history::OutputKey::new(Id::nil(), value.location);
					Some(
						output_key
							.combine(value.output_chunks)
							.map_err(WorkflowError::DeserializeEventData)?,
					)
				}
			},
			error_count: value.error_count,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for SignalEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(SignalEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
			body: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location);
					input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for SignalSendEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(SignalSendEvent {
			signal_id: value
				.signal_id
				.ok_or(WorkflowError::MissingEventData("signal_id"))?,
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for MessageSendEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(MessageSendEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for SubWorkflowEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(SubWorkflowEvent {
			sub_workflow_id: value
				.sub_workflow_id
				.ok_or(WorkflowError::MissingEventData("sub_workflow_id"))?,
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for LoopEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(LoopEvent {
			state: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location.clone());
					input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
			output: {
				if value.output_chunks.is_empty() {
					None
				} else {
					// workflow_id not needed
					let output_key = keys::history::OutputKey::new(Id::nil(), value.location);
					Some(
						output_key
							.combine(value.output_chunks)
							.map_err(WorkflowError::DeserializeEventData)?,
					)
				}
			},
			iteration: value
				.iteration
				.ok_or(WorkflowError::MissingEventData("iteration"))?
				.try_into()
				.map_err(|_| WorkflowError::IntegerConversion)?,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for SleepEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(SleepEvent {
			deadline_ts: value
				.deadline_ts
				.ok_or(WorkflowError::MissingEventData("deadline_ts"))?,
			state: value
				.sleep_state
				.ok_or(WorkflowError::MissingEventData("sleep_state"))?,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for RemovedEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(RemovedEvent {
			name: value.name,
			event_type: value
				.inner_event_type
				.ok_or(WorkflowError::MissingEventData("inner_event_type"))?,
		})
	}
}

fn value_to_str(v: &serde_json::Value) -> WorkflowResult<String> {
	match v {
		serde_json::Value::String(s) => Ok(s.clone()),
		_ => cjson::to_string(&v).map_err(WorkflowError::CjsonSerializeTags),
	}
}
