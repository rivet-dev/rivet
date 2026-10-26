//! Operator KV commands use the typed volume API and preserve opaque bytes at the CLI boundary.

use super::{HexBytes, VolumeAddress};
use anyhow::Result;
use bytes::Bytes;
use clap::Subcommand;
use depot_client::Client;
use depot_protocol::command::{Command as TypedCommand, CommandTag, KvApply, KvGet};
use rivet_data::generated::{depot_kv_command_v1 as kv, depot_operator_command_v1 as wire};
use rivet_keys::keys::codec::{FormalKey, KeyAddress};
use serde_json::{Value, json};

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Read a committed value. Volume addresses use partition hex with optional :name hex; keys and values use hex.
	Get {
		volume: VolumeAddress,
		key: HexBytes,
	},

	/// Replace a value and wait for its volume quorum before acknowledging.
	Put {
		volume: VolumeAddress,
		key: HexBytes,
		value: HexBytes,
	},

	/// Delete one key without deleting its volume.
	Delete {
		volume: VolumeAddress,
		key: HexBytes,
	},

	/// Atomically replace or delete a key only if its current bytes match the expectation.
	CompareAndSet {
		volume: VolumeAddress,
		key: HexBytes,
		#[arg(
			long,
			conflicts_with = "expect_absent",
			required_unless_present = "expect_absent"
		)]
		expect: Option<HexBytes>,
		#[arg(long)]
		expect_absent: bool,
		#[arg(long, conflicts_with = "delete", required_unless_present = "delete")]
		value: Option<HexBytes>,
		#[arg(long)]
		delete: bool,
	},
}

/// Routes the complete operation to the maintainer; retries retain the operation identity.
pub(super) async fn execute(client: &Client, command: &Command) -> Result<Value> {
	match command {
		Command::Get { volume, key } => {
			let value = client
				.command::<KvGet>(
					&volume.0,
					kv::KvGet {
						key: key.0.clone().into(),
					},
				)
				.await?
				.value;
			Ok(json!({ "value": value.map(hex::encode) }))
		}
		Command::Put { volume, key, value } => {
			client
				.command::<KvApply>(
					&volume.0,
					kv::KvApply {
						mutations: vec![kv::Mutation::Put(kv::Put {
							key: key.0.clone().into(),
							value: value.0.clone().into(),
						})],
					},
				)
				.await?;
			Ok(json!({ "written": true }))
		}
		Command::Delete { volume, key } => {
			client
				.command::<KvApply>(
					&volume.0,
					kv::KvApply {
						mutations: vec![kv::Mutation::Delete(kv::Delete {
							key: key.0.clone().into(),
						})],
					},
				)
				.await?;
			Ok(json!({ "deleted": true }))
		}
		Command::CompareAndSet {
			volume,
			key,
			expect,
			expect_absent: _,
			value,
			delete: _,
		} => {
			let response = client
				.command::<CompareAndSet>(
					&volume.0,
					wire::CompareAndSetRequest {
						key: key.0.clone().into(),
						expected: (expect.as_ref().map(|value| value.0.clone())).map(Into::into),
						value: (value.as_ref().map(|value| value.0.clone())).map(Into::into),
					},
				)
				.await?;
			Ok(json!({ "applied": response.applied, "value": response.value.map(hex::encode) }))
		}
	}
}

/// Conditional operator mutations tag their successful write with the operation id.
/// Finding the tag is success even if a later operator has since changed the value.
pub(crate) struct CompareAndSet;

impl TypedCommand for CompareAndSet {
	const TAG: CommandTag =
		CommandTag(rivet_keys::keys::tags::depot_command::OPERATOR_COMPARE_AND_SET);
	type Request = rivet_data::versioned::DepotOperatorCompareAndSetRequestData;
	type Response = rivet_data::versioned::DepotOperatorCompareAndSetResponseData;
}

rivet_keys::volume_kv_key! {
	MutationKey(key: Bytes, operation_id: uuid::Uuid) => [OPERATOR_MUTATION, {key}, {operation_id}];
	value: vbare(rivet_data::versioned::DepotOperatorMutationTagData, rivet_data::DEPOT_OPERATOR_COMMAND_VERSION);
}

pub(crate) fn register(registry: &mut depot_worker::command::CommandRegistry) -> Result<()> {
	registry.register::<CompareAndSet>(compare_and_set)
}

async fn compare_and_set(
	kv: depot_worker::command::KvHandle,
	request: wire::CompareAndSetRequest,
) -> Result<wire::CompareAndSetResponse> {
	let marker = MutationKey::new(
		Bytes::copy_from_slice(&request.key),
		uuid::Uuid::from_bytes(kv.operation_id()),
	);
	loop {
		let mut txn = kv.begin().await?;
		if txn.get(&marker.pack()).await?.is_some() {
			return Ok(wire::CompareAndSetResponse {
				applied: true,
				value: request.value,
			});
		}
		let current = txn.get(&request.key).await?.map(|value| value.to_vec());
		if current.as_deref() != request.expected.as_deref() {
			return Ok(wire::CompareAndSetResponse {
				applied: false,
				value: current.map(Into::into),
			});
		}
		match &request.value {
			Some(value) => txn.put(&request.key, value)?,
			None => txn.delete(&request.key)?,
		}
		txn.put(
			&marker.pack(),
			&marker.serialize(wire::MutationTag {
				operation_id: kv.operation_id(),
			})?,
		)?;
		match kv.commit(txn).await {
			Ok(_) => {
				return Ok(wire::CompareAndSetResponse {
					applied: true,
					value: request.value,
				});
			}
			Err(
				depot_client::filesystem::Error::Conflict | depot_client::filesystem::Error::TooOld,
			) => continue,
			Err(error) => return Err(error.into()),
		}
	}
}
