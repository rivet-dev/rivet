//! Process ownership for the storage worker's shard, maintainers, and singleton candidate.

use crate::{
	Worker,
	bootstrap::StorageDirectory,
	coordinator_requests::CoordinatorRequests,
	tablet_coordinator_machine::runner::TabletCoordinatorService,
	tablet_maintainer_machine::{executor::cold_store::ColdStoreConfig, types::MaintainerConfig},
	tablet_supervisor::{SupervisorDependencies, TabletSupervisor},
};
use anyhow::{Context, Result, anyhow, ensure};
use depot_log::log_shard_machine::{runner::Shard, snapshot::LogShardSnapshot};
use depot_volume::ups::UpsLogClient;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use object_store::ObjectStore;
use rivet_config::{Config, config::depot::ColdStoreCompression};
use rivet_env::{
	Env,
	capabilities::{shutdown::Shutdown, slatedb::SlateDbFactory},
	tokio::task::JoinHandle,
};
use rivet_singleton_coordinator::{
	authority::Authority,
	runtime::{Coordinator, SingletonServices},
	singleton_coordinator_machine::types::CoordinatorTenure,
	store::ObjectLeaseStore,
};
use rivet_topology::{FenceReason, Topology};
use slatedb::config::CompressionCodec;
use std::{sync::Arc, time::Duration};
use tokio::sync::{oneshot, watch};
use universalpubsub::{PubSub, SubjectCapacities};

pub struct StorageDependencies {
	pub fork_peer: Option<Arc<dyn crate::tablet_maintainer_machine::runner::ForkPeer>>,
	pub base_reader: Option<Arc<dyn crate::filesystem::base::BaseChunkReader>>,
	pub commands: Arc<crate::command::CommandRegistry>,
	/// Dropped when storage services stop, but topology and UPS outlive them, so this is not
	/// the lock's last holder: the process keeps the directory until it can no longer serve
	/// at all, as the engine's `main` does, because a later process treats the released lock
	/// as proof this one exited.
	pub directory: Arc<StorageDirectory>,
	pub pubsub: PubSub,
	pub topology: Arc<Topology>,
	pub object_store: Arc<dyn ObjectStore>,
	/// The same store through a client reserved for lease objects, so a coordinator lease
	/// renewal never waits behind bulk uploads; see `ObjectStores::lease` in engine-context.
	pub lease_object_store: Arc<dyn ObjectStore>,
	pub databases: Arc<dyn SlateDbFactory>,
	/// Application services sharing the tablet coordinator's elected tenure.
	pub singleton_services: Option<Arc<dyn SingletonServices>>,
	/// The process filesystem cache budget, shared with the process volume client.
	pub filesystem_cache: Option<Arc<depot_client::filesystem::CacheBudget>>,
	/// Receives the log shard's snapshot once recovery starts. The shard rejects requests as
	/// `NotServing` until its phase reaches `Serving`, so a caller that must not see that
	/// rejection waits on the snapshot first.
	pub log_shard_snapshot: Option<oneshot::Sender<watch::Receiver<LogShardSnapshot>>>,
	/// Receives the singleton candidate's handover handle once it starts. A graceful shutdown
	/// steps down through it before UPS drains, so the owner keeps answering coordinator
	/// requests until a successor takes over; closing the storage service afterwards finds the
	/// candidate already stopped.
	pub singleton_handover:
		Option<watch::Sender<Option<rivet_singleton_coordinator::runtime::Handover>>>,
	/// Receives a view of this worker's tablet coordinator once its service exists. The view
	/// holds a client while this worker's coordinator admits commands, so a caller that must
	/// not see `NotCoordinator` from its own coordinator waits on it first.
	pub coordinator_active: Option<watch::Sender<Option<CoordinatorActive>>>,
}

/// Holds this worker's tablet coordinator client while it admits commands, and `None` before
/// its election or after its tenure ends.
pub type CoordinatorActive =
	watch::Receiver<Option<crate::tablet_coordinator_machine::runner::Client>>;

/// Keep servicing final client closes, then drain UPS before requesting storage shutdown.
/// The owned task retains the directory lock through cleanup even if this caller is cancelled.
#[tracing::instrument(skip_all)]
pub async fn start(shutdown: Shutdown, config: Config, deps: StorageDependencies) -> Result<()> {
	start_configured(shutdown, config, deps, None).await
}

pub async fn start_with_driver(
	shutdown: Shutdown,
	config: Config,
	deps: StorageDependencies,
	driver: Arc<dyn crate::alarms::DepotDriver>,
) -> Result<()> {
	start_configured(shutdown, config, deps, Some(driver)).await
}

async fn start_configured(
	mut shutdown: Shutdown,
	config: Config,
	deps: StorageDependencies,
	driver: Option<Arc<dyn crate::alarms::DepotDriver>>,
) -> Result<()> {
	config
		.depot()
		.validate_storage_shutdown(config.runtime.force_shutdown_duration())?;
	let env = deps.pubsub.env().clone();
	let pubsub = deps.pubsub.clone();
	let topology = deps.topology.clone();
	let (stop, stopping) = watch::channel(false);
	let task_stop = stop.clone();
	let joined = rivet_env::tokio::task::spawn(env.as_ref(), async move {
		let worker = Worker::start(&config, deps.pubsub.clone(), deps.topology.clone()).await?;
		let result = run(config, deps, task_stop, stopping, driver).await;
		let closed = worker.close().await;
		result.and(closed)
	});
	let mut owner = ServiceTask {
		env,
		topology,
		stop,
		joined: Some(joined),
	};

	let drained = async {
		shutdown.recv().await?;
		pubsub.requests_drained().await;
		Ok::<_, anyhow::Error>(())
	};
	let finished = rivet_env::select_biased!("finish storage cleanup before observing another shutdown signal",
		result = owner.joined.as_mut().context("missing storage task")? => Some(result),
		result = drained => { result?; None },
	);
	let completed = if let Some(result) = finished {
		result
	} else {
		owner.stop.send_replace(true);
		owner.joined.as_mut().context("missing storage task")?.await
	};
	owner.joined.take();
	let result = completed
		.context("storage service task failed")
		.and_then(|result| result);
	if result.is_err() {
		let _ = owner.topology.fence(FenceReason::DependentFailed).await;
	}
	result
}

struct ServiceTask {
	env: Arc<dyn Env>,
	topology: Arc<Topology>,
	stop: watch::Sender<bool>,
	joined: Option<JoinHandle<Result<()>>>,
}

impl Drop for ServiceTask {
	fn drop(&mut self) {
		self.stop.send_replace(true);
		if let Some(joined) = self.joined.take() {
			let topology = self.topology.clone();
			self.env.spawn(Box::pin(async move {
				// Cancellation revokes serving immediately but does not release filesystem ownership.
				let _ = topology.fence(FenceReason::DependentFailed).await;
				if !matches!(joined.await, Ok(Ok(()))) {
					tracing::error!("storage service failed during cleanup");
				}
			}));
		}
	}
}

/// All component futures stay owned until they return; an error stops and joins the siblings.
async fn run(
	config: Config,
	deps: StorageDependencies,
	stop: watch::Sender<bool>,
	mut stopping: watch::Receiver<bool>,
	driver: Option<Arc<dyn crate::alarms::DepotDriver>>,
) -> Result<()> {
	// Validate configuration and construct fenced transports before starting owned tasks.
	let settings = config.depot();
	let env = deps.pubsub.env().clone();
	ensure!(
		deps.directory.identity.worker_id == *deps.pubsub.worker_id()
			&& deps.directory.identity.instance_id.as_bytes()
				== deps.pubsub.instance_id().as_bytes(),
		"storage directory and UPS must share worker identity"
	);
	let root = settings
		.log_directory
		.clone()
		.context("storage services require storage.log_directory")?;
	let mut limits = deps.directory.limits.clone();
	// The shard admits exactly what this worker advertises reading, so it never stores a
	// payload it told writers it cannot read.
	limits.read_versions = deps
		.topology
		.snapshot()?
		.worker_registration
		.as_ref()
		.context("storage services need this worker's topology registration")?
		.versions
		.to_vec();
	let max_grant_batch_volumes = limits.max_forget_batch_volumes;
	ensure!(
		settings.log_request_capacity() > limits.max_memory_requests as usize,
		"storage.log_request_capacity must exceed log_shard.max_memory_requests"
	);
	let log_capacities = SubjectCapacities {
		incoming: settings.log_request_capacity(),
		outgoing: settings.log_request_capacity(),
	};
	let volume_capacities = SubjectCapacities {
		incoming: settings.volume_request_capacity(),
		outgoing: settings.volume_request_capacity(),
	};
	let coordinator_capacities = SubjectCapacities {
		incoming: settings.coordinator_request_capacity(),
		outgoing: settings.coordinator_request_capacity(),
	};
	let timeout = Duration::from_millis(settings.volume_request_timeout_ms());
	let heartbeat_gc = Arc::new(
		super::heartbeat_gc::HeartbeatGcHost::new(
			env.clone(),
			deps.object_store.clone(),
			deps.topology.clone(),
			super::heartbeat_gc::config(&settings.heartbeat_gc),
		)
		.context("depot.heartbeat_gc is invalid")?,
	);
	let gate = deps.topology.clone();
	let replicas = Arc::new(
		UpsLogClient::new(
			deps.pubsub.clone(),
			log_capacities,
			timeout,
			Arc::new(move || gate.may_admit().unwrap_or(false)),
		)?
		.with_lanes(UpsLogClient::lanes(
			&deps.pubsub,
			log_capacities,
			settings.log_client_inflight_per_worker(),
		)?),
	);
	let lease = Arc::new(ObjectLeaseStore::new(deps.lease_object_store.clone()));
	// Routing misses and completed membership reports refresh local assignments immediately.
	let (manifest_refresh, manifest_refreshed) = watch::channel(());
	let reports = Arc::new(
		CoordinatorRequests::new(
			deps.pubsub.clone(),
			deps.topology.clone(),
			lease.clone(),
			coordinator_capacities,
			timeout,
		)?
		.with_manifest_refresh(manifest_refresh.clone()),
	);

	// Start recovery before accepting traffic. The core rejects requests until recovery completes.
	let shard = Arc::new(Shard::start(
		env.clone(),
		root,
		deps.pubsub.worker_id().clone(),
		limits,
	)?);
	if let Some(snapshot) = deps.log_shard_snapshot {
		// The caller may have stopped waiting, which leaves nothing to tell.
		let _ = snapshot.send(shard.snapshot.clone());
	}
	// Replica state changes reach each tablet's maintainer as they sync.
	shard.install_status_notifier(Arc::new(crate::status_notifier::UpsStatusNotifier {
		pubsub: deps.pubsub.clone(),
		capacities: volume_capacities,
		timeout,
	}));
	// The balancer's switch follows the dynamic config; its settings are fixed at startup.
	let (balancing_enabled, balancing_enabled_updates) =
		watch::channel(config.dynamic().depot().balancing.enabled);
	let coordinator_service = Arc::new(
		TabletCoordinatorService::new(env.clone(), deps.object_store.clone())
			.with_balancing(Some(balancing_settings(
				&settings.balancing,
				&deps.topology.config().lease,
			)?))
			.with_balancing_enabled(balancing_enabled_updates)
			.with_learning_deadline_ms(settings.learning_deadline_ms())
			.with_stop_transport(Arc::new(crate::stop_requests::StopRequests {
				replicas: replicas.clone(),
				pubsub: deps.pubsub.clone(),
				topology: deps.topology.clone(),
				// This is a maintainer subject, including when the coordinator calls itself.
				capacities: volume_capacities,
				timeout: Duration::from_millis(settings.stop_timeout_ms()),
			})),
	);
	if let Some(active) = &deps.coordinator_active {
		active.send_replace(Some(coordinator_service.active.subscribe()));
	}
	let singleton = match Coordinator::start(
		env.clone(),
		deps.topology.clone(),
		lease,
		Arc::new(StorageSingletons {
			tablets: coordinator_service.clone(),
			heartbeat_gc,
			application: deps.singleton_services.clone(),
		}),
		// The step-down margin covers the store stamp uncertainty this worker's Topology uses.
		rivet_singleton_coordinator::singleton_coordinator_machine::types::Config::from_topology_timing(
			&deps.topology.config().lease,
		),
	) {
		Ok(singleton) => {
			if let Some(handover) = &deps.singleton_handover {
				handover.send_replace(Some(singleton.handover()));
			}
			singleton
		}
		Err(error) => {
			let shard =
				Arc::try_unwrap(shard).map_err(|_| anyhow!("unstarted shard still shared"))?;
			shard.shutdown().await?;
			return Err(error);
		}
	};
	let checkpoints = depot_fs_core::checkpoints::CheckpointConfig {
		max_named: settings.checkpoints.max_named_checkpoints_per_volume,
		retention: settings.checkpoints.retention.as_ref().map(|p| {
			depot_fs_core::FsRetentionPolicy {
				interval_ms: p.interval_ms,
				count: p.count,
			}
		}),
	};
	// One bulk read budget for the whole process, shared by every tablet's read sessions.
	let bulk_budget = settings
		.bulk_read_bytes_per_sec
		.filter(|rate| *rate > 0)
		.map(|rate| {
			let burst = (rate / 4).max(4 * 1024 * 1024);
			std::sync::Arc::new(crate::filesystem::bulk_budget::BulkReadBudget::new(
				rate, burst,
			))
		});
	if bulk_budget.is_some() && settings.bulk_read_min_chunks.unwrap_or(0) == 0 {
		tracing::warn!(
			"depot.bulk_read_bytes_per_sec has no effect because depot.bulk_read_min_chunks is unset; set it to pace bulk runs"
		);
	}
	crate::metrics::FS_BULK_BUDGET_BYTES_PER_SEC.set(bulk_budget.as_ref().map_or(0, |budget| {
		i64::try_from(budget.bytes_per_sec()).unwrap_or(i64::MAX)
	}));
	let mut supervisor = TabletSupervisor::new(
		deps.pubsub.worker_id().clone(),
		SupervisorDependencies {
			fork_peer: deps.fork_peer.clone(),
			base_reader: deps.base_reader.clone(),
			log_shard: shard.clone(),
			env: env.clone(),
			lease: deps.topology.clone(),
			versions: deps.topology.clone(),
			cold_store: deps.databases.clone(),
			replicas,
			split_reporter: reports,
			holder_watches: deps.topology.clone(),
			cold_config: ColdStoreConfig {
				checkpoints: checkpoints.clone(),
				max_read_sessions: settings
					.max_read_sessions
					.unwrap_or(ColdStoreConfig::default().max_read_sessions),
				read_session_idle_ms: settings
					.read_session_idle_ms
					.unwrap_or(ColdStoreConfig::default().read_session_idle_ms),
				latest_session_idle_ms: settings
					.read_cursor_session_idle_ms
					.unwrap_or(ColdStoreConfig::default().latest_session_idle_ms),
				flush_stage_concurrency: settings
					.flush_stage_concurrency
					.unwrap_or(ColdStoreConfig::default().flush_stage_concurrency),
				volume_lock_stripes: settings
					.store_volume_lock_stripes
					.unwrap_or(ColdStoreConfig::default().volume_lock_stripes),
				bulk_budget: bulk_budget.clone(),
				bulk_run: crate::filesystem::session::BulkRunConfig {
					min_chunks: settings.bulk_read_min_chunks.unwrap_or(0),
					cache_blocks: settings.bulk_read_cache_blocks.unwrap_or(true),
					fetch_tasks: settings.bulk_read_fetch_tasks.unwrap_or(1).max(1),
				},
				volume_locks: match settings.store_volume_locks {
					None => ColdStoreConfig::default().volume_locks,
					Some(rivet_config::config::depot::StoreVolumeLocks::Core) => {
						crate::filesystem::locks::VolumeLockMode::Core
					}
					Some(rivet_config::config::depot::StoreVolumeLocks::Stripes) => {
						crate::filesystem::locks::VolumeLockMode::Stripes
					}
				},
				unflushed_budget_bytes: settings.cold_store.unflushed_budget_bytes(),
				max_concurrent_compactions: settings.cold_store.max_concurrent_compactions(),
				l0_max_ssts: settings.cold_store.l0_max_ssts(),
				compacted_sst_size_bytes: settings.cold_store.compacted_sst_bytes(),
				max_compaction_sources: settings.cold_store.l0_max_ssts(),
				compactor_poll_interval_ms: ColdStoreConfig::default().compactor_poll_interval_ms,
				compression_codec: match settings.cold_store_compression {
					None => ColdStoreConfig::default().compression_codec,
					Some(ColdStoreCompression::None) => None,
					Some(ColdStoreCompression::Lz4) => Some(CompressionCodec::Lz4),
				},
				..Default::default()
			},
			maintainer_config: MaintainerConfig {
				// The shard rejects a batched forget or trim above its own limit, so the
				// maintainer batches up to exactly that limit.
				max_forget_batch_volumes: max_grant_batch_volumes,
				alarms: (&settings.alarms).into(),
				idle_unload_after_ms: settings
					.idle_unload_after_ms
					.unwrap_or(MaintainerConfig::default().idle_unload_after_ms),
				local_region: config.topology().datacenter_label,
				fork_check_grace_ms: settings.forks.check_grace_ms,
				max_concurrent_materializations: settings.forks.max_concurrent_materializations,
				materialize_batch_chunks: settings.forks.materialize_batch_chunks,
				checkpoints,
				write_to_cold_after_ms: settings.write_to_cold_after_ms(),
				write_to_cold_above_bytes: settings
					.write_to_cold_above_bytes
					.unwrap_or(MaintainerConfig::default().write_to_cold_above_bytes),
				flush_piggyback_min_bytes: settings
					.flush_piggyback_min_bytes
					.unwrap_or(MaintainerConfig::default().flush_piggyback_min_bytes),
				memtable_flush_throttle_ms: settings
					.memtable_flush_throttle_ms
					.unwrap_or(MaintainerConfig::default().memtable_flush_throttle_ms),
				max_cold_reads: settings
					.max_cold_reads
					.unwrap_or(MaintainerConfig::default().max_cold_reads),
				max_cold_reads_per_volume: settings
					.max_cold_reads_per_volume
					.unwrap_or(MaintainerConfig::default().max_cold_reads_per_volume),
				max_background_cold_reads: settings
					.max_background_cold_reads
					.unwrap_or(MaintainerConfig::default().max_background_cold_reads),
				simulated_fs_read_cost_us: settings.simulated_fs_read_cost_us.unwrap_or(0),
				max_keyed_in_flight: settings
					.store_max_keyed_in_flight
					.unwrap_or(MaintainerConfig::default().max_keyed_in_flight),
				max_flush_staging: settings
					.store_max_flush_staging
					.unwrap_or(MaintainerConfig::default().max_flush_staging),
				max_flush_uploads: settings
					.store_max_flush_uploads
					.unwrap_or(MaintainerConfig::default().max_flush_uploads),
				inbox_capacity: settings
					.maintainer_inbox_capacity
					.unwrap_or(MaintainerConfig::default().inbox_capacity),
				max_queued_opens: settings
					.maintainer_max_queued_opens
					.unwrap_or(MaintainerConfig::default().max_queued_opens),
				max_requests: settings
					.maintainer_max_requests
					.unwrap_or(MaintainerConfig::default().max_requests),
				max_loaded_volumes: settings
					.maintainer_max_loaded_volumes
					.unwrap_or(MaintainerConfig::default().max_loaded_volumes),
				admission_scan_limit: settings
					.store_admission_scan_limit
					.unwrap_or(MaintainerConfig::default().admission_scan_limit),
				..Default::default()
			},
		},
	);
	if let Some(driver) = driver {
		supervisor = supervisor.with_alarms(driver)?;
	}
	let (manifests, view) = watch::channel(None);
	let manifest = supervisor.watch_manifest(
		deps.object_store.clone(),
		manifests,
		stopping.clone(),
		vec![shard.manifest_refreshes(), manifest_refreshed],
		Duration::from_millis(settings.manifest_refresh_ms()),
	);
	let mut services: FuturesUnordered<BoxFuture<'static, (&'static str, Result<()>)>> =
		FuturesUnordered::new();
	// The log request loop is one serial task for the whole shard: decode, submit and reply.
	let log = rivet_metrics::monitor_task(
		"log_shard_service",
		super::log_shard::start(
			deps.pubsub.clone(),
			deps.topology.clone(),
			shard.clone(),
			log_capacities,
			stopping.clone(),
		),
	);
	services.push(async move { ("log requests", log.await) }.boxed());
	let worker_volumes =
		crate::command::volume_host::OpenVolumeBudget::from_config(&config.depot().kv_commands)?;
	let tablets = super::tablet::start_with_commands(
		deps.pubsub.clone(),
		deps.topology.clone(),
		supervisor,
		view,
		volume_capacities,
		log_capacities,
		stopping.clone(),
		deps.commands.clone(),
		crate::command::volume_host::Config {
			process_cache: deps.filesystem_cache.clone(),
			worker_volumes: Some(worker_volumes),
			manifest_refresh: Some(manifest_refresh),
			..crate::command::volume_host::Config::from_depot(config.depot())
		},
	);
	services.push(async move { ("volume requests", tablets.await) }.boxed());
	let commands = super::coordinator::start(
		deps.pubsub.clone(),
		deps.topology.clone(),
		coordinator_service.active.subscribe(),
		coordinator_capacities,
		stopping.clone(),
	);
	services.push(async move { ("coordinator requests", commands.await) }.boxed());
	services.push(async move { ("manifest watcher", manifest.await) }.boxed());
	let balancing_switch =
		forward_balancing_enabled(config.dynamic_watch(), balancing_enabled, stopping.clone());
	services.push(async move { ("balancing switch", balancing_switch.await) }.boxed());
	tracing::info!("storage services started");

	// Draining subscriptions may finish before the UPS request counter reaches zero.
	// Keep the remaining handlers alive until the caller confirms the complete drain.
	let result = loop {
		if *stopping.borrow() {
			break Ok(());
		}
		rivet_env::select_biased!("stop storage admission before handling another completion",
			changed = stopping.changed() => { if changed.is_err() { break Ok(()); } },
			finished = services.next() => {
				let Some((name, result)) = finished else { break Err(anyhow!("storage services stopped unexpectedly")); };
				if let Err(error) = result { break Err(error.context(name)); }
				if !deps.pubsub.is_draining() || name == "manifest watcher" {
					break Err(anyhow!("{name} stopped before storage shutdown"));
				}
			},
		);
	};

	// Revoke on failure, join maintainers and handlers, release the singleton, then join disk I/O.
	if result.is_err() {
		let _ = deps.topology.fence(FenceReason::DependentFailed).await;
	}
	stop.send_replace(true);
	let mut result = result;
	while let Some((name, closed)) = services.next().await {
		result = result.and(closed.with_context(|| name));
	}
	let singleton_closed = singleton.close().await;
	let shard = Arc::try_unwrap(shard)
		.map_err(|_| anyhow!("storage services retained the shard after shutdown"))?;
	let shard_closed = shard.shutdown().await;
	tracing::info!("storage services stopped");
	drop(deps.directory);
	result.and(singleton_closed).and(shard_closed)
}

/// All hosts share one election. Any one stopping relinquishes the tenure; cleanup joins all.
struct StorageSingletons {
	tablets: Arc<dyn SingletonServices>,
	/// Topology's heartbeat collector, which needs one runner per cluster.
	heartbeat_gc: Arc<dyn SingletonServices>,
	application: Option<Arc<dyn SingletonServices>>,
}

#[async_trait::async_trait]
impl SingletonServices for StorageSingletons {
	async fn run(&self, authority: Authority) -> Result<()> {
		let application = async {
			match &self.application {
				Some(application) => application.run(authority.clone()).await,
				None => std::future::pending().await,
			}
		};
		rivet_env::select_biased!("any singleton host ending relinquishes the shared tenure",
			result = self.tablets.run(authority.clone()) => result,
			result = self.heartbeat_gc.run(authority.clone()) => result,
			result = application => result,
		)
	}

	async fn reconcile_stopped(&self, tenure: &CoordinatorTenure) -> Result<()> {
		// An error from one host must not skip another host's cleanup.
		let application = async {
			match &self.application {
				Some(application) => application.reconcile_stopped(tenure).await,
				None => Ok(()),
			}
		};
		let (tablets, heartbeat_gc, application) = futures_util::join!(
			self.tablets.reconcile_stopped(tenure),
			self.heartbeat_gc.reconcile_stopped(tenure),
			application,
		);
		tablets.and(heartbeat_gc).and(application)
	}
}



/// The coordinator's automatic placement settings, or none when the operator disabled it.
///
/// A new worker must stay listed longer than a dead worker can stay listed: a worker that
/// registers, passes its disk probe and crashes soon after stays in discovery for up to one
/// lease after its last renewal, so a shorter stability window could place tablets on it.
fn balancing_settings(
	config: &rivet_config::config::TabletBalancingConfig,
	lease: &rivet_topology::config::LeaseTiming,
) -> Result<crate::tablet_coordinator_machine::state::balancing::Settings> {
	let listed_after_death_ms = lease
		.ttl_ms
		.saturating_add(lease.renew_interval_ms)
		.saturating_add(lease.zombie_grace_ms);
	ensure!(
		config.destination_stability_ms > listed_after_death_ms,
		"depot.balancing.destination_stability_ms must exceed the topology lease plus one renewal and the takeover grace ({listed_after_death_ms} ms)"
	);
	Ok(
		crate::tablet_coordinator_machine::state::balancing::Settings {
			tablets_per_core: config.tablets_per_core,
			destination_stability_ms: config.destination_stability_ms,
			missing_worker_ms: config.missing_worker_ms,
			max_disk_probes: config.max_disk_probes,
		},
	)
}

/// Publishes the dynamic `depot.balancing.enabled` setting to `enabled` whenever it changes,
/// until storage stops. The coordinator service follows `enabled` in every tenure.
async fn forward_balancing_enabled(
	mut dynamic: watch::Receiver<Arc<rivet_config::config::Root>>,
	enabled: watch::Sender<bool>,
	mut stopping: watch::Receiver<bool>,
) -> Result<()> {
	loop {
		let value = dynamic.borrow_and_update().depot().balancing.enabled;
		enabled.send_if_modified(|current| {
			if *current == value {
				return false;
			}
			*current = value;
			true
		});
		if *stopping.borrow() {
			return Ok(());
		}
		rivet_env::select_biased!("storage shutdown ends the balancing switch before another config change",
			changed = stopping.changed() => { if changed.is_err() { return Ok(()); } },
			changed = dynamic.changed() => { if changed.is_err() { return Ok(()); } },
		);
	}
}
