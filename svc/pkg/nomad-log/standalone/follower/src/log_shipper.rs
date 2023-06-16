use nomad_util::log_stream::StreamType;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use std::{collections::HashMap, fmt, time::Duration};
use tokio::sync::{broadcast, mpsc};

const SHIP_LOG_INTERVAL: Duration = Duration::from_secs(2);

// TODO: Update this to optimize for ClickHouse
/// 8 KB maximum s4ze for log entries.
///
/// This value is also limited by the message size we can write to NATS & Redis.
/// When we add support for Kafka, this will be able to be a much larger value.
///
/// **Antiquated reasoning for this value**
///
/// Hard cap for batch size is 16 MB. Recommended to keep it below 1 MB.
/// https://www.scylladb.com/2019/03/27/best-practices-for-scylla-applications/#:~:text=There%20is%20a%20hard%20limit,database%20at%20any%20particular%20time.
const LOG_ENTRIES_BATCH_MESSAGE_MAX_LEN: usize = 8 * 1024;

/// Log entry shipped from a `TaskFollower`.
pub struct LogEntry {
	pub alloc: String,
	pub task: String,
	pub stream_type: StreamType,
	pub ts: i64,
	pub message: Vec<u8>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct LogEntryKey {
	alloc: String,
	task: String,
	stream_type: StreamType,
}

/// Log entry stored in the buffer.
///
/// The alloc ID is stored as the key in the `log_entires_buffer` map.
struct MinimalLogEntry {
	ts: i64,
	message: Vec<u8>,
}

/// Receives incoming logs and ships them to the message channel.
pub struct LogShipper {
	client: chirp_client::Client,
	shutdown: (broadcast::Sender<()>, broadcast::Receiver<()>),
	log_rx: mpsc::Receiver<LogEntry>,

	/// Backlog of all logs received since the last time `ship_log_buffer` was
	/// called.
	///
	/// We only ship these logs every `SHIP_LOG_INTERVAL` in order to batch logs
	/// together from noisy producers.
	log_entires_buffer: HashMap<LogEntryKey, Vec<MinimalLogEntry>>,
}

impl fmt::Debug for LogShipper {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("LogShipper").finish_non_exhaustive()
	}
}

impl LogShipper {
	pub fn new(
		client: chirp_client::Client,
		shutdown: (broadcast::Sender<()>, broadcast::Receiver<()>),
		log_rx: mpsc::Receiver<LogEntry>,
	) -> Self {
		LogShipper {
			client,
			shutdown,
			log_rx,
			log_entires_buffer: HashMap::new(),
		}
	}

	#[tracing::instrument]
	pub async fn start(mut self) {
		let mut ship_timer = tokio::time::interval(SHIP_LOG_INTERVAL);
		loop {
			tokio::select! {
				_ = ship_timer.tick() => {
					match self.ship_log_buffer().await {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to ship logs");
							break;
						}
					}
				}
				entry = self.log_rx.recv() => {
					if let Some(entry) = entry {
						self.write_log(entry);
					} else {
						tracing::info!("log receiver dropped, exiting loop");
						break;
					}
				}
				res = self.shutdown.1.recv() => {
					tracing::info!("log shipper received shutdown");
					if let Err(err) = res {
						tracing::error!(?err, "err in shutdown recv");
					}

					break;
				}
			}
		}

		tracing::info!("log shipper shutting down");
		let _ = self.shutdown.0.send(());
	}

	/// Drains the log buffer and dispatches them as Chirp messages.
	#[tracing::instrument]
	async fn ship_log_buffer(&mut self) -> GlobalResult<()> {
		// Ship all logs as messages
		let mut total_task_count = 0usize;
		let mut total_log_entries = 0usize;
		let mut total_entries_message_len = 0usize;

		for (key, entries) in self.log_entires_buffer.drain() {
			total_task_count += 1;
			total_log_entries += entries.len();

			// Ship the entries in batches with the total message size <
			// LOG_ENTRIES_BATCH_MESSAGE_MAX_LEN. The entries get batched
			// together with a max data len in order to ensure that we aren't
			// sending really large messages.
			let entries_len = entries.len();
			let mut batch_message_len = 0usize;
			let mut entries_batch = Vec::new();
			let mut last_msg_ts = 0i64;
			let mut last_msg_idx = 0u32;
			for (i, entry) in entries.into_iter().enumerate() {
				if entry.ts < last_msg_ts {
					tracing::warn!("time went backwards, entry timestamp is less than previous");
				}

				// Determine the entry index. We do this in order to be able to
				// reliably sort messages in the order they were logged.
				if entry.ts > last_msg_ts {
					last_msg_idx = 0;
				} else {
					last_msg_idx += 1;
				}
				last_msg_ts = entry.ts;
				let idx = last_msg_idx;

				// Add the entry to the batch
				batch_message_len += entry.message.len();
				total_entries_message_len += entry.message.len();
				entries_batch.push(backend::nomad_log::LogEntry {
					ts: entry.ts,
					idx,
					message: entry.message,
				});

				// Ship entries if the last item in the list or larger than
				// the max batch size
				if batch_message_len > LOG_ENTRIES_BATCH_MESSAGE_MAX_LEN || i == entries_len - 1 {
					batch_message_len = 0;
					msg!([self.client] @dont_log_body nomad_log::msg::entries(&key.alloc, &key.task, key.stream_type.as_str()) {
						alloc: key.alloc.clone(),
						task: key.task.clone(),
						stream_type: match key.stream_type {
							StreamType::StdOut => backend::nomad_log::StreamType::StdOut as i32,
							StreamType::StdErr => backend::nomad_log::StreamType::StdErr as i32,
						},
						entries: std::mem::take(&mut entries_batch),
					})
					.await?;
				}
			}
		}

		tracing::info!(
			?total_task_count,
			?total_log_entries,
			?total_entries_message_len,
			"shipped log entries"
		);

		Ok(())
	}

	/// Writes a new log entry to the buffer.
	#[tracing::instrument(skip(entry))]
	fn write_log(&mut self, entry: LogEntry) {
		// TODO: Adding and removing values from a HashMap like this is is very
		// allocation-heavy

		tracing::trace!(alloc = ?entry.alloc, task = ?entry.task, stream_type = ?entry.stream_type, "writing log entry");

		let LogEntry {
			alloc,
			task,
			stream_type,
			ts,
			message,
		} = entry;
		let minimal_entry = MinimalLogEntry { ts, message };

		// Save the entry
		let key = LogEntryKey {
			alloc,
			task,
			stream_type,
		};
		if let Some(entries) = self.log_entires_buffer.get_mut(&key) {
			entries.push(minimal_entry);
		} else {
			self.log_entires_buffer.insert(key, vec![minimal_entry]);
		}
	}
}
