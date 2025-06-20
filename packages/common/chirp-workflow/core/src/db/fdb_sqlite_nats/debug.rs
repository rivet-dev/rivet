use anyhow::*;
use fdb_util::{end_of_key_range, FormalChunkedKey, FormalKey, SERIALIZABLE, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
	tuple::{PackResult, TupleDepth, TupleUnpack},
};
use futures_util::{StreamExt, TryStreamExt};
use indoc::indoc;
use rivet_pools::prelude::*;
use std::{
	ops::Deref,
	result::Result::{Err, Ok},
};
use tracing::Instrument;
use uuid::Uuid;

use super::{keys, sqlite::SqlStub, DatabaseFdbSqliteNats};
use crate::{
	db::debug::{
		ActivityError, ActivityEvent, DatabaseDebug, Event, EventData, HistoryData, LoopEvent,
		MessageSendEvent, SignalData, SignalEvent, SignalSendEvent, SignalState, SubWorkflowEvent,
		WorkflowData, WorkflowState,
	},
	history::{
		event::{EventType, RemovedEvent, SleepEvent, SleepState},
		location::Location,
	},
};

// HACK: We alias global error here because its hardcoded into the sql macros
type GlobalError = anyhow::Error;

impl DatabaseFdbSqliteNats {
	#[tracing::instrument(skip_all)]
	async fn get_workflows_inner(
		&self,
		workflow_ids: Vec<Uuid>,
		tx: &fdb::RetryableTransaction,
	) -> std::result::Result<Vec<WorkflowData>, fdb::FdbBindingError> {
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
				output_chunks,
				error_entry,
				has_wake_condition_entry,
				worker_instance_id_entry,
				silence_ts_entry,
			) = tokio::try_join!(
				tx.get_ranges_keyvalues(
					fdb::RangeOption {
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
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
						let v = serde_json::Value::String(key.v.clone());

						Ok((key.k, v))
					}
					Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
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
						fdb::RangeOption {
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
						fdb::RangeOption {
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
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

			let workflow_name = name_key
				.deserialize(&name_entry.ok_or(fdb::FdbBindingError::CustomError(
					format!("key should exist: {name_key:?}").into(),
				))?)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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

			let error = if let Some(error_entry) = error_entry {
				Some(
					error_key
						.deserialize(&error_entry)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
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
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				output: output
					.map(|x| serde_json::from_str(x.get()))
					.transpose()
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				error,
				state,
			});
		}

		Ok(res)
	}

	#[tracing::instrument(skip_all)]
	async fn get_signals_inner(
		&self,
		signal_ids: Vec<Uuid>,
		tx: &fdb::RetryableTransaction,
	) -> std::result::Result<Vec<SignalData>, fdb::FdbBindingError> {
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
						fdb::RangeOption {
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
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

			let signal_name = name_key
				.deserialize(&name_entry.ok_or(fdb::FdbBindingError::CustomError(
					format!("key should exist: {name_key:?}").into(),
				))?)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

			let body = body_key
				.combine(body_chunks)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

			let workflow_id = if let Some(workflow_id_entry) = workflow_id_entry {
				Some(
					workflow_id_key
						.deserialize(&workflow_id_entry)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				)
			} else {
				None
			};

			let ack_ts = if let Some(ack_ts_entry) = ack_ts_entry {
				Some(
					ack_ts_key
						.deserialize(&ack_ts_entry)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
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
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				state,
			});
		}

		Ok(res)
	}
}

// NOTE: Most of the reads here are SNAPSHOT because we don't want this to conflict with the actual wf engine.
// Its just for debugging
#[async_trait::async_trait]
impl DatabaseDebug for DatabaseFdbSqliteNats {
	#[tracing::instrument(skip_all)]
	async fn get_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<Vec<WorkflowData>> {
		self.pools
			.fdb()?
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
			.fdb()?
			.run(|tx, _mc| {
				let name = name.clone();
				async move {
					let mut workflow_ids = Vec::new();

					let data_subspace = self
						.subspace
						.subspace(&keys::workflow::DataSubspaceKey::new());

					let mut stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
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
							.unpack::<JustUuid>(entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
	async fn silence_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<()> {
		self.pools
			.fdb()?
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
						let silence_ts_key = keys::workflow::SilenceTsKey::new(workflow_id);
						let wake_sub_workflow_key =
							keys::workflow::WakeSubWorkflowKey::new(workflow_id);

						let Some(name_entry) =
							tx.get(&self.subspace.pack(&name_key), SERIALIZABLE).await?
						else {
							tracing::warn!(?workflow_id, "workflow not found");
							continue;
						};

						let workflow_name = name_key
							.deserialize(&name_entry)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let wake_conditions_subspace = self.subspace.subspace(
							&keys::wake::WorkflowWakeConditionKey::subspace_without_ts(
								workflow_name.clone(),
							),
						);

						let (
							sub_workflow_wake_keys,
							tag_keys,
							wake_condition_keys,
							worker_instance_id_entry,
							silence_ts_entry,
							wake_sub_workflow_entry,
						) = tokio::try_join!(
							// Read sub workflow wake conditions
							tx.get_ranges_keyvalues(
								fdb::RangeOption {
									mode: StreamingMode::WantAll,
									..(&sub_workflow_wake_subspace).into()
								},
								SERIALIZABLE,
							)
							.map(|res| match res {
								Ok(entry) => self
									.subspace
									.unpack::<keys::wake::SubWorkflowWakeKey>(entry.key())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into())),
								Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
							})
							.try_collect::<Vec<_>>(),
							// Read tags
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
							.try_collect::<Vec<_>>(),
							// Read wake conditions
							tx.get_ranges_keyvalues(
								fdb::RangeOption {
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
										.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
								)),
								Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
							})
							.try_collect::<Vec<_>>(),
							async {
								tx.get(&self.subspace.pack(&worker_instance_id_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
							},
							async {
								tx.get(&self.subspace.pack(&silence_ts_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
							},
							async {
								tx.get(&self.subspace.pack(&wake_sub_workflow_key), SERIALIZABLE)
									.await
									.map_err(Into::into)
							},
						)?;

						if silence_ts_entry.is_some() {
							continue;
						}

						if worker_instance_id_entry.is_some() {
							return Err(fdb::FdbBindingError::CustomError(
								"cannot silence a running workflow with the fdb driver".into(),
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
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
						let has_wake_condition_key =
							keys::workflow::HasWakeConditionKey::new(workflow_id);
						tx.clear(&self.subspace.pack(&has_wake_condition_key));

						tx.set(
							&self.subspace.pack(&silence_ts_key),
							&silence_ts_key
								.serialize(rivet_util::timestamp::now())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					Ok(())
				}
			})
			.await
			.map_err(Into::into)
	}

	#[tracing::instrument(skip_all)]
	async fn wake_workflows(&self, workflow_ids: Vec<Uuid>) -> Result<()> {
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				let workflow_ids = workflow_ids.clone();
				async move {
					for workflow_id in workflow_ids {
						let name_key = keys::workflow::NameKey::new(workflow_id);
						let name_entry = tx
							.get(&self.subspace.pack(&name_key), SNAPSHOT)
							.await
							.map_err(Into::<fdb::FdbBindingError>::into)?;

						let workflow_name = name_key
							.deserialize(&name_entry.ok_or(fdb::FdbBindingError::CustomError(
								format!("key should exist: {name_key:?}").into(),
							))?)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let wake_condition_key = keys::wake::WorkflowWakeConditionKey::new(
							workflow_name,
							workflow_id,
							keys::wake::WakeCondition::Immediate,
						);
						tx.set(
							&self.subspace.pack(&wake_condition_key),
							&wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);

						let has_wake_condition_key =
							keys::workflow::HasWakeConditionKey::new(workflow_id);
						tx.set(
							&self.subspace.pack(&has_wake_condition_key),
							&has_wake_condition_key
								.serialize(())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
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
		workflow_id: Uuid,
		include_forgotten: bool,
	) -> Result<Option<HistoryData>> {
		let pool = &self
			.pools
			.sqlite(crate::db::sqlite_db_name_internal(workflow_id), true)
			.await?;

		let (wf_data, event_rows, error_rows) = tokio::try_join!(
			self.get_workflows(vec![workflow_id]),
			sql_fetch_all!(
				[SqlStub {}, AmalgamEventRow, pool]
				"
				-- Activity events
				SELECT
					json(location) AS location,
					NULL AS tags,
					0 AS event_type,
					version,
					create_ts,
					activity_name AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					json(input) AS input,
					json(output) AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM workflow_activity_events
				WHERE ? OR NOT forgotten
				UNION ALL
				-- Signal listen events
				SELECT
					json(location) AS location,
					NULL AS tags,
					1 AS event_type,
					version,
					create_ts,
					signal_name AS name,
					signal_id AS auxiliary_id,
					NULL AS auxiliary_id2,
					NULL AS input,
					json(body) AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM workflow_signal_events
				WHERE ? OR NOT forgotten
				UNION ALL
				-- Signal send events
				SELECT
					json(location),
					json(tags) AS tags,
					2 AS event_type,
					version,
					create_ts,
					signal_name AS name,
					signal_id AS auxiliary_id,
					workflow_id AS auxiliary_id2,
					json(body) AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM workflow_signal_send_events
				WHERE ? OR NOT forgotten
				UNION ALL
				-- Message send events
				SELECT
					json(location) AS location,
					json(tags) AS tags,
					3 AS event_type,
					version,
					create_ts,
					message_name AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					json(body) AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten				
				FROM workflow_message_send_events
				WHERE ? OR NOT forgotten
				UNION ALL
				-- Sub workflow events
				SELECT
					json(location) AS location,
					COALESCE(json(tags), '{}') AS tags,
					4 AS event_type,
					version,
					create_ts,
					sub_workflow_name AS name,
					sub_workflow_id AS auxiliary_id,
					NULL AS auxiliary_id2,
					json(input) AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM workflow_sub_workflow_events AS sw
				WHERE ? OR NOT forgotten
				UNION ALL
				-- Loop events
				SELECT
					json(location) AS location,
					NULL AS tags,
					5 AS event_type,
					version,
					create_ts,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS auxiliary_id2,
					json(state) AS input,
					json(output) AS output,
					iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM workflow_loop_events
				WHERE ? OR NOT forgotten
				UNION ALL
				SELECT
					json(location),
					NULL AS tags,
					6 AS event_type,
					version,
					create_ts,
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
				FROM workflow_sleep_events
				WHERE ? OR NOT forgotten
				UNION ALL
				SELECT
					json(location) AS location,
					NULL AS tags,
					7 AS event_type,
					version,
					create_ts,
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
				FROM workflow_branch_events
				WHERE ? OR NOT forgotten
				UNION ALL
				SELECT
					json(location) AS location,
					NULL AS tags,
					8 AS event_type,
					1 AS version,
					create_ts,
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
				FROM workflow_removed_events
				WHERE ? OR NOT forgotten
				UNION ALL
				SELECT
					json(location) AS location,
					NULL AS tags,
					9 AS event_type,
					version,
					create_ts,
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
				FROM workflow_version_check_events
				WHERE ? OR NOT forgotten
				ORDER BY location ASC
				",
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
				include_forgotten,
			),
			sql_fetch_all!(
				[SqlStub {}, ActivityErrorRow, pool]
				"
				SELECT
					json(location) AS location,
					error,
					COUNT(error) AS count,
					MAX(ts) AS latest_ts
				FROM workflow_activity_errors
				GROUP BY location, error
				ORDER BY latest_ts
				",
			),
		)?;

		let Some(wf) = wf_data.into_iter().next() else {
			return Ok(None);
		};

		Ok(Some(HistoryData {
			wf,
			events: build_history(event_rows, error_rows)?,
		}))
	}

	#[tracing::instrument(skip_all)]
	async fn get_signals(&self, signal_ids: Vec<Uuid>) -> Result<Vec<SignalData>> {
		self.pools
			.fdb()?
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
		workflow_id: Option<Uuid>,
		name: Option<&str>,
		state: Option<SignalState>,
	) -> Result<Vec<SignalData>> {
		// NOTE: this does a full scan of all keys under signal/data and filters in memory
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				let name = name.clone();
				let workflow_id = workflow_id.clone();
				async move {
					let mut signal_ids = Vec::new();

					let data_subspace = self
						.subspace
						.subspace(&keys::signal::DataSubspaceKey::new());

					let mut stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
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
							.unpack::<JustUuid>(entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

								name_matches = &signal_name == name;
							}
						} else if let Ok(workflow_id_key) = self
							.subspace
							.unpack::<keys::signal::WorkflowIdKey>(entry.key())
						{
							if let Some(workflow_id) = &workflow_id {
								let signal_workflow_id = workflow_id_key
									.deserialize(entry.value())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
	async fn silence_signals(&self, signal_ids: Vec<Uuid>) -> Result<()> {
		self.pools
			.fdb()?
			.run(|tx, _mc| {
				let signal_ids = signal_ids.clone();

				async move {
					// TODO: Parallelize
					for signal_id in signal_ids {
						let signal_name_key = keys::signal::NameKey::new(signal_id);
						let create_ts_key = keys::signal::CreateTsKey::new(signal_id);
						let workflow_id_key = keys::signal::WorkflowIdKey::new(signal_id);
						let silence_ts_key = keys::signal::SilenceTsKey::new(signal_id);

						let (
							signal_name_entry,
							create_ts_entry,
							workflow_id_entry,
							silence_ts_entry,
						) = tokio::try_join!(
							tx.get(&self.subspace.pack(&signal_name_key), SNAPSHOT),
							tx.get(&self.subspace.pack(&create_ts_key), SNAPSHOT),
							tx.get(&self.subspace.pack(&workflow_id_key), SNAPSHOT),
							tx.get(&self.subspace.pack(&silence_ts_key), SERIALIZABLE),
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
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let create_ts = create_ts_key
							.deserialize(&create_ts_entry.ok_or(
								fdb::FdbBindingError::CustomError(
									format!("key should exist: {create_ts_key:?}").into(),
								),
							)?)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let workflow_id = workflow_id_key
							.deserialize(&workflow_id_entry.ok_or(
								fdb::FdbBindingError::CustomError(
									format!("key should exist: {workflow_id_key:?}").into(),
								),
							)?)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						let workflow_name_key = keys::workflow::NameKey::new(workflow_id);

						let workflow_name_entry = tx
							.get(&self.subspace.pack(&workflow_name_key), SNAPSHOT)
							.await?;

						let workflow_name = workflow_name_key
							.deserialize(&workflow_name_entry.ok_or(
								fdb::FdbBindingError::CustomError(
									format!("key should exist: {workflow_name_key:?}").into(),
								),
							)?)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

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
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}

					Ok(())
				}
			})
			.await
			.map_err(Into::into)
	}
}

// Parses UUID in third position, ignores the rest
pub(crate) struct JustUuid(Uuid);

impl<'de> TupleUnpack<'de> for JustUuid {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, id)) = <(usize, usize, Uuid)>::unpack(input, tuple_depth)?;
		let v = JustUuid(id);

		Ok((&input[0..0], v))
	}
}

impl Deref for JustUuid {
	type Target = Uuid;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(sqlx::FromRow)]
struct ActivityErrorRow {
	location: Location,
	error: String,
	count: i64,
	latest_ts: i64,
}

#[derive(sqlx::FromRow)]
struct AmalgamEventRow {
	location: Location,
	tags: Option<serde_json::Value>,
	version: i64,
	create_ts: i64,
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
		let event_type = value.event_type.try_into().context("integer conversion")?;
		let event_type = EventType::from_repr(event_type)
			.with_context(|| format!("invalid event type: {}", value.event_type))?;

		Ok(Event {
			location: value.location.clone(),
			version: value.version.try_into().context("integer conversion")?,
			create_ts: value.create_ts,
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
					.filter(|row| row.location == event.location)
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

	Ok(events)
}
