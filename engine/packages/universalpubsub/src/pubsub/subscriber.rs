//! Whether a worker holds a subscriber for a subject, asked without reaching the subscriber.
use std::sync::atomic::Ordering;

use anyhow::Result;
use axum::{
	extract::{Path, State},
	http::{StatusCode, Version},
	response::{IntoResponse, Response},
};
use futures_util::{StreamExt, stream};
use rivet_env::capabilities::http::{PeerLane, PeerRoute};
use uuid::Uuid;

use super::{PubSub, WorkerId};
use crate::errors;

/// The route a peer is asked on: the worker the question is for and the subject in hex.
pub(crate) const PATH: &str = "/ups/v1/subscriber/{worker_id}/{subject}";

impl PubSub {
	/// Whether `worker` currently holds a subscriber for `subject`: the registration a stream
	/// open to it would be queued on. The local worker answers from its own registry; a peer
	/// answers one request from its registry, which reaches no subscriber. A stream
	/// open to an absent subscriber fails with `no_responders`; this is that answer without the
	/// stream, so it proves the registration exists, not that whatever is behind it is alive.
	/// Fails when the worker cannot be asked: unresolvable, unreachable, or too old to serve
	/// the route.
	pub async fn has_subscriber(
		&self,
		worker: &WorkerId,
		subject: impl AsRef<[u8]>,
	) -> Result<bool> {
		let subject = subject.as_ref();
		if worker == self.worker_id() {
			return Ok(self.inner.subscriptions.lock().contains_key(subject));
		}

		let endpoint = self
			.inner
			.topology
			.resolve_worker(worker)
			.await
			.map_err(super::request::endpoint_unresolved)?
			.ok_or_else(|| errors::UnknownWorker.build())?;
		let request = rivet_env::capabilities::http::Request::builder()
			.method("GET")
			.version(Version::HTTP_2)
			.uri(format!(
				"{}/ups/v1/subscriber/{}/{}",
				endpoint.api_peer_address.trim_end_matches('/'),
				Uuid::from_bytes(*worker.as_bytes()),
				hex(subject)
			))
			.extension(PeerRoute {
				lane: PeerLane::Request,
				key: self.inner.next_route_key.fetch_add(1, Ordering::Relaxed),
			})
			.body(stream::empty().boxed())?;
		// The answer travels in the status; the body is empty.
		let clock = self.env().clock();
		let response = rivet_env::timeout(
			clock.as_ref(),
			self.inner.options.request_timeout,
			self.inner.client.execute_stream(request),
		)
		.await
		.map_err(super::request::timeout_error)?
		.map_err(super::http::transport_error)?;
		match response.status() {
			StatusCode::OK => Ok(true),
			StatusCode::NO_CONTENT => Ok(false),
			// A peer from before this route answers not found, which must not read as an
			// absent subscriber.
			StatusCode::NOT_FOUND => Err(errors::Transport
				.build()
				.context("the peer does not answer subscriber questions")),
			status => Err(super::http::status_error(status)),
		}
	}
}

pub(super) async fn receive(
	State(ups): State<PubSub>,
	Path((worker_id, subject)): Path<(String, String)>,
) -> Response {
	let Ok(worker_id) = Uuid::parse_str(&worker_id) else {
		return StatusCode::BAD_REQUEST.into_response();
	};
	if WorkerId::from_bytes(*worker_id.as_bytes()) != *ups.worker_id() {
		return StatusCode::CONFLICT.into_response();
	}
	let Some(subject) = unhex(&subject) else {
		return StatusCode::BAD_REQUEST.into_response();
	};
	if ups.inner.subscriptions.lock().contains_key(&subject) {
		StatusCode::OK.into_response()
	} else {
		StatusCode::NO_CONTENT.into_response()
	}
}

fn hex(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
	if !text.len().is_multiple_of(2) {
		return None;
	}
	(0..text.len())
		.step_by(2)
		.map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
		.collect()
}
