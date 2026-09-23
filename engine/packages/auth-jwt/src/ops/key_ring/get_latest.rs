use crate::storage_keys::{self, SigningKeyRingKey};
use anyhow::Result;
use gas::prelude::*;

use super::{ReadOutput, decode_committed};

#[derive(Debug)]
pub struct Input;

/// Reads the ring through consensus before planning a mutation, including unfinished writes.
#[operation]
pub async fn auth_jwt_key_ring_get_latest(
	ctx: &OperationCtx,
	_input: &Input,
) -> Result<ReadOutput> {
	let logical_key = SigningKeyRingKey;
	let key = storage_keys::subspace().pack(&logical_key);
	let Some(committed) = ctx
		.op(epoxy::ops::kv::get::Input {
			key,
			mode: epoxy::ops::kv::get::ReadMode::Linearizable {
				target_replicas: None,
			},
		})
		.await?
		.value
	else {
		return Ok(ReadOutput::Absent);
	};
	match decode_committed(&logical_key, committed) {
		Ok(snapshot) => Ok(ReadOutput::Present(Box::new(snapshot))),
		Err(error) => Ok(ReadOutput::Corrupt {
			reason: format!("{error:#}"),
		}),
	}
}
