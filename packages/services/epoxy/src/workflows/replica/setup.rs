use anyhow::*;
use epoxy_protocol::protocol;
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use udb_util::prelude::*;
use universaldb::{KeySelector, RangeOption, options::StreamingMode};

use crate::types;

// IMPORTANT: Do not use `read_cluster_config`. Instead, use the config provided by
// `BeginLearningSignal`. This is because the value of `read_cluster_config` may change between
// activities which can cause the learning process to enter an invalid state.

pub async fn setup_replica(ctx: &mut WorkflowCtx, _input: &super::Input) -> Result<()> {
	// Wait for cooridinator to send begin learning signal
	let begin_learning = ctx.listen::<super::BeginLearningSignal>().await?;

	// TODO: Paralellize replicas
	let total_replicas = begin_learning.config.replicas.len();
	let mut replica_index = 0;

	for replica in begin_learning.config.replicas.iter().cloned() {
		// Skip downloading from ourselves
		if replica.replica_id == ctx.config().epoxy_replica_id() {
			continue;
		}

		replica_index += 1;

		#[derive(Serialize, Deserialize)]
		struct State {
			after_instance: Option<types::Instance>,
			/// Total downloaded instances so far.
			total_downloaded_instances: u64,
		}

		// TODO: This should parallelize downloading chunks
		// Track download progress in case downloads fail
		ctx.loope(
			State {
				after_instance: None,
				total_downloaded_instances: 0,
			},
			|ctx, state| {
				let learning_config = begin_learning.config.clone();
				let replica = replica.clone();
				let replica_index = replica_index;
				let total_replicas = total_replicas;
				async move {
					// Download chunk of instances and save them
					let output = ctx
						.activity(DownloadInstancesChunkInput {
							learning_config,
							from_replica_id: replica.replica_id,
							after_instance: state.after_instance.clone(),
							total_downloaded_instances: state.total_downloaded_instances,
							count: crate::consts::DOWNLOAD_INSTANCE_COUNT,
							replica_index,
							total_replicas,
						})
						.await?;

					// Update after_instance for next iteration if we have a last instance
					if let Some(last_instance) = output.last_instance {
						state.after_instance = Some(last_instance);
						state.total_downloaded_instances += 1;
					} else {
						// No more instances
						return Ok(Loop::Break(()));
					}

					Ok(Loop::<()>::Continue)
				}
				.boxed()
			},
		)
		.await?;
	}

	#[derive(Serialize, Deserialize)]
	struct RecoverState {
		after_key: Option<Vec<u8>>,
		total_recovered_keys: u64,
	}

	ctx.loope(
		RecoverState {
			after_key: None,
			total_recovered_keys: 0,
		},
		|ctx, state| {
			let learning_config = begin_learning.config.clone();
			async move {
				// Recover chunk of keys
				let output = ctx
					.activity(RecoverKeysChunkInput {
						learning_config,
						after_key: state.after_key.clone(),
						count: crate::consts::RECOVER_KEY_CHUNK_SIZE,
						total_recovered_keys: state.total_recovered_keys,
					})
					.await?;

				// Update state for next iteration
				if let Some(last_key) = output.last_key {
					state.after_key = Some(last_key);
					state.total_recovered_keys += output.recovered_count;
				} else {
					// No more keys to recover
					tracing::info!(
						total_recovered_keys = state.total_recovered_keys,
						"finished recovering keys"
					);
					return Ok(Loop::Break(()));
				}

				Ok(Loop::<()>::Continue)
			}
			.boxed()
		},
	)
	.await?;

	// Notify coordinator that we're now active
	ctx.activity(NotifyActiveInput {
		learning_config: begin_learning.config.clone(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct DownloadInstancesChunkInput {
	/// Config received from BeginLearningSignal
	pub learning_config: types::ClusterConfig,
	pub from_replica_id: protocol::ReplicaId,
	pub after_instance: Option<types::Instance>,
	pub count: u64,
	pub replica_index: usize,
	pub total_replicas: usize,
	pub total_downloaded_instances: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInstancesChunkOutput {
	/// The last instance in the downloaded chunk.
	///
	/// Used for pagination.
	///
	/// If none, will assume there are no more chunks and stop downloading.
	pub last_instance: Option<types::Instance>,
}

#[activity(DownloadInstancesChunk)]
pub async fn download_instances_chunk(
	ctx: &ActivityCtx,
	input: &DownloadInstancesChunkInput,
) -> Result<DownloadInstancesChunkOutput> {
	let config = &input.learning_config;
	let proto_config: protocol::ClusterConfig = input.learning_config.clone().into();

	tracing::info!(
		from_replica_id = ?input.from_replica_id,
		replica_progress = format!("{}/{}", input.replica_index, input.total_replicas - 1), // -1 to exclude self
		total_downloaded_instances = input.total_downloaded_instances,
		after_instance = ?input.after_instance,
		count = input.count,
		"downloading instances chunk"
	);

	// Send download request to replica
	let request = protocol::Request {
		from_replica_id: config.coordinator_replica_id,
		to_replica_id: input.from_replica_id,
		kind: protocol::RequestKind::DownloadInstancesRequest(protocol::DownloadInstancesRequest {
			after_instance: input.after_instance.clone().map(Into::into),
			count: input.count,
		}),
	};

	let response =
		crate::http_client::send_message(&proto_config, input.from_replica_id, request).await?;

	// Extract instances from response
	let protocol::ResponseKind::DownloadInstancesResponse(download_response) = response.kind else {
		bail!("unexpected response type for download instances request");
	};
	let instances = download_response.instances;

	tracing::info!(instance_count = instances.len(), "received instances");

	// Apply each log entry from the downloaded instances
	let total_entries = instances.len();
	for (idx, entry) in instances.iter().enumerate() {
		tracing::debug!(
			progress = format!("{}/{}", idx + 1, total_entries),
			?entry.instance,
			state = ?entry.log_entry.state,
			"applying log entry"
		);

		// Apply the log entry to replay any uncommitted operations
		apply_log_entry(ctx, &entry.log_entry, &entry.instance).await?;
	}

	// Return whether we should continue downloading chunks and the last instance
	Ok(DownloadInstancesChunkOutput {
		last_instance: instances.last().map(|entry| entry.instance.clone().into()),
	})
}

/// Save log entry to UDB & call the appropriate message handler.
///
/// This function is idempotent since message handlers are executed in the same UDB transaction
/// that the log entry is saved at. If the download or another message handler fails, it's safe to
/// re-execute this.
///
/// This function does NOT commit values to the KV store, even for committed instances.
/// We only save the log entries and update instance states. The actual KV values are recovered
/// later in the `recover_keys_chunk` phase. This is because:
///
/// 1. During download, we receive instances in chunks and may not have all dependencies yet
/// 2. To determine the correct final value, we need to analyze ALL instances and their dependencies
/// 4. Only "leaf" instances (those not depended upon by other commits) with the highest sequence
///    number should have their values applied to KV
///
/// If we committed values during this phase, we might incorrectly apply an older value when
/// a newer one exists but hasn't been downloaded yet, or apply a value that should be superseded
/// by another concurrent operation.
async fn apply_log_entry(
	ctx: &ActivityCtx,
	log_entry: &protocol::LogEntry,
	instance: &protocol::Instance,
) -> Result<()> {
	let replica_id = ctx.config().epoxy_replica_id();

	tracing::info!(
		?instance,
		?log_entry.state,
		"replaying log entry"
	);

	// Replay the log entry
	ctx.udb()?
		.run(move |tx, _| {
			let log_entry = log_entry.clone();
			let instance = instance.clone();

			async move {
				let subspace = crate::keys::subspace(replica_id);
				let log_key =
					crate::keys::replica::LogEntryKey::new(instance.replica_id, instance.slot_id);
				let packed_key = subspace.pack(&log_key);

				// Read existing entry to determine if we need to replay this log entry
				if let Some(bytes) = tx.get(&packed_key, SERIALIZABLE).await? {
					let existing = log_key
						.deserialize(&bytes)
						.map_err(|e| FdbBindingError::CustomError(e.into()))?;

					let existing_order = crate::replica::log::state_order(&existing.state);
					let new_order = crate::replica::log::state_order(&log_entry.state);

					if existing_order >= new_order {
						tracing::debug!(
							?instance,
							?existing.state,
							"existing log entry has a higher state order, will not replay log entry"
						);
						return Result::Ok(());
					}
				}

				// Replay request
				let payload = protocol::Payload {
					proposal: protocol::Proposal {
						commands: log_entry.commands,
					},
					seq: log_entry.seq,
					deps: log_entry.deps,
					instance,
				};
				match log_entry.state {
					protocol::State::PreAccepted => {
						let request = protocol::PreAcceptRequest { payload };
						crate::replica::messages::pre_accept(&*tx, replica_id, request)
							.await
							.map_err(|e| FdbBindingError::CustomError(e.into()))?;
					}
					protocol::State::Accepted => {
						let request = protocol::AcceptRequest { payload };
						crate::replica::messages::accept(&*tx, replica_id, request)
							.await
							.map_err(|e| FdbBindingError::CustomError(e.into()))?;
					}
					protocol::State::Committed => {
						let request = protocol::CommitRequest { payload };
						crate::replica::messages::commit(&*tx, replica_id, request, false)
							.await
							.map_err(|e| FdbBindingError::CustomError(e.into()))?;
					}
				}

				Result::Ok(())
			}
		})
		.await?;

	tracing::info!(
		?instance,
		?log_entry.state,
		"successfully replayed log entry"
	);

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct RecoverKeysChunkInput {
	/// Config received from BeginLearningSignal
	pub learning_config: types::ClusterConfig,
	/// The last key value from the previous chunk, used for pagination
	pub after_key: Option<Vec<u8>>,
	pub count: u64,
	pub total_recovered_keys: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverKeysChunkOutput {
	/// The last key value that was recovered in this chunk.
	///
	/// Used for pagination to skip past all instances of this key in the next iteration.
	///
	/// If none, will assume there are no more keys and stop recovering.
	pub last_key: Option<Vec<u8>>,
	/// Number of keys recovered in this chunk
	pub recovered_count: u64,
}

/// Recovers committed values for a chunk of keys after downloading all log entries.
///
/// This is necessary because during the download phase (`apply_log_entry`), we only
/// downloaded and stored log entries without committing values to KV. We need this
/// separate recovery phase to analyze all dependencies and determine the correct final
/// values.
///
/// The function uses a single-pass scan through the KeyInstance subspace, collecting all
/// instances for each key as it iterates. When it encounters a new key (different from
/// the current one being collected), it immediately processes the previous key's instances
/// and then starts collecting for the new key. This approach avoids redundant database
/// scans compared to the previous implementation that would first identify unique keys
/// then re-scan for each key's instances.
#[activity(RecoverKeysChunk)]
pub async fn recover_keys_chunk(
	ctx: &ActivityCtx,
	input: &RecoverKeysChunkInput,
) -> Result<RecoverKeysChunkOutput> {
	let replica_id = ctx.config().epoxy_replica_id();

	tracing::info!(
		?replica_id,
		total_recovered_keys = input.total_recovered_keys,
		after_key_len = input.after_key.as_ref().map(|k| k.len()),
		count = input.count,
		"recovering keys chunk"
	);

	let (last_key, recovered_count) = ctx
		.udb()?
		.run(move |tx, _| {
			let after_key = input.after_key.clone();
			let count = input.count;

			async move {
				let subspace = crate::keys::subspace(replica_id);

				// Build the key instance prefix to scan
				let key_instance_all =
					crate::keys::replica::KeyInstanceKey::subspace_for_all_keys();
				let prefix = subspace.pack(&key_instance_all);

				// Build range start key - either from after_key or from the beginning
				let begin_key = if let Some(after_key) = &after_key {
					// Skip past all KeyInstance entries for the last processed key.
					let key_instance_subspace =
						crate::keys::replica::KeyInstanceKey::subspace(after_key.clone());
					let mut key_after_all_instances = subspace.pack(&key_instance_subspace);
					// Append 0xFF to get past all instances for this key
					key_after_all_instances.push(0xFF);
					KeySelector::first_greater_or_equal(key_after_all_instances)
				} else {
					KeySelector::first_greater_or_equal(prefix.clone())
				};

				// Build range end key - after all KEY_INSTANCE entries
				let mut end_prefix = prefix.clone();
				end_prefix.push(0xFF);
				let end_key = KeySelector::first_greater_or_equal(end_prefix);

				// Scan for key instances
				let range_option = RangeOption {
					begin: begin_key,
					end: end_key,
					limit: Some(count as usize),
					mode: StreamingMode::WantAll,
					..Default::default()
				};

				let mut stream = tx.get_ranges_keyvalues(range_option, SERIALIZABLE);

				// Iterate over stream and aggregate data for each key
				let mut current_key: Option<Vec<u8>> = None;
				let mut current_instances: Vec<(protocol::ReplicaId, protocol::SlotId)> =
					Vec::new();
				let mut recovered_count = 0u64;
				let mut last_processed_key = None;
				let mut scanned_count = 0u64;

				while let Some(kv) = stream.try_next().await? {
					scanned_count += 1;

					// Parse the key instance entry to extract the key and instance info
					let key_instance = subspace
						.unpack::<crate::keys::replica::KeyInstanceKey>(kv.key())
						.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))?;

					let key = key_instance.key;
					let instance = (
						key_instance.instance_replica_id,
						key_instance.instance_slot_id,
					);

					// Check if this is a new key
					if let Some(ref cur_key) = current_key {
						if cur_key != &key {
							// We've hit a different key - process the current one
							recover_key_value_with_instances(
								&*tx,
								replica_id,
								cur_key,
								&current_instances,
							)
							.await?;
							last_processed_key = Some(cur_key.clone());
							recovered_count += 1;

							// Start collecting for the new key
							current_key = Some(key);
							current_instances = vec![instance];
						} else {
							// Same key, add to instances
							current_instances.push(instance);
						}
					} else {
						// First key
						current_key = Some(key);
						current_instances = vec![instance];
					}
				}

				// Handle the last key if we haven't exceeded our limit
				// If scanned_count < count, we've reached the end of the subspace
				if let Some(ref key) = current_key {
					if scanned_count < count {
						// We've reached the end of the range, process the last key
						recover_key_value_with_instances(&*tx, replica_id, key, &current_instances)
							.await?;
						recovered_count += 1;
						// No more keys to process
						last_processed_key = None;
					} else {
						// We hit the limit before finishing this key
						// Don't update last_processed_key - we want to continue from the last
						// successfully processed key so this unprocessed key will be found again
						// in the next iteration
					}
				}

				// If no keys were processed despite scanning {count} instances,
				// it means a single key has too many instances (i.e. larger than
				// the range limit)
				if recovered_count == 0 && scanned_count >= count {
					return Err(universaldb::FdbBindingError::CustomError(
						anyhow!(
							"single key has more than {} instances, cannot process in one chunk",
							count
						)
						.into(),
					));
				}

				tracing::info!(
					?replica_id,
					recovered_count,
					scanned_count,
					"recovered keys in chunk"
				);

				// Return the last key value for pagination
				Result::Ok((last_processed_key, recovered_count))
			}
		})
		.await?;

	Ok(RecoverKeysChunkOutput {
		last_key,
		recovered_count,
	})
}

/// Performs topological sort on committed entries based on their dependencies using
/// Kahn's algorithm (BFS-based).
///
/// This approach is chosen because:
/// - It naturally detects cycles (which would indicate data corruption in EPaxos)
/// - It handles partial dependency graphs well (not all deps may be in our filtered set)
/// - It processes entries level-by-level in intuitive execution order
/// - The queue-based implementation is clear and maintainable
///
/// Returns entries in topological order where dependencies come before dependents.
/// If a cycle is detected, returns an error.
fn topological_sort_entries(
	entries: &[CommittedEntry],
) -> Result<Vec<&CommittedEntry>, anyhow::Error> {
	if entries.is_empty() {
		return Ok(Vec::new());
	}

	// Build a map for quick lookup
	let entry_map: HashMap<(protocol::ReplicaId, protocol::SlotId), &CommittedEntry> =
		entries.iter().map(|e| (e.instance, e)).collect();

	// Initialize in-degree map and adjacency list
	let mut in_degree: HashMap<(protocol::ReplicaId, protocol::SlotId), usize> = HashMap::new();
	let mut adj_list: HashMap<
		(protocol::ReplicaId, protocol::SlotId),
		Vec<(protocol::ReplicaId, protocol::SlotId)>,
	> = HashMap::new();

	// Initialize all nodes with 0 in-degree
	for entry in entries {
		in_degree.insert(entry.instance, 0);
		adj_list.insert(entry.instance, Vec::new());
	}

	// Build dependency graph
	// Note: In EPaxos, if A depends on B, then B must be executed before A
	for entry in entries {
		for dep in &entry.deps {
			let dep_instance = (dep.replica_id, dep.slot_id);
			// Only count dependencies that are in our committed set
			if entry_map.contains_key(&dep_instance) {
				// dep_instance -> entry.instance (entry depends on dep_instance)
				adj_list
					.get_mut(&dep_instance)
					.unwrap()
					.push(entry.instance);
				*in_degree.get_mut(&entry.instance).unwrap() += 1;
			}
		}
	}

	// Find all nodes with no dependencies
	// Sort them by (replica_id, slot_id) to maintain consistent ordering
	// This ensures that when entries have no dependencies, they're applied
	// in the order they were created (by slot ID)
	let mut initial_nodes: Vec<(protocol::ReplicaId, protocol::SlotId)> = in_degree
		.iter()
		.filter(|(_, degree)| **degree == 0)
		.map(|(&instance, _)| instance)
		.collect();
	initial_nodes.sort(); // Sort by (replica_id, slot_id) tuple
	let mut queue: VecDeque<(protocol::ReplicaId, protocol::SlotId)> = initial_nodes.into();

	let mut sorted_order = Vec::new();

	// Process queue
	while let Some(current) = queue.pop_front() {
		// Add to sorted order
		if let Some(entry) = entry_map.get(&current) {
			sorted_order.push(*entry);
		}

		// Reduce in-degree for dependent nodes
		if let Some(dependents) = adj_list.get(&current) {
			// Collect newly ready nodes
			let mut newly_ready = Vec::new();
			for &dependent in dependents {
				let degree = in_degree.get_mut(&dependent).unwrap();
				*degree -= 1;
				if *degree == 0 {
					newly_ready.push(dependent);
				}
			}
			// Sort and add to queue to maintain consistent ordering
			newly_ready.sort();
			for node in newly_ready {
				queue.push_back(node);
			}
		}
	}

	// Check for cycles
	if sorted_order.len() != entries.len() {
		return Err(anyhow!(
			"Cycle detected in dependency graph: sorted {} entries out of {}",
			sorted_order.len(),
			entries.len()
		));
	}

	Ok(sorted_order)
}

/// Recovers the final committed value for a single key using pre-collected instances.
///
/// This optimized version accepts instances that have already been collected during scanning,
/// avoiding the need to re-scan the KeyInstance subspace.
///
/// The recovery process:
/// 1. Fetches log entries for all provided instances
/// 2. Filters for only committed entries (ignoring pre-accepted or accepted states)
/// 3. Sorts committed entries topologically based on their dependencies
/// 4. Applies commands from each entry in dependency order
///
/// The dependency-aware ordering is crucial: EPaxos allows concurrent operations, and
/// some operations (like check-and-set) depend on seeing the correct previous values.
/// By applying commands in topological order, we ensure that:
/// - Dependencies are satisfied before dependent operations execute
/// - Check-and-set operations see the correct state from previous set operations
/// - The final state is consistent with the consensus protocol's ordering
///
/// This is why we can't commit values during `apply_log_entry` - we need to see all
/// instances and their dependencies first to correctly determine the execution order.
async fn recover_key_value_with_instances(
	tx: &universaldb::Transaction,
	replica_id: protocol::ReplicaId,
	key: &[u8],
	instances: &[(protocol::ReplicaId, protocol::SlotId)],
) -> Result<(), universaldb::FdbBindingError> {
	let subspace = crate::keys::subspace(replica_id);

	tracing::debug!(
		key_len = key.len(),
		instance_count = instances.len(),
		"fetching log entries for key recovery"
	);

	// Fetch log entries in parallel
	let mut committed_entries = Vec::new();
	for chunk in instances.chunks(32) {
		// Fetch log entries
		let mut batch_keys = Vec::with_capacity(chunk.len());
		let mut futures = Vec::with_capacity(batch_keys.len());
		for &(instance_replica_id, instance_slot_id) in chunk {
			let log_key =
				crate::keys::replica::LogEntryKey::new(instance_replica_id, instance_slot_id);
			let packed_key = subspace.pack(&log_key);
			futures.push(tx.get(&packed_key, SERIALIZABLE));
			batch_keys.push((packed_key, log_key, instance_replica_id, instance_slot_id));
		}
		let batch_results = futures_util::future::try_join_all(futures).await?;

		// Process results with their corresponding metadata
		for (bytes, (_, log_key, instance_replica_id, instance_slot_id)) in
			batch_results.into_iter().zip(batch_keys.iter())
		{
			// Missing log entry indicates data corruption
			let bytes = bytes.ok_or_else(|| {
				universaldb::FdbBindingError::CustomError(
					anyhow!(
						"missing log entry for instance ({}, {}), data corruption detected",
						instance_replica_id,
						instance_slot_id
					)
					.into(),
				)
			})?;

			// Collect committed entries
			let entry = log_key
				.deserialize(&bytes)
				.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))?;
			if matches!(entry.state, protocol::State::Committed) {
				committed_entries.push(CommittedEntry {
					instance: (*instance_replica_id, *instance_slot_id),
					entry: entry.clone(),
					seq: entry.seq,
					deps: entry.deps.clone(),
				});
			}
		}
	}

	if committed_entries.is_empty() {
		return Result::Ok(());
	}

	// Sort entries topologically to respect dependencies
	// This ensures that operations are applied in the correct order,
	// particularly important for dependent operations like check-and-set
	let sorted_entries = topological_sort_entries(&committed_entries)
		.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))?;

	tracing::debug!(
		key_len = key.len(),
		sorted_count = sorted_entries.len(),
		"applying commands in dependency order"
	);

	// Apply commands from each entry in dependency order
	// This ensures that check-and-set operations see the correct previous values
	for entry in sorted_entries {
		// Filter commands relevant to this key
		let commands_for_key = entry
			.entry
			.commands
			.iter()
			.filter(|cmd| {
				crate::replica::utils::extract_key_from_command(&cmd).map_or(false, |k| k == key)
			})
			.cloned()
			.collect::<Vec<_>>();

		if !commands_for_key.is_empty() {
			tracing::trace!(
				instance = ?entry.instance,
				command_count = commands_for_key.len(),
				"applying commands from entry"
			);

			// Execute commands
			crate::replica::commit_kv::commit_kv(&*tx, replica_id, &commands_for_key).await?;
		}
	}

	Result::Ok(())
}

#[derive(Debug)]
struct CommittedEntry {
	instance: (protocol::ReplicaId, protocol::SlotId),
	entry: protocol::LogEntry,
	seq: u64, // Seq is u64 in protocol
	deps: Vec<protocol::Instance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct NotifyActiveInput {
	/// Config received from BeginLearningSignal
	pub learning_config: types::ClusterConfig,
}

#[activity(NotifyActive)]
pub async fn notify_active(ctx: &ActivityCtx, input: &NotifyActiveInput) -> Result<()> {
	let config = &input.learning_config;
	let proto_config: protocol::ClusterConfig = config.clone().into();

	tracing::info!("notifying coordinator that replica is active");

	// Send status update to coordinator
	let request = protocol::Request {
		from_replica_id: ctx.config().epoxy_replica_id(),
		to_replica_id: config.coordinator_replica_id,
		kind: protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(
			protocol::CoordinatorUpdateReplicaStatusRequest {
				replica_id: ctx.config().epoxy_replica_id(),
				status: protocol::ReplicaStatus::Active,
			},
		),
	};

	crate::http_client::send_message(&proto_config, config.coordinator_replica_id, request).await?;

	tracing::info!("notified coordinator of active status");
	Ok(())
}
