use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::ActorContext as CoreActorContext;

use crate::{NapiInvalidArgument, napi_anyhow_error};

#[napi]
pub struct Schedule {
	inner: CoreActorContext,
}

impl Schedule {
	pub(crate) fn new(inner: CoreActorContext) -> Self {
		Self { inner }
	}
}

#[napi]
impl Schedule {
	#[napi]
	pub fn after(&self, duration_ms: i64, action_name: String, args: Buffer) -> napi::Result<()> {
		let duration_ms = u64::try_from(duration_ms).map_err(|_| {
			napi_anyhow_error(
				NapiInvalidArgument {
					argument: "durationMs".to_owned(),
					reason: "must be non-negative".to_owned(),
				}
				.build(),
			)
		})?;
		self.inner.after(
			Duration::from_millis(duration_ms),
			&action_name,
			args.as_ref(),
		);
		Ok(())
	}

	#[napi]
	pub fn at(&self, timestamp_ms: i64, action_name: String, args: Buffer) {
		self.inner.at(timestamp_ms, &action_name, args.as_ref());
	}
}
