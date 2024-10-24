use std::{future::Future, pin::Pin};

use futures_util::{Stream, StreamExt};
use nomad_client_new::apis::configuration::Configuration;
use redis::AsyncCommands;
use rivet_pools::prelude::*;
use serde::{de::DeserializeOwned, Deserialize};

use crate::error::NomadError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NomadEventResponse {
	pub events: Option<Vec<NomadEvent>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NomadEvent {
	pub topic: String,
	pub r#type: String,
	pub key: String,
	pub namespace: String,
	pub index: u64,
	pub payload: Box<serde_json::value::RawValue>,
}

impl NomadEvent {
	/// Decodes the event payload in to a given datastructure if the topic and type match.
	pub fn decode<T>(&self, topic: &str, ty: &str) -> Result<Option<T>, NomadError>
	where
		T: DeserializeOwned,
	{
		if self.topic != topic || self.r#type != ty {
			return Ok(None);
		}

		let payload = serde_json::from_str(self.payload.get())?;

		Ok(payload)
	}
}

pub struct Monitor {
	stream: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
	buf: Vec<u8>,
}

impl Monitor {
	// TODO: Write most recent index to Redis
	#[tracing::instrument]
	pub async fn new(
		configuration: Configuration,
		topics: &[&str],
		index: Option<usize>,
	) -> Result<Monitor, NomadError> {
		// Open event stream for the alloc and event topics. See more topics here:
		// https://www.nomadproject.io/api-docs/events#event-topics
		let res = reqwest::get(format!(
			"{}/event/stream?index={index}&{topics}",
			configuration.base_path,
			index = index.unwrap_or(0),
			topics = topics
				.iter()
				.map(|x| format!("topic={}", x))
				.collect::<Vec<_>>()
				.join("&"),
		))
		.await?;
		if !res.status().is_success() {
			return Err(NomadError::StreamResponseStatus {
				status: res.status().into(),
			});
		}

		let stream = res.bytes_stream();

		Ok(Monitor {
			stream: Pin::from(Box::new(stream)),
			buf: Vec::new(),
		})
	}

	#[tracing::instrument(skip_all)]
	pub async fn next(&mut self) -> Result<Vec<NomadEvent>, NomadError> {
		// Buffer events and parse each event
		while let Some(item) = self.stream.next().await {
			let item = item?;

			for byte in item {
				self.buf.push(byte);

				if byte == b'\n' {
					// Parse the event and clear the buffer
					let event_res =
						serde_json::from_slice::<NomadEventResponse>(self.buf.as_slice())?;
					self.buf.clear();

					if let Some(events) = event_res.events {
						return Ok(events);
					}
				}
			}
		}

		Err(NomadError::EventStreamClosed)
	}

	#[tracing::instrument(skip(configuration, redis_conn, handle))]
	pub async fn run<F, Fut>(
		configuration: Configuration,
		mut redis_conn: RedisPool,
		redis_index_key: &str,
		topics: &[&str],
		handle: F,
	) -> Result<(), NomadError>
	where
		F: Fn(NomadEvent) -> Fut,
		Fut: Future<Output = ()>,
	{
		// Read initial index
		let index = match redis_conn.get::<_, Option<usize>>(&redis_index_key).await {
			Ok(Some(index)) => {
				tracing::info!(?index, "start index");
				Some(index)
			}
			Ok(None) => {
				tracing::info!("no start index");
				None
			}
			Err(err) => {
				tracing::error!(?err, "error fetching start index");
				None
			}
		};

		let mut monitor = Monitor::new(configuration, topics, index).await?;

		loop {
			let events = monitor.next().await?;

			// Update index
			if let Some(last) = events.last() {
				tracing::trace!(?last.index, "writing index");
				match redis_conn.set(&redis_index_key, last.index).await {
					Ok(()) => {}
					Err(err) => {
						tracing::error!(?err, "error setting start index");
					}
				};
			}

			// Handle events
			for event in events {
				handle(event).await;
			}
		}
	}
}
