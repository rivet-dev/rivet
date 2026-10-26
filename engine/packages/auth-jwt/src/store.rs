//! Typed generated BARE object storage. Only the singleton executor writes this object.
use crate::key_ring_machine::types::{ReadResult, StoredRing};
use anyhow::{Context, Result, ensure};
use object_store::{GetResult, ObjectStore, ObjectStoreExt};
use rivet_keys::keys::KeyAddress;

rivet_keys::s3_key! {
	pub SigningKeyRingKey() => [RIVET, CLUSTER, AUTH, JWT, KEY_RING];
	value: vbare(rivet_data::versioned::AuthJwtKeyRingData, rivet_data::AUTH_JWT_KEY_RING_VERSION);
}
pub const MAX_RING_BYTES: u64 = 64 * 1024;

/// Reject malformed, oversized or unversioned objects without ever treating them as absent.
pub async fn decode_result(result: GetResult) -> Result<StoredRing> {
	ensure!(
		result.meta.size <= MAX_RING_BYTES,
		"JWT ring exceeds byte limit"
	);
	let etag = result
		.meta
		.e_tag
		.clone()
		.filter(|v| !v.is_empty())
		.context("JWT ring lacks ETag")?;
	let bytes = result.bytes().await?;
	ensure!(
		bytes.len() <= MAX_RING_BYTES as usize,
		"JWT ring exceeds byte limit"
	);
	crate::decode_signing_key_ring(&bytes)?;
	Ok(StoredRing {
		bytes: bytes.to_vec(),
		etag,
	})
}
pub async fn read(store: &dyn ObjectStore) -> ReadResult {
	let result = match store.get(&SigningKeyRingKey::new().pack()).await {
		Ok(result) => result,
		Err(object_store::Error::NotFound { .. }) => return ReadResult::Missing,
		Err(error) => return ReadResult::Unknown(error.to_string()),
	};
	match decode_result(result).await {
		Ok(stored) => ReadResult::Found(stored),
		Err(error) => {
			tracing::warn!(
				?error,
				"JWT key ring is invalid; maintenance will not overwrite it"
			);
			ReadResult::Invalid(error.to_string())
		}
	}
}
