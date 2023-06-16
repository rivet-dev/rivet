use futures_util::{Stream, StreamExt};
use nomad_client::apis::configuration::Configuration;
use serde::Deserialize;
use std::pin::Pin;

use crate::error::NomadError;

const MAX_LOG_LINE_LEN: usize = 1024;

/// JSON frame returned from the file stream.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", untagged)]
#[allow(unused)]
enum StreamFrame {
	#[serde(rename_all = "PascalCase")]
	Data {
		data: String,
		offset: usize,
	},
	#[serde(rename_all = "PascalCase")]
	FileEvent {
		file_event: FrameFileEvent,
	},
	Empty {},
}

#[derive(Debug, Deserialize)]
enum FrameFileEvent {
	#[serde(rename = "file deleted")]
	FileDeleted,
	#[serde(rename = "file truncated")]
	FileTruncated,
}

/// Event returned from the log stream.
#[derive(Debug, Clone)]
pub enum NomadLogEvent {
	LogLine(NomadLogLine),
	Finished,
}

/// A line complete to the \n from the log.
#[derive(Debug, Clone)]
pub struct NomadLogLine {
	pub ts: i64,
	pub message: Vec<u8>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum StreamType {
	StdOut,
	StdErr,
}

impl StreamType {
	pub fn as_str(&self) -> &'static str {
		match self {
			StreamType::StdOut => "stdout",
			StreamType::StdErr => "stderr",
		}
	}
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum StreamOrigin {
	Start,
	End,
}

impl StreamOrigin {
	pub fn as_str(&self) -> &'static str {
		match self {
			StreamOrigin::Start => "start",
			StreamOrigin::End => "end",
		}
	}
}

pub struct LogStream {
	stream: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,

	/// Buffer for the JSON Nomad event.
	///
	/// This will be parsed to `StreamFrame`.
	frame_buf: Vec<u8>,

	/// Buffer for a single line of the log.
	log_line_buf: Vec<u8>,
}

impl LogStream {
	#[tracing::instrument]
	pub async fn new(
		configuration: Configuration,
		alloc: String,
		task: String,
		r#type: StreamType,
		origin: StreamOrigin,
		offset: Option<usize>,
	) -> Result<LogStream, NomadError> {
		// Open event stream for the alloc and event topics. See more topics here:
		// https://www.nomadproject.io/api-docs/events#event-topics
		let res = reqwest::Client::new()
			.get(format!(
				"{}/client/fs/logs/{}",
				configuration.base_path, alloc
			))
			.query(&[
				("task", task.as_str()),
				("type", r#type.as_str()),
				("origin", origin.as_str()),
				("offset", offset.unwrap_or(0).to_string().as_str()),
				("follow", "true"),
			])
			.send()
			.await?;
		tracing::info!(url = %res.url(), "started stream");
		if !res.status().is_success() {
			return Err(NomadError::StreamResponseStatus {
				status: res.status().into(),
			});
		}

		let stream = res.bytes_stream();

		Ok(LogStream {
			stream: Pin::from(Box::new(stream)),
			frame_buf: Vec::new(),
			log_line_buf: Vec::new(),
		})
	}

	#[tracing::instrument(skip_all)]
	pub async fn next(&mut self) -> Result<Vec<NomadLogEvent>, NomadError> {
		let mut pending_log_events = Vec::<NomadLogEvent>::new();

		// Buffer events and parse each event
		while let Some(item) = self.stream.next().await {
			let item = item?;

			for byte in item {
				self.frame_buf.push(byte);

				// Match closing JSON delimiters to check if we need to parse
				// JSON. This doesn't guarantee there will be valid JSON.
				if matches!(byte, b'}' | b']' | b'"') {
					// Attempt to parse JSON from the given buffer. If JSON is
					// invalid, ignore and keep reading bytes.
					let frame_value = match serde_json::from_slice::<serde_json::Value>(
						self.frame_buf.as_slice(),
					) {
						Ok(x) => x,
						Err(_) => {
							// JSON is not valid, continue parsing bytes
							// until we reach valid JSON
							continue;
						}
					};

					// Decode in to a frame
					let frame = serde_json::from_value::<StreamFrame>(frame_value)?;
					tracing::trace!(?frame, "log frame");
					self.frame_buf.clear();

					// Process the event
					match frame {
						StreamFrame::Empty {} => {}
						StreamFrame::Data { data, .. } => {
							let data_decoded = base64::decode(data)?;

							// Buffer the logs and split at each line
							for log_byte in data_decoded {
								if log_byte == b'\n' {
									self.write_log_line(&mut pending_log_events);
								} else {
									// Append to log if line is not too long
									if self.log_line_buf.len() < MAX_LOG_LINE_LEN {
										self.log_line_buf.push(log_byte);
									}
								}
							}
						}
						StreamFrame::FileEvent { .. } => {
							self.finish_log(&mut pending_log_events);
						}
					}

					// Return pending logs if any
					if !pending_log_events.is_empty() {
						return Ok(pending_log_events);
					}
				}
			}
		}

		tracing::warn!("unexpectedly reached end of log stream");

		self.finish_log(&mut pending_log_events);

		Ok(pending_log_events)
	}

	/// Writes the rest of the log line and the `Finished` to the pending
	/// events.
	fn finish_log(&mut self, pending_log_events: &mut Vec<NomadLogEvent>) {
		// Write the rest of the log
		if !self.log_line_buf.is_empty() {
			self.write_log_line(pending_log_events);
		}

		// Write finished event
		pending_log_events.push(NomadLogEvent::Finished);
	}

	/// Write the log line to the log.
	fn write_log_line(&mut self, pending_log_events: &mut Vec<NomadLogEvent>) {
		let message = std::mem::take(&mut self.log_line_buf);
		pending_log_events.push(NomadLogEvent::LogLine(NomadLogLine {
			ts: crate::util::now(),
			message,
		}));
	}
}
