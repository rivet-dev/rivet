use std::time::Duration;

use anyhow::*;
use serde::Serialize;
use serde_json::value::RawValue;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub mod error;
use error::Error;

const BATCH_INTERVAL: Duration = Duration::from_millis(500);
const BATCH_SIZE: usize = 10_000;

/// A handle to the ClickHouse inserter service
#[derive(Clone)]
pub struct ClickHouseInserterHandle {
	sender: mpsc::Sender<Event>,
}

#[derive(Serialize)]
struct Event {
	source: &'static str,
	database: &'static str,
	table: &'static str,
	columns: Box<RawValue>,
}

impl ClickHouseInserterHandle {
	/// Sends an event to the ClickHouse database with a specific database
	pub fn insert(
		&self,
		database: &'static str,
		table: &'static str,
		columns: impl serde::Serialize,
	) -> Result<()> {
		// Serialize the columns to a JSON string
		let columns = serde_json::value::to_raw_value(&columns).map_err(|e| {
			tracing::error!(?e, "failed to serialize columns for ClickHouse");
			Error::SerializationError(e)
		})?;

		// Send the event to the background task
		self.sender
			.try_send(Event {
				source: "clickhouse",
				database,
				table,
				columns,
			})
			.map_err(|e| {
				tracing::error!(?e, "failed to send event to ClickHouse inserter");
				Error::ChannelSendError
			})?;

		Ok(())
	}
}

struct InserterService {
	receiver: mpsc::Receiver<Event>,
	vector_url: String,
	client: reqwest::Client,
	cancel_token: CancellationToken,
}

impl InserterService {
	async fn run(mut self) -> Result<(), anyhow::Error> {
		let mut interval = tokio::time::interval(BATCH_INTERVAL);
		let mut events = Vec::new();

		loop {
			tokio::select! {
				_ = self.cancel_token.cancelled() => {
					tracing::info!("clickhouse inserter service shutting down");
					// Send remaining events before shutting down
					if !events.is_empty() {
						if let Err(e) = self.send_events(&events).await {
							tracing::error!(?e, "failed to send final events to Vector");
						}
					}
					break;
				}
				// Receive new events from the channel
				Some(event) = self.receiver.recv() => {
					events.push(event);

					// Send batch if it's reached the size limit
					if events.len() >= BATCH_SIZE {
						if let Err(e) = self.send_events(&events).await {
							tracing::error!(?e, "failed to send events to Vector");
						}
						events.clear();
					}
				}
				// Timer tick - send any pending events
				_ = interval.tick() => {
					if !events.is_empty() {
						if let Err(e) = self.send_events(&events).await {
							tracing::error!(?e, "failed to send events to Vector");
						}
						events.clear();
					}
				}
			}
		}

		Ok(())
	}

	async fn send_events(&self, events: &[Event]) -> Result<(), anyhow::Error> {
		let mut payload = Vec::new();

		// Write each event as a separate JSON line (NDJSON format)
		for event in events {
			serde_json::to_writer(&mut payload, event)?;
			payload.push(b'\n');
		}

		tracing::debug!(
			event_count = events.len(),
			event_bytes = payload.len(),
			"sending events to Vector"
		);

		let response = self
			.client
			.post(&self.vector_url)
			.header("Content-Type", "application/x-ndjson")
			.body(payload)
			.send()
			.await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await?;
			tracing::error!(?status, ?body, "vector http request failed");
			return Err(anyhow::anyhow!(
				"Vector HTTP request failed: {} {}",
				status,
				body
			));
		}

		Ok(())
	}
}

/// Creates a new ClickHouse inserter service
pub fn create_inserter(
	vector_host: impl Into<String>,
	vector_port: u16,
) -> Result<ClickHouseInserterHandle> {
	let (sender, receiver) = mpsc::channel(BATCH_SIZE * 2);
	let vector_url = format!("http://{}:{}", vector_host.into(), vector_port);

	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(5))
		.build()
		.map_err(Error::ReqwestBuildError)?;

	let cancel_token = CancellationToken::new();

	let service = InserterService {
		receiver,
		vector_url,
		client,
		cancel_token: cancel_token.clone(),
	};

	// Spawn the background task
	let _ = tokio::spawn(async move {
		if let Err(e) = service.run().await {
			tracing::error!(?e, "clickhouse inserter service failed");
		}
	});

	Ok(ClickHouseInserterHandle { sender })
}
