use std::panic::{AssertUnwindSafe, panic_any};

use anyhow::{Result, anyhow};
use ciborium::{de::from_reader, ser::into_writer};
use futures::FutureExt;
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"validation_error",
	"Actor validation failed",
	"Failed to {operation} {target}: {reason}"
)]
struct ActorValidationError {
	operation: String,
	target: String,
	reason: String,
}

pub(crate) fn decode_cbor<T>(bytes: &[u8], target: &str) -> Result<T>
where
	T: serde::de::DeserializeOwned,
{
	from_reader(bytes).map_err(|error| {
		ActorValidationError {
			operation: "parse".to_owned(),
			target: target.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})
}

pub(crate) fn encode_cbor<T>(value: &T, target: &str) -> Result<Vec<u8>>
where
	T: Serialize,
{
	let mut bytes = Vec::new();
	into_writer(value, &mut bytes).map_err(|error| {
		ActorValidationError {
			operation: "serialize".to_owned(),
			target: target.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})?;
	Ok(bytes)
}

pub(crate) async fn catch_unwind_result<F, T>(future: F) -> Result<T>
where
	F: std::future::Future<Output = Result<T>> + Send,
{
	AssertUnwindSafe(future)
		.catch_unwind()
		.await
		.map_err(panic_payload_to_error)?
}

pub(crate) fn panic_with_error(error: anyhow::Error) -> ! {
	panic_any(error)
}

fn panic_payload_to_error(payload: Box<dyn std::any::Any + Send>) -> anyhow::Error {
	match payload.downcast::<anyhow::Error>() {
		Ok(error) => *error,
		Err(payload) => match payload.downcast::<String>() {
			Ok(message) => anyhow!(*message),
			Err(payload) => match payload.downcast::<&'static str>() {
				Ok(message) => anyhow!(*message),
				Err(_) => anyhow!("typed actor callback panicked"),
			},
		},
	}
}
