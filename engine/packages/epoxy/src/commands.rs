//! Application command handlers run transactions on the tablet maintainer.
use anyhow::{Result, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{CommandRegistry, KvHandle, ReadHandle};
use epoxy_protocol::{protocol, versioned};
use std::time::Duration;

/// Registers Epoxy's commands. Key volumes close `key_idle_close` after their last command,
/// which also begins their unload: a replica keeps no per-key state between rounds, and every
/// round of one proposal runs well within that window.
pub fn register(registry: &mut CommandRegistry, key_idle_close: Duration) -> Result<()> {
	registry.register_with::<ReplicaCommand>(256, Some(key_idle_close), replica)?;
	registry.register_read::<ReplicaRead>(256, Some(key_idle_close), replica_read)
}

/// Transitions are fenced by their ballots, versions and configuration epochs. A resent
/// single-replica commit finds its own value and writes nothing, and a resent ballot
/// reservation only skips a ballot number.
pub struct ReplicaCommand;

impl Command for ReplicaCommand {
	const TAG: CommandTag =
		CommandTag(rivet_keys::keys::tags::depot_command::EPOXY_REPLICA_COMMAND);
	// Every operation reads and writes only the key's record and has no effect, so a
	// reservation on a new key volume is written with the volume's creation. On a volume that
	// existed, the rerun compares the value it reads, whoever wrote it.
	const SPECULATIVE_CREATE: bool = true;
	type Request = versioned::ReplicaCommand;
	type Response = versioned::ReplicaCommandResponse;
}

async fn replica(
	kv: KvHandle,
	request: protocol::ReplicaCommand,
) -> Result<protocol::ReplicaCommandResponse> {
	ensure!(
		kv.volume_key()
			== &crate::depot::keys::partition::key_volume(request.replica_id, &request.key),
		"record database does not match replica key"
	);
	crate::depot::local_replica::LocalReplica::new(kv, request.replica_id)
		.run(request.operation.into())
		.await
}

/// A stale-tolerant committed value read. It runs on a maintainer read session when the key
/// volume is not open, so it never opens the writer for a gateway lookup. Only `GetReplica`
/// is accepted: its callers tolerate a possibly stale value.
pub struct ReplicaRead;

impl Command for ReplicaRead {
	const TAG: CommandTag = CommandTag(rivet_keys::keys::tags::depot_command::EPOXY_REPLICA_READ);
	type Request = versioned::ReplicaCommand;
	type Response = versioned::ReplicaCommandResponse;
}

async fn replica_read(
	read: ReadHandle,
	request: protocol::ReplicaCommand,
) -> Result<protocol::ReplicaCommandResponse> {
	ensure!(
		read.volume_key()
			== &crate::depot::keys::partition::key_volume(request.replica_id, &request.key),
		"record database does not match replica key"
	);
	match request.operation {
		protocol::ReplicaOperation::GetReplica => Ok(
			protocol::ReplicaCommandResponse::ReplicaValue(protocol::ReplicaValue {
				value: crate::depot::local_replica::LocalReplica::get_read(&read, &request.key)
					.await?,
			}),
		),
		protocol::ReplicaOperation::InspectReplica
		| protocol::ReplicaOperation::PrepareRequest(_)
		| protocol::ReplicaOperation::AcceptRequest(_)
		| protocol::ReplicaOperation::CommitRequest(_)
		| protocol::ReplicaOperation::KvReadStateRequest(_) => {
			anyhow::bail!("replica read accepts only GetReplica")
		}
	}
}

/// A missing database is an empty snapshot; reads never allocate a durable volume.
pub(crate) async fn read<C: Command>(
	client: &depot_client::Client,
	key: &rivet_keys::keys::VolumeKey,
	request: depot_protocol::command::Request<C>,
	missing: depot_protocol::command::Response<C>,
) -> Result<depot_protocol::command::Response<C>> {
	match client.command::<C>(key, request).await {
		Err(error) if is_missing(&error) => Ok(missing),
		result => result,
	}
}

pub(crate) fn is_missing(error: &anyhow::Error) -> bool {
	error
		.downcast_ref::<rivet_error::RivetError>()
		.is_some_and(|error| error.group() == "depot" && error.code() == "volume_not_found")
}

/// Epoxy creates its database lazily. The storage host creates a missing database inside the
/// command's first open, so the command runs once with one operation id.
pub(crate) async fn run<C: Command>(
	client: &depot_client::Client,
	key: &rivet_keys::keys::VolumeKey,
	request: depot_protocol::command::Request<C>,
) -> Result<depot_protocol::command::Response<C>> {
	client
		.command_with::<C>(
			key,
			request,
			depot_client::CommandOptions {
				create_if_missing: true,
			},
		)
		.await
}
