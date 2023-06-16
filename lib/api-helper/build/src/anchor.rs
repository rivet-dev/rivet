use chirp_client::{error::ClientError, TailAnchor};
use rivet_api::models;
use serde::{Deserialize, Serialize};

/// Used in blocking API requests to denote the last received message from a given endpoint.
/// This is used to block a request until a new event has occurred.
#[derive(Debug, Clone, Deserialize)]
pub struct WatchIndexQuery {
	watch_index: Option<String>,
}

impl WatchIndexQuery {
	pub fn has_watch_index(&self) -> bool {
		self.watch_index.is_some()
	}

	/// Converts the `WatchIndexQuery` into a `TailAnchor` for use with the Chirp client.
	pub fn to_consumer(self) -> Result<Option<TailAnchor>, ClientError> {
		if let Some(watch_index) = self.watch_index {
			Ok(Some(chirp_client::TailAnchor {
				start_time: watch_index.parse()?,
			}))
		} else {
			Ok(None)
		}
	}
}

/// Anchor response sent back to the client. Equivalent and opposite to `WatchIndexQuery`.
#[derive(Debug, Clone, Serialize)]
pub struct WatchResponse {
	index: String,
}

impl WatchResponse {
	/// Create a new anchor response with a given timestamp.
	pub fn new(ts: impl ToString) -> Self {
		WatchResponse {
			index: ts.to_string(),
		}
	}

	// TODO: The `+ 1` may not be necessary
	pub fn new_as_model(ts: i64) -> Box<models::WatchResponse> {
		Box::new(models::WatchResponse {
			index: (ts + 1).to_string(),
		})
	}

	// TODO: Remove when all `<api service>::utils::watch_response`'s is fully deprecated
	#[deprecated(note = "Use `api_helper::anchor::WatchResponse::new_as_model`")]
	pub fn watch_index(&self) -> &String {
		&self.index
	}
}
