use crate::{
	heartbeat::{
		HeartbeatRead, Observation, Read, UnobservedHeartbeat, UnobservedReason, read_heartbeat,
		read_heartbeat_detail,
	},
	keys::s3::{StorageHeartbeatPrefix, WorkerHeartbeatKey},
	worker::{Worker, WorkerId, WorkerRole},
};
use anyhow::{Result, ensure};
use futures_util::StreamExt;
use object_store::ObjectStore;
use rivet_keys::keys::KeyAddress;
use std::time::Duration;

/// Storage workers are few, but a refresh must not issue one GET per listed worker at once.
const HEARTBEAT_READ_CONCURRENCY: usize = 8;

/// A listing can advance healthy peers even when one listed heartbeat cannot be read.
pub struct StorageListing {
	pub entries: Vec<Observation>,
	/// The listed storage workers whose heartbeat gave no observation and that no earlier
	/// listing observed, so their advertised versions are unknown: they may run an older
	/// binary.
	pub unobserved: Vec<UnobservedHeartbeat>,
	/// The other heartbeats this listing read without an observation: listed storage workers
	/// whose earlier observation stays in `entries`, and this process's own stateless
	/// heartbeat. While one may still be renewed, the listing cannot establish that a retained
	/// observation expired.
	pub unconfirmed: Vec<UnobservedHeartbeat>,
}

impl StorageListing {
	/// Whether some heartbeat gave no observation, so the listing alone does not prove that
	/// every retained observation is current.
	pub fn partial(&self) -> bool {
		!self.unobserved.is_empty() || !self.unconfirmed.is_empty()
	}
}

/// Lists every storage worker heartbeat. Production discovery and simulators that drive
/// the topology machine directly share this read.
///
/// A listed heartbeat whose ETag matches one in `known` is the same object version, so its
/// earlier observation is reused instead of read again. Only heartbeats that changed since
/// the last listing cost a GET.
pub async fn list_storage(
	store: &dyn ObjectStore,
	worker_registration: Option<&crate::topology_machine::types::WorkerRegistration>,
	known: &[Observation],
	clock: &dyn rivet_env::Clock,
	read_timeout_ms: u64,
) -> Result<StorageListing> {
	let listing = store.list(Some(&StorageHeartbeatPrefix::new().pack()));
	let mut reads = listing
		.map(|meta| async move {
			let meta = meta?;
			let key = WorkerHeartbeatKey::unpack(&meta.location)?;
			ensure!(
				key.role == WorkerRole::Storage,
				"storage listing returned a stateless worker"
			);

			let previous = known.iter().find(|entry| {
				entry.role == WorkerRole::Storage && entry.worker_id == key.worker_id
			});
			if let Some(entry) = previous.filter(|entry| {
				meta.e_tag
					.as_deref()
					.is_some_and(|listed| same_etag(&entry.etag, listed))
			}) {
				crate::metrics::DISCOVERY_HEARTBEAT_READS
					.with_label_values(&["reused"])
					.inc();
				return Ok((Some(entry.clone()), None));
			}

			crate::metrics::DISCOVERY_HEARTBEAT_READS
				.with_label_values(&["read"])
				.inc();
			let read = rivet_env::timeout(
				clock,
				Duration::from_millis(read_timeout_ms),
				read_heartbeat_detail(store, WorkerRole::Storage, &key.worker_id),
			)
			.await
			.unwrap_or(HeartbeatRead::Unreachable);
			let reason = match read {
				HeartbeatRead::Found(entry) => return Ok((Some(entry), None)),
				HeartbeatRead::Missing => return Ok((None, None)),
				HeartbeatRead::Unreachable => UnobservedReason::Unreadable,
				HeartbeatRead::Unusable(reason) => reason,
			};
			// A heartbeat that gave no observation is not evidence that the listed worker
			// disappeared. Keep its previous observation until a later read or expiry evidence
			// supersedes it.
			if previous.is_none() {
				crate::metrics::UNOBSERVED_WORKERS
					.with_label_values(&[reason.label()])
					.inc();
			}
			let heartbeat = UnobservedHeartbeat {
				worker_id: key.worker_id.clone(),
				reason,
			};
			Ok((previous.cloned(), Some(heartbeat)))
		})
		.buffer_unordered(HEARTBEAT_READ_CONCURRENCY);
	let mut entries = Vec::new();
	let mut unobserved = Vec::new();
	let mut unconfirmed = Vec::new();
	while let Some(entry) = reads.next().await {
		let (entry, unread) = entry?;
		if let Some(unread) = unread {
			match entry {
				Some(_) => unconfirmed.push(unread),
				None => unobserved.push(unread),
			}
		}
		if let Some(entry) = entry {
			entries.push(entry);
		}
	}
	// The local stateless heartbeat anchors store time without listing the
	// high-cardinality stateless namespace.
	if let Some(worker_registration) =
		worker_registration.filter(|r| r.role == WorkerRole::Stateless)
	{
		match rivet_env::timeout(
			clock,
			Duration::from_millis(read_timeout_ms),
			read_heartbeat(
				store,
				worker_registration.role,
				&worker_registration.identity.worker_id,
			),
		)
		.await
		.unwrap_or(Read::Unknown)
		{
			Read::Found(entry) => entries.push(entry),
			Read::Missing => {}
			Read::Unknown => {
				unconfirmed.push(UnobservedHeartbeat {
					worker_id: worker_registration.identity.worker_id.clone(),
					reason: UnobservedReason::Unreadable,
				});
				if let Some(entry) = known.iter().find(|entry| {
					entry.role == WorkerRole::Stateless
						&& entry.worker_id == worker_registration.identity.worker_id
				}) {
					entries.push(entry.clone());
				}
			}
		}
	}

	Ok(StorageListing {
		entries,
		unobserved,
		unconfirmed,
	})
}

/// Listings and reads may quote an ETag differently, so quotes are ignored.
fn same_etag(a: &str, b: &str) -> bool {
	a.trim_matches('"') == b.trim_matches('"')
}

/// Reads `id`'s heartbeat by exact key, in `first_role` first and then the other role, so
/// a caller that knows which kind of worker it addresses pays one GET. Returns the worker
/// and its heartbeat's `Last-Modified`, which bounds how long the endpoint stays a valid hint.
pub async fn lookup_worker(
	store: &dyn ObjectStore,
	id: &WorkerId,
	first_role: WorkerRole,
) -> Result<Option<(Worker, i64)>> {
	for role in roles_from(first_role) {
		match read_heartbeat(store, role, id).await {
			Read::Missing => continue,
			Read::Unknown => anyhow::bail!("worker heartbeat unreadable"),
			Read::Found(entry) => {
				if entry.heartbeat.state == crate::heartbeat::HeartbeatState::Released {
					return Ok(None);
				}
				let worker = Worker {
					zone: entry.heartbeat.zone.as_deref().map(str::to_owned),
					storage_capacity_millicores: entry.heartbeat.storage_capacity_millicores,
					id: id.clone(),
					role,
					instance_id: crate::worker::WorkerInstanceId::from_bytes(
						entry.heartbeat.instance_id,
					),
					api_peer_address: entry.heartbeat.api_peer_address.to_string(),
					versions: entry.heartbeat.versions.as_slice().into(),
				};
				return Ok(Some((worker, entry.modified_unix_ms)));
			}
		}
	}

	Ok(None)
}

/// Both roles, starting with `first`.
pub(crate) fn roles_from(first: WorkerRole) -> [WorkerRole; 2] {
	match first {
		WorkerRole::Stateless => [WorkerRole::Stateless, WorkerRole::Storage],
		WorkerRole::Storage => [WorkerRole::Storage, WorkerRole::Stateless],
	}
}

/// Reads `id`'s heartbeat by exact key, stateless role first and then storage, for a process
/// watch. `Missing` means neither role has one; any unreadable role makes it `Unknown`.
pub async fn read_watched(store: &dyn ObjectStore, id: &WorkerId) -> Read {
	for role in roles_from(WorkerRole::Stateless) {
		match read_heartbeat(store, role, id).await {
			Read::Missing => continue,
			found @ (Read::Found(_) | Read::Unknown) => return found,
		}
	}
	Read::Missing
}
