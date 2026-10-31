//! Payload ownership and the injected replica transport. No protocol decisions live here.

pub mod recovery;
pub(crate) mod requests;

use super::{
	effect::VolumeClientEffect,
	event::{RpcError, VolumeClientEvent},
	types::{EntryRef, MutationIdentity, OpId, PayloadId},
};
use depot_types::{VolumeLogKey, payload_codec::PayloadDigest};
use rivet_data::generated::{
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogEntryBody, VolumeLogReplicaState},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition},
};
use rivet_topology::worker::WorkerId;
use std::{collections::BTreeMap, future::Future, pin::Pin};
use vbare::Encode;

pub type TransportFuture<'a, T, E = RpcError> =
	Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

/// The peer must reply only after its shard has written the request to disk.
/// Client mutations carry the core-selected replica set version unchanged, including retries;
/// adapters must not replace it from a cache while the request is in flight.
pub trait VolumeClientTransport: Send + Sync {
	/// Promotes the expected incarnation and tail only after its repair is complete.
	fn mark_in_sync(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		replica_incarnation: [u8; 16],
		expected_appended: VolumeLogPosition,
	) -> TransportFuture<'_, VolumeLogReplicaState>;

	/// Returns a bounded entry range and the replica state needed to validate it.
	fn read(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		start: u64,
		max_entries: u32,
		max_bytes: u32,
	) -> TransportFuture<'_, (VolumeLogReplicaState, Vec<VolumeLogEntry>)>;

	/// Removes a suffix only if the replica still has the observed tail.
	#[allow(
		clippy::too_many_arguments,
		reason = "transport preserves the core-selected version and exact wire preconditions"
	)]
	fn truncate(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		expected_appended: VolumeLogPosition,
		keep: VolumeLogPosition,
		mark_repairing: bool,
	) -> TransportFuture<'_, VolumeLogReplicaState>;

	/// Announces the client's quorum progress without matching a log position.
	fn advance_committed(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		committed: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState>;

	/// Loads the volume through its maintainer and returns the authoritative cold row. With
	/// `create_if_absent`, the maintainer creates a missing volume inside the same open.
	fn open(
		&self,
		volume_key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		deadline_boot_ms: u64,
	) -> TransportFuture<'_, rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse>;

	/// Releases the client's volume through the maintainer after draining.
	fn close(
		&self,
		volume_log_key: VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> TransportFuture<'_, ()>;

	/// Writes the client fence to disk before returning replica evidence.
	fn claim(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState>;

	/// Matches the preceding position and writes the complete entry batch to disk before replying.
	///
	/// The executor polls each append's future once while it processes the append effect, so
	/// a transport that fixes the append's place in its send order before the future first
	/// yields delivers one replica's appends in the order the core issued them.
	#[allow(
		clippy::too_many_arguments,
		reason = "transport preserves the core-selected version and exact wire preconditions"
	)]
	fn append(
		&self,
		worker: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState>;

	/// Until when new writes toward `worker` are refused at `now_boot_ms`, if they are, from
	/// what every client sharing this transport's process learned: its log shard refused their
	/// appends, or answered them slowly. A transport that keeps no such record across clients
	/// answers `None`, and each client then sheds only on its own replicas' refusals.
	fn refusing_until(&self, worker: &WorkerId, now_boot_ms: u64) -> Option<u64>;
}

#[derive(Default)]
pub struct Payloads {
	bodies: BTreeMap<PayloadId, VolumeLogEntryBody>,
	next: u64,
}

impl Payloads {
	/// Measure once at the input boundary. The core stores only the resulting handle and size.
	pub fn insert(&mut self, body: VolumeLogEntryBody) -> anyhow::Result<(PayloadId, u32)> {
		let encoded_bytes = entry_encoded_bytes(&body)?;
		let id = self.insert_measured(body)?;
		Ok((id, encoded_bytes))
	}

	/// Retains a body the caller already measured and hashed. This only allocates an ID and
	/// inserts, so callers holding a shared lock can do payload-sized work before taking it.
	pub fn insert_measured(&mut self, body: VolumeLogEntryBody) -> anyhow::Result<PayloadId> {
		self.next = self
			.next
			.checked_add(1)
			.ok_or_else(|| anyhow::anyhow!("payload IDs exhausted"))?;
		let id = PayloadId(self.next);
		self.bodies.insert(id, body);
		Ok(id)
	}

	/// Resolves a retained payload and attaches its immutable retry identity for transport.
	pub fn entry(&self, entry: &EntryRef) -> anyhow::Result<VolumeLogEntry> {
		let mut value = VolumeLogEntry {
			written_at_unix_ms: entry.written_at_unix_ms,
			volume_seq: entry.id.volume_seq,
			client_epoch: entry.id.client_epoch,
			body: if entry.payload == PayloadId(0) {
				VolumeLogEntryBody::ClaimEntry
			} else {
				self.bodies
					.get(&entry.payload)
					.ok_or_else(|| anyhow::anyhow!("released payload"))?
					.clone()
			},
		};

		// Retry identity belongs to mutations; control entries must not carry it.
		if let Some(identity) = &entry.mutation {
			match &mut value.body {
				VolumeLogEntryBody::PayloadWrite(body) => {
					body.client_seq = identity.client_seq;
					body.request_id = identity.request_id;
					body.payload_digest = identity.payload_digest;
				}
				_ => anyhow::bail!("mutation details on a control entry"),
			}
		}
		Ok(value)
	}

	pub fn release(&mut self, payload: PayloadId) {
		self.bodies.remove(&payload);
	}

	/// Execute one lifecycle or replica request through the same injected peer boundary.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn execute(
		&mut self,
		effect: VolumeClientEffect,
		transport: &dyn VolumeClientTransport,
	) -> anyhow::Result<VolumeClientEvent> {
		Ok(match effect {
			VolumeClientEffect::MarkInSyncRequest {
				op_id,
				worker_id,
				volume_log_key,
				client,
				tablet_replica_set_version,
				replica_incarnation,
				expected_appended,
			} => {
				let result = transport
					.mark_in_sync(
						worker_id.clone(),
						volume_log_key,
						client,
						tablet_replica_set_version,
						replica_incarnation,
						expected_appended,
					)
					.await;
				VolumeClientEvent::MatchResponse {
					op_id,
					worker_id,
					result,
				}
			}
			VolumeClientEffect::ReadVolumeLogEntriesRequest {
				op_id,
				worker_id,
				volume_log_key,
				start,
				max_entries,
				max_bytes,
			} => {
				let result = match transport
					.read(
						worker_id.clone(),
						volume_log_key,
						start,
						max_entries,
						max_bytes,
					)
					.await
				{
					// Store fetched bodies outside the core and return bounded payload handles.
					Ok((state, entries)) => Ok((state, self.retain_entries(entries)?)),
					Err(error) => Err(error),
				};
				VolumeClientEvent::ReadVolumeLogEntriesResponse {
					op_id,
					worker_id,
					result,
				}
			}
			VolumeClientEffect::TruncateRequest {
				op_id,
				worker_id,
				volume_log_key,
				client,
				tablet_replica_set_version,
				expected_appended,
				keep,
				mark_repairing,
			} => {
				let result = transport
					.truncate(
						worker_id.clone(),
						volume_log_key,
						client,
						tablet_replica_set_version,
						expected_appended,
						keep,
						mark_repairing,
					)
					.await;
				VolumeClientEvent::MatchResponse {
					op_id,
					worker_id,
					result,
				}
			}
			VolumeClientEffect::AdvanceCommittedRequest {
				op_id,
				worker_id,
				volume_log_key,
				client,
				tablet_replica_set_version,
				committed_volume_seq,
			} => {
				let result = transport
					.advance_committed(
						worker_id.clone(),
						volume_log_key,
						client,
						tablet_replica_set_version,
						committed_volume_seq,
					)
					.await;
				VolumeClientEvent::AdvanceCommittedResponse {
					op_id,
					worker_id,
					result,
				}
			}
			VolumeClientEffect::VolumeOpenRequest {
				op_id,
				volume_key,
				client_incarnation,
				create_if_absent,
				deadline_boot_ms,
				initial: _,
				read,
			} => VolumeClientEvent::VolumeOpenResponse {
				op_id,
				result: transport
					.open(
						volume_key,
						client_incarnation,
						create_if_absent,
						Vec::new(),
						read,
						deadline_boot_ms,
					)
					.await,
			},
			VolumeClientEffect::VolumeCloseRequest {
				op_id,
				volume_log_key,
				client,
			} => VolumeClientEvent::VolumeCloseResponse {
				op_id,
				result: transport.close(volume_log_key, Some(client)).await,
			},
			VolumeClientEffect::ClaimRequest {
				op_id,
				worker_id,
				volume_log_key,
				client,
				tablet_replica_set_version,
			} => {
				let result = transport
					.claim(
						worker_id.clone(),
						volume_log_key,
						client,
						tablet_replica_set_version,
					)
					.await;
				VolumeClientEvent::ClaimResponse {
					op_id,
					worker_id,
					result,
				}
			}
			VolumeClientEffect::AppendRequest { .. } => {
				anyhow::bail!("appends must be prepared synchronously")
			}
			VolumeClientEffect::WaitForCommitResponse { .. }
			| VolumeClientEffect::ConfirmResponse { .. }
			| VolumeClientEffect::OpenResponse { .. }
			| VolumeClientEffect::RecoveryResponse { .. }
			| VolumeClientEffect::CloseResponse { .. }
			| VolumeClientEffect::WriteResponse { .. }
			| VolumeClientEffect::ReleasePayload { .. } => {
				anyhow::bail!("caller replies and payload releases belong to the owner")
			}
		})
	}

	/// Retain a fetched page only after transport finishes; no payload lock is held across I/O.
	pub fn retain_entries(
		&mut self,
		entries: Vec<VolumeLogEntry>,
	) -> anyhow::Result<Vec<EntryRef>> {
		let mut retained: Vec<EntryRef> = Vec::with_capacity(entries.len());
		for entry in entries {
			let mutation = mutation_identity(&entry.body);
			let (payload, encoded_bytes) = match self.insert(entry.body) {
				Ok(stored) => stored,
				Err(error) => {
					for entry in &retained {
						self.release(entry.payload);
					}
					return Err(error);
				}
			};

			retained.push(EntryRef {
				written_at_unix_ms: entry.written_at_unix_ms,
				mutation,
				id: VolumeLogPosition {
					volume_seq: entry.volume_seq,
					client_epoch: entry.client_epoch,
				},
				payload,
				encoded_bytes,
			});
		}

		Ok(retained)
	}

	/// Copy the immutable wire entries while processing the effect, before any future can be delayed.
	pub fn prepare_append(&self, effect: &VolumeClientEffect) -> anyhow::Result<PreparedAppend> {
		let VolumeClientEffect::AppendRequest {
			op_id,
			worker_id,
			volume_log_key,
			client,
			tablet_replica_set_version,
			previous,
			entries,
			committed_volume_seq,
		} = effect
		else {
			anyhow::bail!("not an append request");
		};
		Ok(PreparedAppend {
			op_id: *op_id,
			worker: worker_id.clone(),
			key: volume_log_key.clone(),
			client: client.clone(),
			tablet_replica_set_version: *tablet_replica_set_version,
			previous: previous.clone(),
			entries: entries
				.iter()
				.map(|e| self.entry(e))
				.collect::<anyhow::Result<_>>()?,
			committed: *committed_volume_seq,
		})
	}
}

/// The retry identity written to disk is replayed; payload bytes stay with the executor.
fn mutation_identity(body: &VolumeLogEntryBody) -> Option<MutationIdentity> {
	match body {
		VolumeLogEntryBody::PayloadWrite(b) => Some(MutationIdentity {
			client_seq: b.client_seq,
			request_id: b.request_id,
			payload_digest: b.payload_digest,
		}),
		_ => None,
	}
}

impl Payloads {
	/// Payload bodies still held, for tests that check releases.
	pub fn live_count(&self) -> usize {
		self.bodies.len()
	}

	/// The bodies of a creating open's initial writes, which stay retained for the core.
	pub fn initial_writes(
		&self,
		writes: &[super::state::InitialWrite],
	) -> anyhow::Result<Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>> {
		writes
			.iter()
			.map(|write| match self.bodies.get(&write.payload) {
				Some(VolumeLogEntryBody::PayloadWrite(body)) => {
					let mut body = body.clone();
					body.client_seq = write.identity.client_seq;
					body.request_id = write.identity.request_id;
					body.payload_digest = write.identity.payload_digest;
					Ok(body)
				}
				Some(_) => anyhow::bail!("an initial write is not a payload write"),
				None => anyhow::bail!("released initial write"),
			})
			.collect()
	}

	/// Hash only opaque mutation content, independently of position and retry identity.
	pub fn digest(&self, payload: PayloadId) -> anyhow::Result<PayloadDigest> {
		let body = self
			.bodies
			.get(&payload)
			.ok_or_else(|| anyhow::anyhow!("released payload"))?;
		payload_digest(body)
	}
}

/// Hash only opaque mutation content, independently of position and retry identity.
pub fn payload_digest(body: &VolumeLogEntryBody) -> anyhow::Result<PayloadDigest> {
	match body {
		VolumeLogEntryBody::PayloadWrite(write) => {
			// The digest covers the decoded bytes, so it is the same for every codec.
			let decoded = depot_types::payload_codec::decode(write)?;
			Ok(depot_types::payload_codec::payload_digest(&decoded))
		}
		VolumeLogEntryBody::ClaimEntry => anyhow::bail!("not a volume mutation"),
	}
}

/// Encoded entry size the core uses for admission, including fixed entry framing.
pub fn entry_encoded_bytes(body: &VolumeLogEntryBody) -> anyhow::Result<u32> {
	Ok(u32::try_from(body.encoded_len() + ENTRY_FRAMING_BYTES)?)
}

/// Returns true when the body is certainly larger than `max_bytes` once encoded. The opaque
/// payload alone is a lower bound on the encoding, so oversized writes are rejected before an
/// encode copies them.
pub fn entry_exceeds_bytes(body: &VolumeLogEntryBody, max_bytes: u32) -> bool {
	match body {
		VolumeLogEntryBody::PayloadWrite(b) => {
			b.payload.len().saturating_add(ENTRY_FRAMING_BYTES) > max_bytes as usize
		}
		_ => false,
	}
}

/// Fixed position and timestamp fields that surround an encoded entry body.
const ENTRY_FRAMING_BYTES: usize = 24;

/// An emitted append owns its wire bytes independently of retained history and its future's poll time.
pub struct PreparedAppend {
	op_id: OpId,
	worker: WorkerId,
	key: VolumeLogKey,
	client: ClientIdentity,
	tablet_replica_set_version: u64,
	previous: VolumeLogPosition,
	entries: Vec<VolumeLogEntry>,
	committed: u64,
}
impl PreparedAppend {
	/// The owner prepares synchronously, then may dispatch, delay, or discard this transport task safely.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn execute(self, transport: &dyn VolumeClientTransport) -> VolumeClientEvent {
		let result = transport
			.append(
				self.worker.clone(),
				self.key,
				self.client,
				self.tablet_replica_set_version,
				self.previous,
				self.entries,
				self.committed,
			)
			.await;
		VolumeClientEvent::AppendResponse {
			op_id: self.op_id,
			worker_id: self.worker,
			result,
		}
	}
}


