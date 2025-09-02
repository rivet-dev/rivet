use std::{
	collections::HashMap,
	ops::Deref,
	result::Result::{Err, Ok},
};

use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use rivet_util::Id;
use tracing::Instrument;
use udb_util::{FormalChunkedKey, FormalKey, SERIALIZABLE, SNAPSHOT, TxnExt, end_of_key_range};
use universaldb::{
	self as udb,
	future::FdbValue,
	options::{ConflictRangeType, StreamingMode},
	tuple::{PackResult, TupleDepth, TupleUnpack},
};

use super::{DatabaseKv, keys, update_metric};
use crate::{
	db::debug::{
		ActivityError, ActivityEvent, DatabaseDebug, Event, EventData, HistoryData, LoopEvent,
		MessageSendEvent, SignalData, SignalEvent, SignalSendEvent, SignalState, SubWorkflowEvent,
		WorkflowData, WorkflowState,
	},
	error::{WorkflowError, WorkflowResult},
	history::{
		event::{EventType, RemovedEvent, SleepEvent, SleepState},
		location::Location,
	},
};

impl DatabaseKv {
	#[tracing::instrument(skip_all)]
	async fn get_workflows_inner(
		&self,
		workflow_ids: Vec<Id>,
		tx: &udb::RetryableTransaction,
	) -> std::result::Result<Vec<WorkflowData>, udb::FdbBindingError> {
		let mut res = Vec::new();

		// TODO: Parallelize
		for workflow_id in workflow_ids {
			// TODO: Do a single range read for all keys under workflow/data
			let name_key = keys::workflow::NameKey::new(workflow_id);
			let tags_subspace_key = keys::workflow::TagKey::subspace(workflow_id);
			let tags_subspace = self.subspace.subspace(&tags_subspace_key);
			let create_ts_key = keys::workflow::CreateTsKey::new(workflow_id);
			let input_key = keys::workflow::InputKey::new(workflow_id);
			let input_subspace = self.subspace.subspace(&input_key);
			let state_key = keys::workflow::StateKey::new(workflow_id);
			let state_subspace = self.subspace.subspace(&state_key);
			let output_key = keys::workflow::OutputKey::new(workflow_id);
			let output_subspace = self.subspace.subspace(&output_key);
			let error_key = keys::workflow::ErrorKey::new(workflow_id);
			let has_wake_condition_key = keys::workflow::HasWakeConditionKey::new(workflow_id);
			let worker_instance_id_key = keys::workflow::WorkerInstanceIdKey::new(workflow_id);
			let silence_ts_key = keys::workflow::SilenceTsKey::new(workflow_id);

			let (
				tags,
				name_entry,
				create_ts_entry,
				input_chunks,
				state_chunks,
				output_chunks,
				error_entry,
				has_wake_condition_entry,
				worker_instance_id_entry,
				silence_ts_entry,
			) = tokio::try_join!(
				tx.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::WantAll,
						..(&tags_subspace).into()
					},
					SNAPSHOT,
				)
				.map(|res| match res {
					Ok(entry) => {
						let key = self
							.subspace
							.unpack::<keys::workflow::TagKey>(entry.key())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
						let v = serde_json::Value::String(key.v.clone());

						Ok((key.k, v))
					}
					Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
				})
				.try_collect::<serde_json::Map<_, _>>(),
				async {
					tx.get(&self.subspace.pack(&name_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
				async {
					tx.get(&self.subspace.pack(&create_ts_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
				async {
					tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&input_subspace).into()
						},
						SNAPSHOT,
					)
					.try_collect::<Vec<_>>()
					.await
					.map_err(Into::into)
				},
				async {
					tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&state_subspace).into()
						},
						SNAPSHOT,
					)
					.try_collect::<Vec<_>>()
					.await
					.map_err(Into::into)
				},
				async {
					tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&output_subspace).into()
						},
						SNAPSHOT,
					)
					.try_collect::<Vec<_>>()
					.await
					.map_err(Into::into)
				},
				async {
					tx.get(&self.subspace.pack(&error_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
				async {
					tx.get(&self.subspace.pack(&has_wake_condition_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
				async {
					tx.get(&self.subspace.pack(&worker_instance_id_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
				async {
					tx.get(&self.subspace.pack(&silence_ts_key), SNAPSHOT)
						.await
						.map_err(Into::into)
				},
			)?;

			let Some(create_ts_entry) = &create_ts_entry else {
				tracing::warn!(?workflow_id, "workflow not found");
				continue;
			};

			let create_ts = create_ts_key
				.deserialize(&create_ts_entry)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let workflow_name = name_key
				.deserialize(&name_entry.ok_or(udb::FdbBindingError::CustomError(
					format!("key should exist: {name_key:?}").into(),
				))?)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let input = input_key
				.combine(input_chunks)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let data = if state_chunks.is_empty() {
				serde_json::value::RawValue::NULL.to_owned()
			} else {
				state_key
					.combine(state_chunks)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			};

			let output = if output_chunks.is_empty() {
				None
			} else {
				Some(
					output_key
						.combine(output_chunks)
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				)
			};

			let error = if let Some(error_entry) = error_entry {
				Some(
					error_key
						.deserialize(&error_entry)
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				)
			} else {
				None
			};

			let state = if silence_ts_entry.is_some() {
				WorkflowState::Silenced
			} else if output.is_some() {
				WorkflowState::Complete
			} else if worker_instance_id_entry.is_some() {
				WorkflowState::Running
			} else if has_wake_condition_entry.is_some() {
				WorkflowState::Sleeping
			} else {
				WorkflowState::Dead
			};

			res.push(WorkflowData {
				workflow_id,
				workflow_name,
				tags: serde_json::Value::Object(tags),
				create_ts,
				input: serde_json::from_str(input.get())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				data: serde_json::from_str(data.get())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				output: output
					.map(|x| serde_json::from_str(x.get()))
					.transpose()
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				error,
				state,
			});
		}

		Ok(res)
	}

	#[tracing::instrument(skip_all)]
	async fn get_signals_inner(
		&self,
		signal_ids: Vec<Id>,
		tx: &udb::RetryableTransaction,
	) -> std::result::Result<Vec<SignalData>, udb::FdbBindingError> {
		let mut res = Vec::new();

		// TODO: Parallelize
		for signal_id in signal_ids {
			let name_key = keys::signal::NameKey::new(signal_id);
			let create_ts_key = keys::signal::CreateTsKey::new(signal_id);
			let body_key = keys::signal::BodyKey::new(signal_id);
			let body_subspace = self.subspace.subspace(&body_key);
			let ack_ts_key = keys::signal::AckTsKey::new(signal_id);
			let silence_ts_key = keys::signal::SilenceTsKey::new(signal_id);
			let workflow_id_key = keys::signal::WorkflowIdKey::new(signal_id);

			let (
				name_entry,
				workflow_id_entry,
				create_ts_entry,
				body_chunks,
				ack_ts_entry,
				silence_ts_entry,
			) = tokio::try_join!(
				tx.get(&self.subspace.pack(&name_key), SNAPSHOT),
				tx.get(&self.subspace.pack(&workflow_id_key), SNAPSHOT),
				tx.get(&self.subspace.pack(&create_ts_key), SNAPSHOT),
				async {
					tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&body_subspace).into()
						},
						SNAPSHOT,
					)
					.try_collect::<Vec<_>>()
					.await
				},
				tx.get(&self.subspace.pack(&ack_ts_key), SNAPSHOT),
				tx.get(&self.subspace.pack(&silence_ts_key), SNAPSHOT),
			)?;

			let Some(create_ts_entry) = &create_ts_entry else {
				tracing::warn!(?signal_id, "signal not found");
				continue;
			};

			let create_ts = create_ts_key
				.deserialize(&create_ts_entry)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let signal_name = name_key
				.deserialize(&name_entry.ok_or(udb::FdbBindingError::CustomError(
					format!("key should exist: {name_key:?}").into(),
				))?)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let body = body_key
				.combine(body_chunks)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let workflow_id = if let Some(workflow_id_entry) = workflow_id_entry {
				Some(
					workflow_id_key
						.deserialize(&workflow_id_entry)
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				)
			} else {
				None
			};

			let ack_ts = if let Some(ack_ts_entry) = ack_ts_entry {
				Some(
					ack_ts_key
						.deserialize(&ack_ts_entry)
						.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				)
			} else {
				None
			};

			let state = if silence_ts_entry.is_some() {
				SignalState::Silenced
			} else if ack_ts.is_some() {
				SignalState::Acked
			} else {
				SignalState::Pending
			};

			res.push(SignalData {
				signal_id,
				signal_name,
				tags: None,
				workflow_id,
				create_ts,
				ack_ts,
				body: serde_json::from_str(body.get())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
				state,
			});
		}

		Ok(res)
	}
}

// NOTE: Most of the reads here are SNAPSHOT because we don't want this to conflict with the actual wf engine.
// Its just for debugging
#[async_trait::async_trait]
impl DatabaseDebug for DatabaseKv {
	#[tracing::instrument(skip_all)]
	async fn get_workflows(&self, workflow_ids: Vec<Id>) -> Result<Vec<WorkflowData>> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let workflow_ids = workflow_ids.clone();
				async move { self.get_workflows_inner(workflow_ids, &tx).await }
			})
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn find_workflows(
		&self,
		tags: &[(String, String)],
		name: Option<&str>,
		state: Option<WorkflowState>,
	) -> Result<Vec<WorkflowData>> {
		// NOTE: this does a full scan of all keys under workflow/data and filters in memory
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let name = name.clone();
				async move {
					let mut workflow_ids = Vec::new();

					let data_subspace = self
						.subspace
						.subspace(&keys::workflow::DataSubspaceKey::new());

					let mut stream = tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::Iterator,
							..(&data_subspace).into()
						},
						SNAPSHOT,
					);

					let mut current_workflow_id = None;
					let mut matching_tags = 0;
					let mut name_matches = name.is_none();
					let mut state_matches = state.is_none() || state == Some(WorkflowState::Dead);

					while let Some(entry) = stream.try_next().await? {
						let workflow_id = *self
							.subspace
							.unpack::<JustId>(entry.key())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						if let Some(curr) = current_workflow_id {
							if workflow_id != curr {
								// Save if matches query
								if matching_tags == tags.len() && name_matches && state_matches {
									workflow_ids.push(curr);

									if workflow_ids.len() >= 100 {
										current_workflow_id = None;
										break;
									}
								}

								// Reset state
								matching_tags = 0;
								name_matches = name.is_none();
								state_matches =
									state.is_none() || state == Some(WorkflowState::Dead);
							}
						}

						current_workflow_id = Some(workflow_id);

						if let Ok(tag_key) =
							self.subspace.unpack::<keys::workflow::TagKey>(entry.key())
						{
							if tags.iter().any(|(k, v)| &tag_key.k == k && &tag_key.v == v) {
								matching_tags += 1;
							}
						} else if let Ok(name_key) =
							self.subspace.unpack::<keys::workflow::NameKey>(entry.key())
						{
							if let Some(name) = &name {
								let workflow_name = name_key
									.deserialize(entry.value())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

								name_matches = &workflow_name == name;
							}
						} else if let Ok(_) = self
							.subspace
							.unpack::<keys::workflow::OutputChunkKey>(entry.key())
						{
							// Has output
							match state {
								Some(WorkflowState::Complete) => state_matches = true,
								Some(_) => state_matches = false,
								None => {}
							}
						} else if let Ok(_) = self
							.subspace
							.unpack::<keys::workflow::WorkerInstanceIdKey>(entry.key())
						{
							match state {
								Some(WorkflowState::Running) => state_matches = true,
								Some(WorkflowState::Sleeping | WorkflowState::Dead) => {
									state_matches = false
								}
								_ => {}
							}
						} else if let Ok(_) = self
							.subspace
							.unpack::<keys::workflow::HasWakeConditionKey>(entry.key())
						{
							match state {
								Some(WorkflowState::Sleeping) => state_matches = true,
								Some(WorkflowState::Dead) => state_matches = false,
								_ => {}
							}
						} else if let Ok(_) = self
							.subspace
							.unpack::<keys::workflow::SilenceTsKey>(entry.key())
						{
							match state {
								Some(WorkflowState::Silenced) => state_matches = true,
								_ => state_matches = false,
							}
						}
					}

					if let (Some(workflow_id), true) = (
						current_workflow_id,
						matching_tags == tags.len() && name_matches && state_matches,
					) {
						workflow_ids.push(workflow_id);
					}

					let workflows = self.get_workflows_inner(workflow_ids, &tx).await?;

					Ok(workflows)
				}
			})
			.instrument(tracing::info_span!("find_workflows_tx"))
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn silence_workflows(&self, workflow_ids: Vec<Id>) -> Result<()> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let workflow_ids = workflow_ids.clone();

				async move {
					// TODO: Parallelize
					for workflow_id in workflow_ids {
						let sub_workflow_wake_subspace = self
							.subspace
							.subspace(&keys::wake::SubWorkflowWakeKey::subspace(workflow_id));
						let tags_subspace = self
							.subspace
							.subspace(&keys::workflow::TagKey::subspace(workflow_id));
						let name_key = keys::workflow::NameKey::new(workflow_id);
						let worker_instance_id_key =
							keys::workflow::WorkerInstanceIdKey::new(workflow_id);
						let output_key = keys::workflow::OutputKey::new(workflow_id);
						let output_subspace = self.subspace.subspace(&output_key);
						let has_wake_condition_key =
							keys::workflow::HasWakeConditionKey::new(workflow_id);
						let silence_ts_key = keys::workflow::SilenceTsKey::new(workflow_id);
						let wake_sub_workflow_key =
							keys::workflow::WakeSubWorkflowKey::new(workflow_id);
						let error_key = keys::workflow::ErrorKey::new(workflow_id);

						let Some(name_entry) =
							tx.get(&self.subspace.pack(&name_key), SERIALIZABLE).await?
						else {
							tracing::warn!(?workflow_id, "workflow not found");
							continue;
						};

						let workflow_name = name_key
							.deserialize(&name_entry)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						let wake_conditions_subspace = self.subspace.subspace(
							&keys::wake::WorkflowWakeConditionKey::subspace_without_ts(
								workflow_name.clone(),
							),
						);

						let (
							sub_workflow_wake_keys,
							tag_keys,
							wake_condition_keys,
							is_running,
							has_output,
							has_wake_condition,
							is_silenced,
							wake_sub_workflow_entry,
							error_entry,
						) = tokio::try_join!(
							// Read sub workflow wake conditions
							tx.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&sub_workflow_wake_subspace).into()
								},
								SERIALIZABLE,
							)
							.map(|res| match res {
								Ok(entry) => self
									.subspace
									.unpack::<keys::wake::SubWorkflowWakeKey>(entry.key())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into())),
								Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
							})
							.try_collect::<Vec<_>>(),
							// Read tags
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
							.try_collect::<Vec<_>>(),
							// Read wake conditions
							tx.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&wake_conditions_subspace).into()
								},
								SNAPSHOT,
							)
							.map(|res| match res {
								Ok(entry) => Ok((
									entry.key().to_vec(),
									self.subspace
										.unpack::<keys::wake::WorkflowWakeConditionKey>(entry.key())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
								)),
								Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
							})
							.try_collect::<Vec<_>>(),
							async {
								tx.get(&self.subspace.pack(&worker_instance_id_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
									.map(|x| x.is_some())
							},
							async {
								tx.get_ranges_keyvalues(
									udb::RangeOption {
										mode: StreamingMode::WantAll,
										limit: Some(1),
										..(&output_subspace).into()
									},
									SNAPSHOT,
								)
								.try_next()
								.await
								.map_err(Into::into)
								.map(|x| x.is_some())
							},
							async {
								tx.get(&self.subspace.pack(&has_wake_condition_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
									.map(|x| x.is_some())
							},
							async {
								tx.get(&self.subspace.pack(&silence_ts_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
									.map(|x| x.is_some())
							},
							async {
								tx.get(&self.subspace.pack(&wake_sub_workflow_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
							},
							async {
								tx.get(&self.subspace.pack(&error_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
							},
						)?;

						if is_silenced {
							continue;
						}

						if is_running {
							return Err(udb::FdbBindingError::CustomError(
								"cannot silence a running workflow".into(),
							));
						}

						for key in sub_workflow_wake_keys {
							tracing::warn!(
								"workflow {} is being waited on by sub workflow {}, silencing anyway",
								key.workflow_id,
								key.sub_workflow_id
							);
						}

						for key in tag_keys {
							let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::new(
								workflow_name.clone(),
								key.k,
								key.v,
								workflow_id,
							);
							tx.clear(&self.subspace.pack(&by_name_and_tag_key));
						}

						// Clear null key
						{
							let by_name_and_tag_key = keys::workflow::ByNameAndTagKey::null(
								workflow_name.clone(),
								workflow_id,
							);
							tx.clear(&self.subspace.pack(&by_name_and_tag_key));
						}

						// Clear wake conditions
						for (raw_key, key) in wake_condition_keys {
							if key.workflow_id != workflow_id {
								continue;
							}

							tx.add_conflict_range(
								&raw_key,
								&end_of_key_range(&raw_key),
								ConflictRangeType::Read,
							)?;

							tx.clear(&raw_key);
						}

						// Clear sub workflow secondary idx
						if let Some(entry) = wake_sub_workflow_entry {
							let sub_workflow_id = wake_sub_workflow_key
								.deserialize(&entry)
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

							let sub_workflow_wake_key =
								keys::wake::SubWorkflowWakeKey::new(sub_workflow_id, workflow_id);

							tx.clear(&self.subspace.pack(&sub_workflow_wake_key));
						}

						// Clear signals secondary index
						let wake_signals_subspace = self
							.subspace
							.subspace(&keys::workflow::WakeSignalKey::subspace(workflow_id));
						tx.clear_subspace_range(&wake_signals_subspace);

						// Clear "has wake condition"
						tx.clear(&self.subspace.pack(&has_wake_condition_key));

						tx.set(
							&self.subspace.pack(&silence_ts_key),
							&silence_ts_key
								.serialize(rivet_util::timestamp::now())
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
						);

						// Clear metric
						let metric = if has_output {
							keys::metric::GaugeMetric::WorkflowComplete(workflow_name.clone())
						} else if has_wake_condition {
							let error = error_key
								.deserialize(&error_entry.ok_or(
									udb::FdbBindingError::CustomError(
										format!("key should exist: {error_key:?}").into(),
									),
								)?)
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

							keys::metric::GaugeMetric::WorkflowDead(workflow_name.clone(), error)
						} else {
							keys::metric::GaugeMetric::WorkflowSleeping(workflow_name.clone())
						};

						update_metric(&tx.subspace(self.subspace.clone()), Some(metric), None);
					}

					Ok(())
				}
			})
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn wake_workflows(&self, workflow_ids: Vec<Id>) -> Result<()> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let workflow_ids = workflow_ids.clone();
				async move {
					let txs = tx.subspace(self.subspace.clone());

					for workflow_id in workflow_ids {
						let name_key = keys::workflow::NameKey::new(workflow_id);
						let worker_instance_id_key =
							keys::workflow::WorkerInstanceIdKey::new(workflow_id);
						let has_wake_condition_key =
							keys::workflow::HasWakeConditionKey::new(workflow_id);
						let error_key = keys::workflow::ErrorKey::new(workflow_id);
						let silence_ts_key = keys::workflow::SilenceTsKey::new(workflow_id);
						let output_key = keys::workflow::OutputKey::new(workflow_id);
						let output_subspace = self.subspace.subspace(&output_key);

						let (
							workflow_name,
							is_running,
							has_wake_condition,
							is_silenced,
							has_output,
							error,
						) = tokio::try_join!(
							txs.read(&name_key, SERIALIZABLE),
							txs.exists(&worker_instance_id_key, SERIALIZABLE),
							txs.exists(&has_wake_condition_key, SERIALIZABLE),
							txs.exists(&silence_ts_key, SERIALIZABLE),
							async {
								tx.get_ranges_keyvalues(
									udb::RangeOption {
										mode: StreamingMode::WantAll,
										limit: Some(1),
										..(&output_subspace).into()
									},
									SNAPSHOT,
								)
								.try_next()
								.await
								.map_err(Into::into)
								.map(|x| x.is_some())
							},
							txs.read_opt(&error_key, SERIALIZABLE),
						)?;

						if is_running || is_silenced {
							continue;
						}

						if has_output {
							return Err(udb::FdbBindingError::CustomError(
								"cannot silence a running workflow".into(),
							));
						}

						txs.write(
							&keys::wake::WorkflowWakeConditionKey::new(
								workflow_name.clone(),
								workflow_id,
								keys::wake::WakeCondition::Immediate,
							),
							(),
						)?;

						txs.write(&has_wake_condition_key, ())?;

						if !has_wake_condition {
							update_metric(
								&txs,
								Some(keys::metric::GaugeMetric::WorkflowDead(
									workflow_name.clone(),
									error.ok_or(udb::FdbBindingError::CustomError(
										format!("key should exist: {error_key:?}").into(),
									))?,
								)),
								Some(keys::metric::GaugeMetric::WorkflowSleeping(workflow_name)),
							);
						}
					}

					Ok(())
				}
			})
			.instrument(tracing::info_span!("wake_workflows_tx"))
			.await?;

		self.wake_worker();

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn get_workflow_history(
		&self,
		workflow_id: Id,
		include_forgotten: bool,
	) -> Result<Option<HistoryData>> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				async move {
					let history_subspace =
						self.subspace
							.subspace(&keys::history::HistorySubspaceKey::new(
								workflow_id,
								if include_forgotten {
									keys::history::HistorySubspaceVariant::All
								} else {
									keys::history::HistorySubspaceVariant::Active
								},
							));

					let (wf, events) = tokio::try_join!(
						async {
							self.get_workflows(vec![workflow_id])
								.await
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
								.map(|wfs| wfs.into_iter().next())
						},
						async {
							let mut events_by_location: HashMap<Location, Vec<Event>> =
								HashMap::new();
							let mut current_event =
								WorkflowHistoryEventBuilder::new(Location::empty(), false);

							let mut stream = tx.get_ranges_keyvalues(
								udb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&history_subspace).into()
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
									.unpack::<keys::history::PartialEventKey>(entry.key())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

								if current_event.location != partial_key.location {
									if current_event.location.is_empty() {
										current_event = WorkflowHistoryEventBuilder::new(
											partial_key.location,
											partial_key.forgotten,
										);
									} else {
										// Insert current event builder to into wf events and
										// reset state
										let previous_event = std::mem::replace(
											&mut current_event,
											WorkflowHistoryEventBuilder::new(
												partial_key.location,
												partial_key.forgotten,
											),
										);
										events_by_location
											.entry(previous_event.location.root())
											.or_default()
											.push(Event::try_from(previous_event).map_err(
												|x| udb::FdbBindingError::CustomError(x.into()),
											)?);
									}
								}

								// Parse current key as any event key
								if let Ok(key) = self
									.subspace
									.unpack::<keys::history::EventTypeKey>(entry.key())
								{
									let event_type = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.event_type = Some(event_type);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::VersionKey>(entry.key())
								{
									let version = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.version = Some(version);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::CreateTsKey>(entry.key())
								{
									let create_ts = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.create_ts = Some(create_ts);
								} else if let Ok(key) =
									self.subspace.unpack::<keys::history::NameKey>(entry.key())
								{
									let name = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.name = Some(name);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::SignalIdKey>(entry.key())
								{
									let signal_id = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.signal_id = Some(signal_id);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::SubWorkflowIdKey>(entry.key())
								{
									let sub_workflow_id = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.sub_workflow_id = Some(sub_workflow_id);
								} else if let Ok(_key) = self
									.subspace
									.unpack::<keys::history::InputChunkKey>(entry.key())
								{
									current_event.input_chunks.push(entry);
								} else if let Ok(_key) = self
									.subspace
									.unpack::<keys::history::OutputChunkKey>(entry.key())
								{
									current_event.output_chunks.push(entry);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::InputHashKey>(entry.key())
								{
									let input_hash = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.input_hash = Some(input_hash);
								} else if let Ok(key) =
									self.subspace.unpack::<keys::history::ErrorKey>(entry.key())
								{
									if let Some(err) = current_event
										.errors
										.iter_mut()
										.find(|err| err.error == key.error)
									{
										err.count += 1;
										err.latest_ts = err.latest_ts.max(key.ts);
									} else {
										current_event.errors.push(ActivityError {
											error: key.error,
											count: 1,
											latest_ts: key.ts,
										});
									}
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::IterationKey>(entry.key())
								{
									let iteration = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.iteration = Some(iteration);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::DeadlineTsKey>(entry.key())
								{
									let deadline_ts = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.deadline_ts = Some(deadline_ts);
								} else if let Ok(key) = self
									.subspace
									.unpack::<keys::history::SleepStateKey>(entry.key())
								{
									let sleep_state = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.sleep_state = Some(sleep_state);
								} else if let Ok(key) =
									self.subspace
										.unpack::<keys::history::InnerEventTypeKey>(entry.key())
								{
									let inner_event_type = key
										.deserialize(entry.value())
										.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

									current_event.inner_event_type = Some(inner_event_type);
								}

								// We ignore keys we don't need (like tags)
							}
							// Insert final event
							if !current_event.location.is_empty() {
								events_by_location
									.entry(current_event.location.root())
									.or_default()
									.push(Event::try_from(current_event).map_err(|x| {
										udb::FdbBindingError::CustomError(x.into())
									})?);
							}

							Ok(events_by_location)
						}
					)?;

					let Some(wf) = wf else {
						return Ok(None);
					};

					let mut flat_events =
						events.into_iter().flat_map(|(_, v)| v).collect::<Vec<_>>();
					flat_events.sort_by(|a, b| a.location.cmp(&b.location));

					Result::<_, udb::FdbBindingError>::Ok(Some(HistoryData {
						wf,
						events: flat_events,
					}))
				}
			})
			.instrument(tracing::info_span!("pull_workflow_history_tx"))
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn get_signals(&self, signal_ids: Vec<Id>) -> Result<Vec<SignalData>> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let signal_ids = signal_ids.clone();
				async move { self.get_signals_inner(signal_ids, &tx).await }
			})
			.instrument(tracing::info_span!("get_signals_tx"))
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn find_signals(
		&self,
		_tags: &[(String, String)],
		workflow_id: Option<Id>,
		name: Option<&str>,
		state: Option<SignalState>,
	) -> Result<Vec<SignalData>> {
		// NOTE: this does a full scan of all keys under signal/data and filters in memory
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let name = name.clone();
				let workflow_id = workflow_id.clone();
				async move {
					let mut signal_ids = Vec::new();

					let data_subspace = self
						.subspace
						.subspace(&keys::signal::DataSubspaceKey::new());

					let mut stream = tx.get_ranges_keyvalues(
						udb::RangeOption {
							mode: StreamingMode::Iterator,
							..(&data_subspace).into()
						},
						SNAPSHOT,
					);

					let mut current_signal_id = None;
					let mut name_matches = name.is_none();
					let mut workflow_id_matches = workflow_id.is_none();
					let mut state_matches = state.is_none() || state == Some(SignalState::Pending);

					while let Some(entry) = stream.try_next().await? {
						let signal_id = *self
							.subspace
							.unpack::<JustId>(entry.key())
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						if let Some(curr) = current_signal_id {
							if signal_id != curr {
								// Save if matches query
								if name_matches && workflow_id_matches && state_matches {
									signal_ids.push(curr);

									if signal_ids.len() >= 100 {
										current_signal_id = None;
										break;
									}
								}

								// Reset state
								name_matches = name.is_none();
								workflow_id_matches = workflow_id.is_none();
								state_matches =
									state.is_none() || state == Some(SignalState::Pending);
							}
						}

						current_signal_id = Some(signal_id);

						if let Ok(name_key) =
							self.subspace.unpack::<keys::signal::NameKey>(entry.key())
						{
							if let Some(name) = &name {
								let signal_name = name_key
									.deserialize(entry.value())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

								name_matches = &signal_name == name;
							}
						} else if let Ok(workflow_id_key) = self
							.subspace
							.unpack::<keys::signal::WorkflowIdKey>(entry.key())
						{
							if let Some(workflow_id) = &workflow_id {
								let signal_workflow_id = workflow_id_key
									.deserialize(entry.value())
									.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

								workflow_id_matches = &signal_workflow_id == workflow_id;
							}
						} else if let Ok(_) =
							self.subspace.unpack::<keys::signal::AckTsKey>(entry.key())
						{
							// Has ack timestamp
							match state {
								Some(SignalState::Acked) => state_matches = true,
								Some(SignalState::Pending) => state_matches = false,
								_ => {}
							}
						} else if let Ok(_) = self
							.subspace
							.unpack::<keys::signal::SilenceTsKey>(entry.key())
						{
							match state {
								Some(SignalState::Silenced) => state_matches = true,
								_ => state_matches = false,
							}
						}
					}

					if let (Some(signal_id), true) = (
						current_signal_id,
						name_matches && workflow_id_matches && state_matches,
					) {
						signal_ids.push(signal_id);
					}

					let signals = self.get_signals_inner(signal_ids, &tx).await?;

					Ok(signals)
				}
			})
			.instrument(tracing::info_span!("find_signals_tx"))
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn silence_signals(&self, signal_ids: Vec<Id>) -> Result<()> {
		self.pools
			.udb()?
			.run(|tx, _mc| {
				let signal_ids = signal_ids.clone();

				async move {
					// TODO: Parallelize
					for signal_id in signal_ids {
						let signal_name_key = keys::signal::NameKey::new(signal_id);
						let create_ts_key = keys::signal::CreateTsKey::new(signal_id);
						let workflow_id_key = keys::signal::WorkflowIdKey::new(signal_id);
						let silence_ts_key = keys::signal::SilenceTsKey::new(signal_id);
						let ack_ts_key = keys::signal::AckTsKey::new(signal_id);

						let (
							signal_name_entry,
							create_ts_entry,
							workflow_id_entry,
							silence_ts_entry,
							ack_ts_entry,
						) = tokio::try_join!(
							tx.get(&self.subspace.pack(&signal_name_key), SERIALIZABLE),
							tx.get(&self.subspace.pack(&create_ts_key), SERIALIZABLE),
							tx.get(&self.subspace.pack(&workflow_id_key), SERIALIZABLE),
							tx.get(&self.subspace.pack(&silence_ts_key), SERIALIZABLE),
							tx.get(&self.subspace.pack(&ack_ts_key), SERIALIZABLE),
						)?;

						if silence_ts_entry.is_some() {
							continue;
						}

						let Some(signal_name_entry) = signal_name_entry else {
							tracing::warn!(?signal_id, "signal not found");
							continue;
						};

						let signal_name = signal_name_key
							.deserialize(&signal_name_entry)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						let create_ts = create_ts_key
							.deserialize(&create_ts_entry.ok_or(
								udb::FdbBindingError::CustomError(
									format!("key should exist: {create_ts_key:?}").into(),
								),
							)?)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						let workflow_id = workflow_id_key
							.deserialize(&workflow_id_entry.ok_or(
								udb::FdbBindingError::CustomError(
									format!("key should exist: {workflow_id_key:?}").into(),
								),
							)?)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						let workflow_name_key = keys::workflow::NameKey::new(workflow_id);

						let workflow_name_entry = tx
							.get(&self.subspace.pack(&workflow_name_key), SERIALIZABLE)
							.await?;

						let workflow_name = workflow_name_key
							.deserialize(&workflow_name_entry.ok_or(
								udb::FdbBindingError::CustomError(
									format!("key should exist: {workflow_name_key:?}").into(),
								),
							)?)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						// Clear pending key
						let mut pending_signal_key = keys::workflow::PendingSignalKey::new(
							workflow_id,
							signal_name.clone(),
							signal_id,
						);
						pending_signal_key.ts = create_ts;
						tx.clear(&self.subspace.pack(&pending_signal_key));

						// Clear wake condition
						let mut wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name,
							workflow_id,
							keys::wake::WakeCondition::Signal { signal_id },
						);
						wake_condition_key.ts = create_ts;
						tx.clear(&self.subspace.pack(&wake_condition_key));

						tx.set(
							&self.subspace.pack(&silence_ts_key),
							&silence_ts_key
								.serialize(rivet_util::timestamp::now())
								.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
						);

						if ack_ts_entry.is_none() {
							update_metric(
								&tx.subspace(self.subspace.clone()),
								Some(keys::metric::GaugeMetric::SignalPending(signal_name)),
								None,
							);
						}
					}

					Ok(())
				}
			})
			.await
			.map_err(Into::into)
	}
}

// Parses Id in third position, ignores the rest
pub(crate) struct JustId(Id);

impl<'de> TupleUnpack<'de> for JustId {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, id)) = <(usize, usize, Id)>::unpack(input, tuple_depth)?;
		let v = JustId(id);

		Ok((&input[0..0], v))
	}
}

impl Deref for JustId {
	type Target = Id;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

struct WorkflowHistoryEventBuilder {
	location: Location,
	event_type: Option<EventType>,
	version: Option<usize>,
	create_ts: Option<i64>,
	forgotten: bool,
	name: Option<String>,
	signal_id: Option<Id>,
	sub_workflow_id: Option<Id>,
	input_chunks: Vec<FdbValue>,
	output_chunks: Vec<FdbValue>,
	tags: Vec<(String, String)>,
	input_hash: Option<Vec<u8>>,
	errors: Vec<ActivityError>,
	iteration: Option<usize>,
	deadline_ts: Option<i64>,
	sleep_state: Option<SleepState>,
	inner_event_type: Option<EventType>,
}

impl WorkflowHistoryEventBuilder {
	fn new(location: Location, forgotten: bool) -> Self {
		WorkflowHistoryEventBuilder {
			location,
			event_type: None,
			version: None,
			create_ts: None,
			forgotten,
			name: None,
			signal_id: None,
			sub_workflow_id: None,
			input_chunks: Vec::new(),
			output_chunks: Vec::new(),
			tags: Vec::new(),
			input_hash: None,
			errors: Vec::new(),
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
			location: value.location.clone(),
			version: value
				.version
				.ok_or(WorkflowError::MissingEventData("version"))?,
			create_ts: value
				.create_ts
				.ok_or(WorkflowError::MissingEventData("create_ts"))?,
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

impl TryFrom<WorkflowHistoryEventBuilder> for ActivityEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(ActivityEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
			input: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location.clone());
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
			output: {
				if value.output_chunks.is_empty() {
					None
				} else {
					// workflow_id not needed
					let output_key =
						keys::history::OutputKey::new(Id::nil(), value.location.clone());
					let v = output_key
						.combine(value.output_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					Some(
						serde_json::from_str(v.get())
							.map_err(Into::into)
							.map_err(WorkflowError::DeserializeEventData)?,
					)
				}
			},
			errors: value.errors,
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for SignalEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(SignalEvent {
			signal_id: value
				.signal_id
				.ok_or(WorkflowError::MissingEventData("signal_id"))?,
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
			body: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location);
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
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
			workflow_id: value.sub_workflow_id,
			tags: None,
			body: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location);
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
		})
	}
}

impl TryFrom<WorkflowHistoryEventBuilder> for MessageSendEvent {
	type Error = WorkflowError;

	fn try_from(value: WorkflowHistoryEventBuilder) -> WorkflowResult<Self> {
		Ok(MessageSendEvent {
			name: value.name.ok_or(WorkflowError::MissingEventData("name"))?,
			tags: serde_json::Value::Object(
				value
					.tags
					.into_iter()
					.map(|(k, v)| (k, serde_json::Value::String(v)))
					.collect(),
			),
			body: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location);
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
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
			tags: serde_json::Value::Object(
				value
					.tags
					.into_iter()
					.map(|(k, v)| (k, serde_json::Value::String(v)))
					.collect(),
			),
			input: {
				if value.input_chunks.is_empty() {
					return Err(WorkflowError::MissingEventData("input"));
				} else {
					// workflow_id not needed
					let input_key = keys::history::InputKey::new(Id::nil(), value.location);
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
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
					let v = input_key
						.combine(value.input_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					serde_json::from_str(v.get())
						.map_err(Into::into)
						.map_err(WorkflowError::DeserializeEventData)?
				}
			},
			output: {
				if value.output_chunks.is_empty() {
					None
				} else {
					// workflow_id not needed
					let output_key =
						keys::history::OutputKey::new(Id::nil(), value.location.clone());
					let v = output_key
						.combine(value.output_chunks)
						.map_err(WorkflowError::DeserializeEventData)?;

					Some(
						serde_json::from_str(v.get())
							.map_err(Into::into)
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
