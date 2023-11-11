use nomad_util::{log_stream, monitor};
use rivet_operation::prelude::*;
use serde::Deserialize;
use std::{collections::HashMap, fmt};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::log_shipper::LogEntry;

type AllocFilter = Box<dyn Fn(&str) -> bool + Send>;

/// Allocation update event received from Nomad.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AllocationUpdated {
	allocation: nomad_client::models::Allocation,
}

/// Registered allocation that has a worker tailing logs from.
pub struct Alloc {
	all_shutdown_tx: Vec<oneshot::Sender<()>>,
}

impl Alloc {
	async fn shutdown(self, alloc_id: &str) -> GlobalResult<()> {
		tracing::info!(?alloc_id, "shutting down alloc");
		for shutdown_tx in self.all_shutdown_tx {
			match shutdown_tx.send(()) {
				Ok(_) => {}
				Err(_) => {
					tracing::error!("alloc shutdown receiver dropped");
				}
			}
		}

		Ok(())
	}
}

/// Monitors allocation status from the Nomad API and starts tailing logs
/// accordingly.
pub struct AllocMonitor {
	nomad_config: nomad_client::apis::configuration::Configuration,
	monitor: monitor::Monitor,
	allocs: HashMap<String, Alloc>,
	shutdown: (broadcast::Sender<()>, broadcast::Receiver<()>),
	log_tx: mpsc::Sender<LogEntry>,

	/// Filters out which allocations to follow.
	alloc_filter: AllocFilter,
}

impl fmt::Debug for AllocMonitor {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("AllocMonitor").finish_non_exhaustive()
	}
}

impl AllocMonitor {
	#[tracing::instrument(skip(log_tx, alloc_filter))]
	pub async fn new(
		nomad_config: nomad_client::apis::configuration::Configuration,
		shutdown: (broadcast::Sender<()>, broadcast::Receiver<()>),
		log_tx: mpsc::Sender<LogEntry>,
		alloc_filter: AllocFilter,
	) -> GlobalResult<Self> {
		// Create monitor
		let nomad_monitor =
			monitor::Monitor::new(nomad_config.clone(), &["Allocation"], None).await?;

		let monitor = AllocMonitor {
			nomad_config,
			monitor: nomad_monitor,
			allocs: HashMap::new(),
			shutdown,
			log_tx,
			alloc_filter,
		};

		Ok(monitor)
	}

	#[tracing::instrument]
	pub async fn follow_existing_allocs(&mut self) -> GlobalResult<()> {
		// Fetch existing allocations before starting
		let all_allocs = nomad_client::apis::allocations_api::get_allocations(
			&self.nomad_config,
			None,
			None,
			None,
			None,
			None,
		)
		.await?;

		for alloc in all_allocs {
			let alloc_id = unwrap_ref!(alloc.ID);
			let eval_id = unwrap_ref!(alloc.eval_id, "alloc has no eval");
			let job_id = unwrap_ref!(alloc.job_id);
			let client_status = unwrap_ref!(alloc.client_status);
			let task_states = if let Some(x) = alloc.task_states.as_ref() {
				x
			} else {
				tracing::info!(
					?job_id,
					?eval_id,
					?alloc_id,
					"missing task states for alloc"
				);
				continue;
			};

			// Ignore alloc that aren't running and don't match filter
			if client_status != "running" {
				continue;
			}
			if !(self.alloc_filter)(job_id) {
				continue;
			}

			self.start_alloc(
				alloc_id.as_str(),
				job_id,
				task_states,
				log_stream::StreamOrigin::End,
			)
			.await?;
		}

		Ok(())
	}

	#[tracing::instrument]
	pub async fn start(mut self) {
		match self.follow_existing_allocs().await {
			Ok(_) => {}
			Err(err) => {
				tracing::error!(?err, "failed to follow existing allocs");
				let _ = self.shutdown.0.send(());
				return;
			}
		}

		// Listen for Nomad alloc events
		loop {
			tokio::select! {
				res = self.monitor.next() => {
					let events = match res {
						Ok(x) => x,
						Err(err) => {
							tracing::error!(?err, "failed to fetch next nomad event, exiting");
							break;
						}
					};

					self.handle_events(events).await;
				}
				res = self.shutdown.1.recv() => {
					tracing::info!("alloc monitor received shutdown");
					if let Err(err) = res {
						tracing::error!(?err, "err in shutdown recv");
					}
					break;
				}
			}
		}

		tracing::info!("shutting down alloc monitor");
		let _ = self.shutdown.0.send(());

		// Shutdown all allocs
		for (alloc_id, alloc) in self.allocs {
			match alloc.shutdown(&alloc_id).await {
				Ok(_) => {}
				Err(err) => {
					tracing::info!(?err, "failed to shut down alloc");
				}
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn handle_events(&mut self, events: Vec<monitor::NomadEvent>) {
		// Handle events
		for event in events {
			if let Some(payload) = event
				.decode::<AllocationUpdated>("Allocation", "AllocationUpdated")
				.unwrap()
			{
				match self.handle_event(&payload).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn handle_event(
		&mut self,
		AllocationUpdated { allocation: alloc }: &AllocationUpdated,
	) -> GlobalResult<()> {
		let alloc_id = unwrap_ref!(alloc.ID);
		let eval_id = unwrap_ref!(alloc.eval_id, "alloc has no eval");
		let job_id = unwrap_ref!(alloc.job_id);
		let client_status = unwrap_ref!(alloc.client_status);

		// Ignore alloc that doesn't match filter
		if !(self.alloc_filter)(job_id) {
			return Ok(());
		}

		tracing::info!(
			?client_status,
			?alloc_id,
			?eval_id,
			?job_id,
			"alloc updated"
		);

		match client_status.as_str() {
			"pending" => {
				// Do nothing

				Ok(())
			}
			"running" => {
				// Check if already running
				if self.allocs.contains_key(alloc_id) {
					tracing::info!(?alloc_id, "alloc already registered, may be caused by a race condition from fetching all allocations and the allocation monitor");
					return Ok(());
				}

				let task_states = unwrap_ref!(alloc.task_states);
				self.start_alloc(
					alloc_id.as_str(),
					job_id.as_str(),
					task_states,
					log_stream::StreamOrigin::Start,
				)
				.await?;

				Ok(())
			}
			"complete" | "failed" | "lost" => {
				// Validate that is registered
				let alloc = if let Some(alloc) = self.allocs.remove(alloc_id) {
					alloc
				} else {
					tracing::info!(?alloc_id, "alloc not registered");
					return Ok(());
				};

				alloc.shutdown(alloc_id).await?;

				Ok(())
			}
			_ => {
				tracing::warn!(?client_status, "unknown alloc client status");
				Ok(())
			}
		}
	}

	#[tracing::instrument(skip(task_states))]
	async fn start_alloc(
		&mut self,
		alloc_id: &str,
		job_id: &str,
		task_states: &HashMap<String, nomad_client::models::TaskState>,
		origin: log_stream::StreamOrigin,
	) -> GlobalResult<()> {
		tracing::info!(?task_states, "starting alloc");

		// Spawn follower for each task
		let mut all_shutdown_tx = Vec::new();
		for (task, task_state) in task_states {
			if !task_state.state.as_ref().map_or(false, |x| x == "running") {
				continue;
			}

			// Start stdout follower
			let (shutdown_tx, shutdown_rx) = oneshot::channel();
			all_shutdown_tx.push(shutdown_tx);
			match crate::task_follower::TaskFollower::new(
				self.nomad_config.clone(),
				alloc_id.to_string(),
				task.to_string(),
				log_stream::StreamType::StdOut,
				origin.clone(),
				self.log_tx.clone(),
				shutdown_rx,
			)
			.await
			{
				Ok(x) => {
					tokio::task::Builder::new()
						.name("nomad_log_follower::alloc_monitor::start_stdout")
						.spawn(x.start())?;
				}
				Err(err) => {
					tracing::error!(?err, ?task, ?task_state, "failed to start stdout follower");
				}
			}

			// Start stderr follower
			let (shutdown_tx, shutdown_rx) = oneshot::channel();
			all_shutdown_tx.push(shutdown_tx);
			match crate::task_follower::TaskFollower::new(
				self.nomad_config.clone(),
				alloc_id.to_string(),
				task.to_string(),
				log_stream::StreamType::StdErr,
				origin.clone(),
				self.log_tx.clone(),
				shutdown_rx,
			)
			.await
			{
				Ok(x) => {
					tokio::task::Builder::new()
						.name("nomad_log_follower::alloc_monitor::start_stderr")
						.spawn(x.start())?;
				}
				Err(err) => {
					tracing::error!(?err, ?task, ?task_state, "failed to start stderr follower");
				}
			}
		}

		self.allocs
			.insert(alloc_id.to_string(), Alloc { all_shutdown_tx });

		Ok(())
	}
}
