use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use pegboard::{keys, workflows::actor::Allocate};
use rand::{Rng, prelude::SliceRandom, thread_rng};
use rivet_runner_protocol::PROTOCOL_MK2_VERSION;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use universaldb::prelude::*;

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Debug, Serialize, Deserialize)]
struct ActorAllocation {
	actor_id: Id,
	signal: Allocate,
}

#[tokio::test]
async fn test_pending_alloc() -> Result<()> {
	// Setup test environment
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.with_target(false)
		.init();

	let test_id = Uuid::new_v4();
	let dc_label = 1;
	let datacenters = [(
		"test-dc".to_string(),
		rivet_config::config::topology::Datacenter {
			name: "test-dc".to_string(),
			datacenter_label: dc_label,
			is_leader: true,
			peer_url: url::Url::parse("http://127.0.0.1:8080")?,
			public_url: url::Url::parse("http://127.0.0.1:8081")?,
			proxy_url: None,
			valid_hosts: None,
		},
	)]
	.into_iter()
	.collect();

	let api_peer_port = portpicker::pick_unused_port().expect("failed to pick api peer port");
	let guard_port = portpicker::pick_unused_port().expect("failed to pick guard port");

	let test_deps = rivet_test_deps::setup_single_datacenter(
		test_id,
		dc_label,
		datacenters,
		api_peer_port,
		guard_port,
	)
	.await?;
	let test_ctx = TestCtx::new_with_deps(Registry::new(), test_deps)
		.await
		.unwrap();

	tracing::info!("starting pending alloc test");

	let namespace_id = Id::new_v1(dc_label);
	let name = "default";
	let count = 200;

	tracing::info!("populating actors and runners");
	tokio::try_join!(
		populate_actors(&test_ctx, namespace_id, name, count),
		populate_runners(&test_ctx, namespace_id, name, count),
	)
	.unwrap();

	let mut set = JoinSet::new();

	tracing::info!("allocating actors");

	// Runners
	for _ in 0..count {
		let test_ctx = test_ctx.standalone().unwrap();
		let name = name.to_string();

		set.spawn(async move {
			alloc_pending_actors(&test_ctx, namespace_id, &name)
				.await
				.unwrap()
		});
	}

	let mut total_allocated = 0;
	for (allocations, attempted) in set.join_all().await {
		tracing::info!(?attempted, allocated=?allocations.len());
		total_allocated += allocations.len();
	}

	tracing::info!(?total_allocated);

	Ok(())
}

async fn populate_actors(ctx: &TestCtx, namespace_id: Id, name: &str, count: usize) -> Result<()> {
	let dc_label = ctx.config().dc_label();

	ctx.udb()?
		.txn("test_pegboardpending_alloc", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			for _ in 0..count {
				let pending_allocation_ts = util::timestamp::now();
				let actor_id = Id::new_v1(dc_label);

				for replica in 0..10 {
					tx.write(
						&keys::ns::PendingActorByRunnerNameSelectorKey::new(
							namespace_id,
							format!("{name}-{replica}"),
							pending_allocation_ts,
							actor_id,
						),
						0,
					)?;
				}
			}

			Ok(())
		})
		.await
}

async fn populate_runners(ctx: &TestCtx, namespace_id: Id, name: &str, count: usize) -> Result<()> {
	let dc_label = ctx.config().dc_label();

	ctx.udb()?
		.txn("test_pegboardpending_alloc", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			for _ in 0..count {
				let runner_id = Id::new_v1(dc_label);
				let workflow_id = Id::new_v1(dc_label);
				let last_ping_ts = util::timestamp::now();
				let version = 1;
				let remaining_slots = 1;
				let total_slots = 1;

				let remaining_millislots = (remaining_slots * 1000) / total_slots;

				for replica in 0..10 {
					let alloc_key = keys::ns::RunnerAllocIdxKey::new(
						namespace_id,
						format!("{name}-{replica}"),
						version,
						remaining_millislots,
						last_ping_ts,
						runner_id,
					);

					tx.write(
						&alloc_key,
						rivet_data::converted::RunnerAllocIdxKeyData {
							workflow_id,
							remaining_slots,
							total_slots,
							protocol_version: PROTOCOL_MK2_VERSION,
						},
					)?;
				}
			}

			Ok(())
		})
		.await
}

async fn alloc_pending_actors(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	name: &str,
) -> Result<(Vec<ActorAllocation>, usize)> {
	let replica = thread_rng().gen_range(0..10);
	let replica_name = format!("{name}-{replica}");
	let replica_name = &replica_name;

	// First, fetch all of the pending actors with a snapshot read
	let mut pending_actors = ctx
		.udb()?
		.txn("test_pegboardpending_alloc", |tx| async move {
			let start = Instant::now();
			let tx = tx.with_subspace(keys::subspace());

			let pending_actor_subspace = keys::subspace().subspace(
				&keys::ns::PendingActorByRunnerNameSelectorKey::subspace(
					namespace_id,
					replica_name.to_string(),
				),
			);
			let mut queue_stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&pending_actor_subspace).into()
				},
				// NOTE: This is not Serializable because we don't want to conflict with all of the keys, just
				// the one we choose
				Snapshot,
			);

			let mut pending_actors = Vec::new();

			loop {
				if start.elapsed() > EARLY_TXN_TIMEOUT {
					tracing::warn!("timed out reading pending actors queue");
					break;
				}

				let Some(queue_entry) = queue_stream.try_next().await? else {
					break;
				};

				pending_actors.push(
					tx.read_entry::<keys::ns::PendingActorByRunnerNameSelectorKey>(&queue_entry)?,
				);
			}

			Ok(pending_actors)
		})
		.custom_instrument(tracing::info_span!("runner_fetch_pending_actors_tx"))
		.await?;

	// Shuffle for good measure
	pending_actors.shuffle(&mut rand::thread_rng());

	tracing::info!("pausing");
	tokio::time::sleep(Duration::from_secs(3)).await;

	let attempted = pending_actors.len();
	let runner_eligible_threshold = ctx.config().pegboard().runner_eligible_threshold();
	let actor_allocation_candidate_sample_size = ctx
		.config()
		.pegboard()
		.actor_allocation_candidate_sample_size();

	// NOTE: This txn should closely resemble the one found in the allocate_actor activity of the actor wf
	// Split the allocation of each actor into a separate txn. this reduces the scope of each individual txn
	// which reduces conflict rate
	let allocations = futures_util::stream::iter(pending_actors)
		.map(|(queue_key, generation)| async move {
			let queue_key = &queue_key;

			ctx.udb()?
				.txn("test_pegboardpending_alloc", |tx| async move {
					let start = Instant::now();
					let tx = tx.with_subspace(keys::subspace());
					let ping_threshold_ts = util::timestamp::now() - runner_eligible_threshold;

					// Re-check that the queue key still exists in this txn
					if !tx.exists(&queue_key, Snapshot).await? {
						return Ok(None);
					}

					let runner_alloc_subspace =
						keys::subspace().subspace(&keys::ns::RunnerAllocIdxKey::subspace(
							namespace_id,
							replica_name.to_string(),
						));

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::Iterator,
							..(&runner_alloc_subspace).into()
						},
						// NOTE: This is not Serializable because we don't want to conflict with all of the
						// keys, just the one we choose
						Snapshot,
					);

					let mut highest_version = None;
					let mut candidates = Vec::with_capacity(actor_allocation_candidate_sample_size);

					// Select valid runner candidates for allocation
					loop {
						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out allocating pending actors");
							break;
						}

						let Some(entry) = stream.try_next().await? else {
							break;
						};

						let (old_runner_alloc_key, old_runner_alloc_key_data) =
							tx.read_entry::<keys::ns::RunnerAllocIdxKey>(&entry)?;

						if let Some(highest_version) = highest_version {
							// We have passed all of the runners with the highest version. This is reachable if
							// the ping of the highest version runners makes them ineligible
							if old_runner_alloc_key.version < highest_version {
								break;
							}
						} else {
							highest_version = Some(old_runner_alloc_key.version);
						}

						// An empty runner means we have reached the end of the runners with the highest version
						if old_runner_alloc_key.remaining_millislots == 0 {
							break;
						}

						// Ignore runners without valid ping
						if old_runner_alloc_key.last_ping_ts < ping_threshold_ts {
							continue;
						}

						candidates.push((old_runner_alloc_key, old_runner_alloc_key_data));

						// Max candidate size reached
						if candidates.len() >= actor_allocation_candidate_sample_size {
							break;
						}
					}

					// No candidates, allocation cannot be made
					if candidates.is_empty() {
						return Ok(None);
					}

					// Select a candidate at random, weighted by remaining slots
					let (old_runner_alloc_key, old_runner_alloc_key_data) = candidates
						.choose_weighted(&mut rand::thread_rng(), |(key, _)| {
							key.remaining_millislots
						})?;

					for replica in 0..10 {
						// Add read conflict only all replicated runner keys
						let old_runner_name = format!(
							"{}-{replica}",
							old_runner_alloc_key.name.split_once("-").unwrap().0
						);
						let old_runner_alloc_key = keys::ns::RunnerAllocIdxKey::new(
							namespace_id,
							old_runner_name,
							old_runner_alloc_key.version,
							old_runner_alloc_key.remaining_millislots,
							old_runner_alloc_key.last_ping_ts,
							old_runner_alloc_key.runner_id,
						);
						tx.add_conflict_key(&old_runner_alloc_key, ConflictRangeType::Read)?;
						tx.delete(&old_runner_alloc_key);

						// Add read conflict and delete all replicated queue keys
						let runner_name_selector = format!(
							"{}-{replica}",
							queue_key.runner_name_selector.split_once("-").unwrap().0
						);
						let queue_key = keys::ns::PendingActorByRunnerNameSelectorKey::new(
							namespace_id,
							runner_name_selector,
							queue_key.ts,
							queue_key.actor_id,
						);
						tx.add_conflict_key(&queue_key, ConflictRangeType::Read)?;
						tx.delete(&queue_key);
					}

					let new_remaining_slots =
						old_runner_alloc_key_data.remaining_slots.saturating_sub(1);
					let new_remaining_millislots =
						(new_remaining_slots * 1000) / old_runner_alloc_key_data.total_slots;

					// Write new allocation keys with 1 less slot
					for replica in 0..10 {
						tx.write(
							&keys::ns::RunnerAllocIdxKey::new(
								namespace_id,
								format!("{name}-{replica}"),
								old_runner_alloc_key.version,
								new_remaining_millislots,
								old_runner_alloc_key.last_ping_ts,
								old_runner_alloc_key.runner_id,
							),
							rivet_data::converted::RunnerAllocIdxKeyData {
								workflow_id: old_runner_alloc_key_data.workflow_id,
								remaining_slots: new_remaining_slots,
								total_slots: old_runner_alloc_key_data.total_slots,
								protocol_version: old_runner_alloc_key_data.protocol_version,
							},
						)?;
					}

					// Update runner record
					tx.write(
						&keys::runner::RemainingSlotsKey::new(old_runner_alloc_key.runner_id),
						new_remaining_slots,
					)?;

					// Set runner id of actor
					tx.write(
						&keys::actor::RunnerIdKey::new(queue_key.actor_id),
						old_runner_alloc_key.runner_id,
					)?;

					// Insert actor index key
					tx.write(
						&keys::runner::ActorKey::new(
							old_runner_alloc_key.runner_id,
							queue_key.actor_id,
						),
						generation,
					)?;

					return Ok(Some(ActorAllocation {
						actor_id: queue_key.actor_id,
						signal: Allocate {
							runner_id: old_runner_alloc_key.runner_id,
							runner_workflow_id: old_runner_alloc_key_data.workflow_id,
							runner_protocol_version: Some(
								old_runner_alloc_key_data.protocol_version,
							),
						},
					}));
				})
				.custom_instrument(tracing::info_span!("runner_allocate_pending_actors_tx"))
				.await
		})
		.buffer_unordered(1024)
		.filter_map(|res| {
			// Gracefully handle failures because we do not want to fail the entire activity if some
			// allocations were successful
			match res {
				Ok(alloc) => std::future::ready(alloc),
				Err(err) => {
					tracing::error!(?err, "failure during pending actor allocation");

					std::future::ready(None)
				}
			}
		})
		.collect()
		.await;

	Ok((allocations, attempted))
}
