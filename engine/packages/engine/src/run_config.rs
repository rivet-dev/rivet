use anyhow::{Context, Result, ensure};
use rivet_config::config::TabletBalancingConfig;
use rivet_service_manager::{RunConfigData, Service, ServiceClass};
use rivet_topology::worker::WorkerRole;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::watch;

pub fn config(_rivet_config: rivet_config::Config) -> Result<RunConfigData> {
	let services = vec![
		// TODO: Register topology heartbeat GC once a cluster singleton host exists.
		depot_service(WorkerRole::Storage),
		Service::new(
			"guard",
			ServiceClass::Stateless,
			|_config, ctx| Box::pin(rivet_guard::start(ctx)),
			true,
		),
		// TODO: Restore these services as their legacy dependencies are replaced.
		/*
		Service::new(
			"guard",
			ServiceClass::Stateless,
			|config, context| Box::pin(rivet_guard::start(config, context)),
			true,
		),
		Service::new(
			"bootstrap",
			ServiceClass::Once,
			|config, context| Box::pin(rivet_bootstrap::start(config, context)),
			false,
		),
		*/
		Service::new(
			"api_peer",
			ServiceClass::Always,
			|config, context| crate::peer::serve(config, context, None),
			true,
		),
		// Services required for every workload
		Service::new(
			"cluster_config",
			ServiceClass::Always,
			|config, context| rivet_cluster_config::services::start(config, context),
			false,
		),
		/*
		Service::new(
			"version_management",
			ServiceClass::Always,
			|config, context| Box::pin(rivet_version_management::start(config, context)),
			false,
		),
		Service::new(
			"cache_purge",
			ServiceClass::Always,
			|config, context| Box::pin(rivet_cache_purge::start(config, context)),
			false,
		),
		*/
	];

	Ok(RunConfigData { services })
}

fn depot_service(role: WorkerRole) -> Service {
	Service::new(
		match role {
			WorkerRole::Storage => "depot",
			WorkerRole::Stateless => "depot_membership",
		},
		match role {
			WorkerRole::Storage => ServiceClass::Storage,
			WorkerRole::Stateless => ServiceClass::Always,
		},
		|config, context| async move {
			depot_worker::services::worker::start(
				context.shutdown(),
				config,
				context.ups()?,
				context.topology(),
			)
			.await
		},
		true,
	)
}

fn storage_service(
	directory: Arc<depot_worker::bootstrap::StorageDirectory>,
	epoxy: Option<Arc<crate::epoxy_services::EpoxyServices>>,
	handover: watch::Sender<Option<rivet_singleton_coordinator::runtime::Handover>>,
	coordinator: watch::Sender<Option<depot_worker::services::storage::CoordinatorActive>>,
) -> Service {
	Service::new(
		"depot",
		ServiceClass::Storage,
		move |config, ctx| {
			let directory = directory.clone();
			let epoxy = epoxy.clone();
			let handover = handover.clone();
			let coordinator = coordinator.clone();
			async move {
				let mut commands = depot_worker::command::CommandRegistry::with_core()?;

				epoxy::commands::register(
					&mut commands,
					config.depot().kv_commands.short_idle_close(),
				)?;
				crate::depot_command::kv::register(&mut commands)?;
				pegboard::commands::register(&mut commands, &ctx)?;
				pegboard::index::register(&mut commands, &ctx)?;

				let deps = depot_worker::services::storage::StorageDependencies {
					fork_peer: Some(Arc::new(crate::depot_peer::BaseReader(ctx.clone()))),
					base_reader: Some(Arc::new(crate::depot_peer::BaseReader(ctx.clone()))),
					commands: Arc::new(commands),
					directory,
					pubsub: ctx.ups()?,
					topology: ctx.topology(),
					object_store: ctx.object_stores().primary.clone(),
					lease_object_store: ctx.object_stores().lease.clone(),
					databases: ctx.depot_databases(),
					singleton_services: epoxy.map(|host| {
						host as Arc<dyn rivet_singleton_coordinator::runtime::SingletonServices>
					}),
					filesystem_cache: Some(ctx.filesystem_cache()),
					log_shard_snapshot: None,
					singleton_handover: Some(handover),
					coordinator_active: Some(coordinator),
				};
				// Due actor timers come back to their volumes as `ActorAlarm` commands.
				let alarms = Arc::new(pegboard::commands::ActorAlarmDriver(ctx.clone()));
				depot_worker::services::storage::start_with_driver(
					ctx.shutdown(),
					config,
					deps,
					alarms,
				)
				.await
			}
		},
		true,
	)
}

pub(crate) fn depot_role(services: &[Service]) -> Result<Option<WorkerRole>> {
	let storage = services.iter().any(|service| service.name == "depot");
	let peer = services.iter().any(|service| service.name == "api_peer");
	let singletons = services
		.iter()
		.any(|service| service.class == ServiceClass::ClusterSingleton);

	ensure!(
		!storage || peer,
		"Depot storage requires the API-peer service"
	);
	// The storage owner hosts the tablet coordinator and Epoxy. Other singleton services
	// still need their own exclusive host before they can be selected here.
	ensure!(
		!singletons,
		"cluster singleton services have no host in this build"
	);

	if storage {
		Ok(Some(WorkerRole::Storage))
	} else if peer {
		Ok(Some(WorkerRole::Stateless))
	} else {
		Ok(None)
	}
}

/// Start process services, registering this worker in topology when it participates in Depot.
///
/// `host_metrics` is false for one-shot operator commands, which must not run the hardware
/// reporter or its disk probe.
pub async fn start(
	config: rivet_config::Config,
	context: rivet_engine_context::EngineContext,
	services: Vec<Service>,
	storage: Option<Arc<depot_worker::bootstrap::StorageDirectory>>,
	epoxy: Option<Arc<crate::epoxy_services::EpoxyServices>>,
	host_metrics: bool,
) -> Result<()> {
	let role = depot_role(&services)?;
	if role == Some(WorkerRole::Storage) {
		config
			.depot()
			.validate_storage_shutdown(config.runtime.force_shutdown_duration())?;
	}
	if role.is_some() {
		config.api_peer().validate()?;
	}

	let directory_lock = storage
		.as_ref()
		.and_then(|directory| directory.lock.clone());
	let (handover, handovers) = watch::channel(None);
	let (coordinator, coordinators) = watch::channel(None);
	let mut services = services;
	if let Some(role) = role {
		services.retain(|service| service.name != "depot");
		services.push(match role {
			WorkerRole::Storage => storage_service(
				storage.context("storage service requires an owned log directory")?,
				epoxy,
				handover,
				coordinator,
			),
			WorkerRole::Stateless => depot_service(role),
		});
	}
	// Provisioning asks this worker's own coordinator, so it runs only beside storage.
	if role == Some(WorkerRole::Storage)
		&& let Some(tablets) = config.depot().auto_provision_tablets
	{
		services.push(crate::services::depot_auto_provision::service(
			tablets,
			coordinators,
		));
	}
	if host_metrics {
		services.push(crate::services::host_metrics::service(
			role == Some(WorkerRole::Storage),
		));
	}

	let storage_capacity_millicores = if role == Some(WorkerRole::Storage) {
		if let Some(capacity) = config.runtime.worker_cpu_max {
			u32::try_from(capacity).context("storage CPU allocation exceeds u32 millicores")?
		} else {
			context
				.env()
				.host()
				.cpu()
				.await?
				.logical_cpus
				.checked_mul(TabletBalancingConfig::MILLICORES_PER_CORE)
				.context("storage CPU allocation overflows millicores")?
		}
	} else {
		TabletBalancingConfig::MILLICORES_PER_CORE
	};
	ensure!(
		storage_capacity_millicores > 0,
		"storage CPU allocation must be positive"
	);
	let registration = role.map(|role| rivet_topology::WorkerRegistration {
		storage_capacity_millicores,
		identity: context.worker_identity().clone(),
		role,
		api_peer_address: config.api_peer().advertised_url(),
		build_version: config.build_meta().version.clone(),
		versions: rivet_topology::versions::compiled_versions().into(),
		build_git_sha: config.build_meta().git_sha.clone(),
		started_at_unix_ms: context.topology().started_at_unix_ms(),
		directory_lock: directory_lock.clone(),
	});
	let peer = services
		.iter()
		.position(|service| service.name == "api_peer")
		.map(|index| services.remove(index));
	let ups = context.ups()?;

	// Bind the peer listener before advertising, and keep polling it while
	// registration and the remaining services start.
	let work = async {
		if let Some(peer) = peer {
			let listener = async {
				(peer.run)(config.clone(), context.clone()).await?;
				ensure!(ups.is_draining(), "peer listener stopped before shutdown");
				Ok(())
			};
			let services = async {
				let mut shutdown = context.shutdown();
				rivet_env::select_biased!("do not advertise while shutting down",
					signal = shutdown.recv() => { signal?; return Ok(()); },
					_ = ups.wait_listening() => {},
				);
				rivet_service_manager::start(
					config.clone(),
					context.clone(),
					services,
					registration,
				)
				.await
			};
			futures_util::try_join!(listener, services)?;
			Ok(())
		} else {
			rivet_service_manager::start(config.clone(), context.clone(), services, registration)
				.await
		}
	};

	let (teardown_start, teardown_started) = tokio::sync::oneshot::channel();
	let before_drain = async {
		let permitted = async {
			if role == Some(WorkerRole::Storage) {
				depot_worker::services::stopping::wait_for_permit(
					&config,
					ups.clone(),
					context.topology(),
					context.object_stores().lease.clone(),
					&context.shutdown(),
				)
				.await
			} else {
				Ok(())
			}
		};
		// Hand the singleton tenure over while UPS still serves, so the owner answers
		// coordinator requests until a successor takes over. It starts once the stop permit
		// is settled, so an owner's own drain is decided by one coordinator rather than handed
		// to a successor midway.
		let handover_fits = depot_worker::services::stopping::singleton_handover_reserve_ms(
			&config,
			&context.topology().config().lease,
		)
		.is_some();
		let shutdown = context.shutdown();
		let clock = context.env().clock();
		let singleton = async {
			let handover = handovers.borrow().clone();
			match handover {
				Some(handover) if handover_fits => {
					wait_for_singleton_handover(
						&shutdown,
						clock.as_ref(),
						config.runtime.force_shutdown_duration(),
						config.depot().stop_teardown_ms(),
						handover.step_down(),
					)
					.await
				}
				Some(_) => {
					tracing::info!(
						"force-shutdown budget cannot hold a singleton step-down; storage close releases the tenure"
					);
					Ok(())
				}
				None => Ok(()),
			}
		};
		// Stop readiness may replay closes claimed by this process. Join its clients
		// first so those conditional forgets cannot race accepted appends.
		let prepare = async {
			context.ingress().drain().await;
			prepare_shutdown(context.close_depot_clients(), permitted).await
		};
		let prepared = prepare.await;
		let handed_over = singleton.await;
		if let Err(error) = &handed_over {
			tracing::warn!(
				?error,
				"singleton handover failed; storage close releases the tenure"
			);
		}
		if role == Some(WorkerRole::Storage) {
			let deadline = rivet_env::deadline_after(
				context.env().clock().as_ref(),
				std::time::Duration::from_millis(config.depot().stop_teardown_ms()),
			)?;
			let _ = teardown_start.send(deadline);
		}
		prepared
	};
	let lifecycle = universalpubsub::services::lifecycle::run_with_before_drain(
		ups.clone(),
		context.shutdown(),
		config.runtime.force_shutdown_duration(),
		work,
		before_drain,
	);
	let teardown_watchdog = async {
		let Ok(deadline) = teardown_started.await else {
			return std::future::pending::<Result<()>>().await;
		};
		let clock = context.env().clock();
		let _ = rivet_env::timeout_at(clock.as_ref(), deadline, std::future::pending::<()>()).await;
		tracing::error!("storage teardown budget exhausted; forcing process exit");
		context.env().exit(1).await;
		Ok(())
	};
	rivet_env::select_biased!("completed lifecycle precedes teardown watchdog",
	 result = lifecycle => result,
	 result = teardown_watchdog => result,
	)
}

/// A stop request authorizes replaying this process's volume closes. Only a completed
/// client drain establishes that release; failure takes the ordinary service teardown path.
async fn prepare_shutdown(
	close_clients: impl Future<Output = Result<()>>,
	wait_for_permit: impl Future<Output = Result<()>>,
) -> Result<()> {
	close_clients.await?;
	wait_for_permit.await
}

/// Bound the singleton handover within the original shutdown budget, leaving time for storage
/// teardown even if the coordinator never reports that it stopped.
async fn wait_for_singleton_handover(
	shutdown: &rivet_env::capabilities::shutdown::Shutdown,
	clock: &dyn rivet_env::Clock,
	force_after: Duration,
	teardown_ms: u64,
	handover: impl Future<Output = Result<()>>,
) -> Result<()> {
	let deadline = shutdown.deadline(force_after)?.saturating_sub(teardown_ms);
	rivet_env::timeout_at(clock, deadline, handover).await?
}


