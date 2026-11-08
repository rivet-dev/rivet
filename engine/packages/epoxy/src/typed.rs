//! Reads and writes of formal keys: a `rivet_keys` key whose packed bytes are the Epoxy key
//! and whose codec encodes the value.

use anyhow::{Context, Result};
use rivet_keys::keys::{
	KeyAddress,
	codec::{FormalKey, ValueCodec},
};

use crate::{
	client::Client,
	ops::{
		kv::get::{self, ReadMode},
		propose::{
			CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Input, Proposal,
			ProposalResult, SetCommand,
		},
	},
};

/// A formal key whose packed form is the raw bytes Epoxy addresses.
pub trait EpoxyKey: FormalKey + KeyAddress<Packed = bytes::Bytes> {}

impl<K: FormalKey + KeyAddress<Packed = bytes::Bytes>> EpoxyKey for K {}

impl Client {
	/// The key's newest committed value, decoded. A key that was never written or whose last
	/// write deleted it reads as `None`.
	pub async fn read_key<K: EpoxyKey>(
		&self,
		key: K,
	) -> Result<Option<<K::Codec as ValueCodec>::Value>> {
		let output = self
			.op(get::Input {
				key: key.pack().to_vec(),
				mode: ReadMode::Linearizable,
			})
			.await?;
		let Some(value) = output.value.and_then(|committed| committed.value) else {
			return Ok(None);
		};
		Ok(Some(key.deserialize(value)?))
	}

	/// Overwrites the key's value.
	pub async fn write_key<K: EpoxyKey>(
		&self,
		key: K,
		value: <K::Codec as ValueCodec>::Input,
	) -> Result<()> {
		let encoded = key.serialize(value)?.to_vec();
		self.set_key(key.pack().to_vec(), Some(encoded)).await
	}

	/// Deletes the key's value. Deleting a key that was never written still creates its volume,
	/// so a caller that may not know whether the key exists reads it first.
	pub async fn delete_key<K: EpoxyKey>(&self, key: K) -> Result<()> {
		self.set_key(key.pack().to_vec(), None).await
	}

	/// Writes the key's value only if the key holds none. Returns `false` when it already holds
	/// one, leaving that value in place.
	pub async fn create_key<K: EpoxyKey>(
		&self,
		key: K,
		value: <K::Codec as ValueCodec>::Input,
	) -> Result<bool> {
		let encoded = key.serialize(value)?.to_vec();
		let result = self
			.propose(Input {
				proposal: Proposal {
					commands: vec![Command {
						kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
							key: key.pack().to_vec(),
							expect_one_of: vec![None],
							new_value: Some(encoded),
						}),
					}],
				},
				mutable: true,
			})
			.await?;
		match result {
			ProposalResult::Committed => Ok(true),
			ProposalResult::ConsensusFailed {
				reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { .. },
			} => Ok(false),
		}
	}

	async fn set_key(&self, key: Vec<u8>, value: Option<Vec<u8>>) -> Result<()> {
		let result = self
			.propose(Input {
				proposal: Proposal {
					commands: vec![Command {
						kind: CommandKind::SetCommand(SetCommand { key, value }),
					}],
				},
				mutable: true,
			})
			.await?;
		result.resolve().context("Epoxy write was not committed")
	}
}
