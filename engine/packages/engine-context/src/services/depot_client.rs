//! Owns the public volume client and the manifest watcher needed by its final lifecycle closes.

use anyhow::Result;
use depot_client::{
	volume_requests::VolumeRequests,
	volumes::{Config as ClientConfig, VolumeClient, Volumes},
};
use object_store::ObjectStore;
use rivet_config::config::depot::DepotConfig;
use rivet_env::{Env, tokio::task::JoinHandle};
use rivet_topology::{FenceReason, Topology};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use universalpubsub::{PubSub, SubjectCapacities};

/// Handle clones do not own service lifetime. Close this owner before UPS begins draining.
pub(crate) struct Service {
	pub client: VolumeClient,
	stop: watch::Sender<bool>,
	joined: Option<JoinHandle<Result<()>>>,
	env: Arc<dyn Env>,
}

/// Starts only for a registered process. Storage and stateless callers share the same routing path.
pub(crate) fn start(
	config: &DepotConfig,
	pubsub: PubSub,
	topology: Arc<Topology>,
	store: Arc<dyn ObjectStore>,
	filesystem_cache: Arc<depot_client::filesystem::CacheBudget>,
) -> Result<Service> {
	let env = pubsub.env().clone();
	let (manifests, view) = watch::channel(None);
	let (refresh, refreshed) = watch::channel(());
	let lease = topology.clone();
	let may_dispatch: Arc<dyn Fn() -> bool + Send + Sync> =
		Arc::new(move || lease.may_admit().unwrap_or(false));
	let capacities = SubjectCapacities {
		incoming: config.volume_request_capacity(),
		outgoing: config.volume_request_capacity(),
	};
	let timeout = Duration::from_millis(config.volume_request_timeout_ms());
	// Every volume client of this process keeps its alarms in one deadline queue. Its driver
	// runs in the service task below until the volumes have shut down.
	let timers = rivet_state_machine_ephemeral::SharedTimers::new();
	let (timers_stop, timers_stopping) = watch::channel(false);
	let lifecycle = VolumeRequests::new(
		pubsub.clone(),
		view,
		refresh,
		capacities,
		timeout,
		may_dispatch.clone(),
	)?
	.with_command_capacity(config.kv_commands.request_capacity())?
	.with_command_limit(
		depot_worker::command::volume_host::Config::from_depot(config).max_command_bytes,
	)
	.with_shared_timers(timers.clone());
	let mut client_config = ClientConfig {
		filesystem: config.filesystem.clone(),
		process_cache: Some(filesystem_cache),
		..Default::default()
	};
	let overrides = &config.volume_client;
	if let Some(value) = overrides.max_loaded_volumes {
		client_config.max_loaded_volumes = value;
	}
	if let Some(value) = overrides.max_requests {
		client_config.max_requests = value;
	}
	if let Some(value) = overrides.max_volume_key_bytes {
		client_config.max_volume_key_bytes = value;
	}
	if let Some(value) = overrides.sweep_interval_ms {
		client_config.sweep_interval_ms = value;
	}
	if let Some(value) = overrides.open_timeout_ms {
		client_config.client.open_timeout_ms = value;
	}
	if let Some(value) = overrides.close_after_idle_ms {
		client_config.close_after_idle_ms = value;
	}
	if let Some(codec) = overrides.payload_codec {
		client_config.client.payload_codec = match codec {
			rivet_config::config::VolumePayloadCodec::None => {
				rivet_data::generated::depot_log_shard_v1::PayloadCodec::None
			}
			rivet_config::config::VolumePayloadCodec::Lz4 => {
				rivet_data::generated::depot_log_shard_v1::PayloadCodec::Lz4
			}
		};
	}
	client_config.log_inflight_per_worker = config.log_client_inflight_per_worker();
	let volumes = Volumes::start(
		pubsub,
		lifecycle,
		SubjectCapacities {
			incoming: config.log_request_capacity(),
			outgoing: config.log_request_capacity(),
		},
		timeout,
		client_config,
		may_dispatch,
	)?;
	let client = volumes.handle();
	let (stop, stopping) = watch::channel(false);
	let (watcher_stop, watcher_stopping) = watch::channel(false);
	let watcher = depot_worker::tablet_supervisor::watch_manifest(
		env.clone(),
		store,
		manifests,
		watcher_stopping,
		vec![refreshed],
		Duration::from_millis(config.manifest_refresh_ms()),
	);

	// One task owns every lifetime, so cancellation of a close waiter cannot strand a service.
	let timers_env = env.clone();
	let joined = rivet_env::tokio::task::spawn(env.as_ref(), async move {
		let services = async move {
			let result = run_services(volumes, watcher, watcher_stop, stopping, topology).await;
			// The volume clients have stopped, so no alarm is left to wake.
			timers_stop.send_replace(true);
			result
		};
		let (result, driven) =
			futures_util::join!(services, timers.run(timers_env, timers_stopping));
		result.and(driven.map_err(anyhow::Error::from))
	});
	Ok(Service {
		client,
		stop,
		joined: Some(joined),
		env,
	})
}

/// Runs the manifest watcher beside the volumes until a stop or a watcher failure, then shuts
/// the volumes down.
async fn run_services(
	volumes: Volumes,
	watcher: impl std::future::Future<Output = Result<()>>,
	watcher_stop: watch::Sender<bool>,
	mut stopping: watch::Receiver<bool>,
	topology: Arc<Topology>,
) -> Result<()> {
	let mut watcher = Box::pin(watcher);
	let finished = rivet_env::select_biased!("stop admission before handling another manifest update",
		_ = stopping.changed() => None,
		result = watcher.as_mut() => Some(result),
	);
	if let Some(result) = finished {
		// An unexpected watcher exit cannot leave cached routing serving indefinitely.
		let _ = topology.fence(FenceReason::DependentFailed).await;
		let closed = volumes.shutdown().await;
		return result.and(closed);
	}

	// Keep refreshing routes until every owned opener has been closed or its lease has expired.
	let close = async {
		let result = volumes.shutdown().await;
		watcher_stop.send_replace(true);
		result
	};
	let (closed, watched) = futures_util::join!(close, watcher.as_mut());
	closed.and(watched)
}

impl Service {
	/// The task remains owned when a caller cancels this wait; a later close can join it again.
	pub async fn close(&mut self) -> Result<()> {
		self.stop.send_replace(true);
		if let Some(joined) = self.joined.as_mut() {
			let result = joined.await;
			self.joined.take();
			result??;
		}
		Ok(())
	}
}

impl Drop for Service {
	fn drop(&mut self) {
		self.stop.send_replace(true);
		if let Some(joined) = self.joined.take() {
			self.env.spawn(Box::pin(async move {
				if !matches!(joined.await, Ok(Ok(()))) {
					tracing::error!("depot client service failed during cleanup");
				}
			}));
		}
	}
}
