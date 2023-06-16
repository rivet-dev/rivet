use nomad_util::log_stream;
use rivet_operation::prelude::*;
use std::{
	fmt,
	time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

use crate::log_shipper::LogEntry;

/// How long to wait before shipping log messages received when tailling the end
/// of a log.
///
/// We use this because Nomad will send a large chunk of the log, even if we set
/// the origin to the log's env. This way, we ignore the backlog and only listen
/// for new logs.
const LOG_ORIGIN_END_GRACE: Duration = Duration::from_secs(1);

/// Follows the logs of a Nomad task in realtime and ships them to the log
/// shipper.
pub struct TaskFollower {
	alloc: String,
	task: String,
	stream_type: log_stream::StreamType,
	stream_origin: log_stream::StreamOrigin,
	log_stream: log_stream::LogStream,
	log_tx: mpsc::Sender<LogEntry>,
	shutdown_rx: oneshot::Receiver<()>,
}

impl fmt::Debug for TaskFollower {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("TaskFollower")
			.field("alloc", &self.alloc)
			.field("task", &self.task)
			.field("stream_type", &self.stream_type)
			.finish_non_exhaustive()
	}
}

impl TaskFollower {
	#[tracing::instrument(skip(log_tx, shutdown_rx))]
	pub async fn new(
		nomad_config: nomad_client::apis::configuration::Configuration,
		alloc: String,
		task: String,
		stream_type: log_stream::StreamType,
		stream_origin: log_stream::StreamOrigin,
		log_tx: mpsc::Sender<LogEntry>,
		shutdown_rx: oneshot::Receiver<()>,
	) -> GlobalResult<Self> {
		tracing::info!("starting task follower");

		let log_stream = log_stream::LogStream::new(
			nomad_config,
			alloc.clone(),
			task.clone(),
			stream_type.clone(),
			stream_origin.clone(),
			None,
		)
		.await?;

		Ok(Self {
			alloc,
			task,
			stream_type,
			stream_origin,
			log_stream,
			log_tx,
			shutdown_rx,
		})
	}

	#[tracing::instrument]
	pub async fn start(mut self) {
		let start = Instant::now();

		let shutdown_rx = self.shutdown_rx;
		tokio::pin!(shutdown_rx);

		loop {
			tokio::select! {
				res = self.log_stream.next() => {
					let events = match res {
						Ok(x) => x,
						Err(err) => {
							tracing::error!(?err, "log stream failed");
							return;
						}
					};

					// Add grace period to ignore log backlogs
					if
						self.stream_origin == log_stream::StreamOrigin::End &&
						Instant::now().duration_since(start) < LOG_ORIGIN_END_GRACE
					{
						continue;
					}

					for event in events {
						match event {
							log_stream::NomadLogEvent::LogLine(log_stream::NomadLogLine { ts, message }) => {
								match self.log_tx.send(LogEntry {
									alloc: self.alloc.clone(),
									task: self.task.clone(),
									stream_type: self.stream_type.clone(),
									ts,
									message,
								}).await {
									Ok(_) => {}
									Err(_) => {
										tracing::error!("failed to send log entry over channel");
									}
								}
							}
							log_stream::NomadLogEvent::Finished => {
								tracing::info!("log finished");
								return;
							}
						}
					}
				}
				_ = &mut shutdown_rx => {
					tracing::info!("task follower shut down");
					break;
				}
			}
		}
	}
}
