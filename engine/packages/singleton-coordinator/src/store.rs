//! Conditional publication of the regional singleton lease through one typed object.

use crate::{
	keys::s3::CoordinatorLeaseKey,
	singleton_coordinator_machine::{executor::LeaseStore, types::*},
};
use object_store::{GetOptions, GetResult, ObjectStore, ObjectStoreExt, PutMode, UpdateVersion};
use rivet_data::generated::rivet_singleton_coordinator_v1 as wire;
use rivet_env::MutationGuard;
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use rivet_topology::worker::{WorkerId, WorkerIdentity, WorkerInstanceId};
use std::sync::Arc;

/// The state machine retains retry identities and reconciles every mutation by reading back.
pub struct ObjectLeaseStore {
	store: Arc<dyn ObjectStore>,
}

impl ObjectLeaseStore {
	pub fn new(store: Arc<dyn ObjectStore>) -> Self {
		Self { store }
	}

	/// Dispatch exactly once with the captured ETag. A transport failure never authorizes a rebase.
	async fn write(&self, data: wire::Data, etag: Option<String>, guard: &dyn MutationGuard) {
		let Ok(bytes) = CoordinatorLeaseKey::new().serialize(data) else {
			return;
		};
		let mode = match etag {
			Some(etag) => PutMode::Update(UpdateVersion {
				e_tag: Some(etag),
				version: None,
			}),
			None => PutMode::Create,
		};
		if !guard.may_send() {
			return;
		}
		// Cancellation and failed responses are uncertain; the core reads the exact object again.
		let _ = self
			.store
			.put_opts(
				&CoordinatorLeaseKey::new().pack(),
				bytes.into(),
				mode.into(),
			)
			.await;
	}
}

/// The outcome of an operator's conditional write, which has no state machine to read it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorWrite {
	/// The write landed on the version the operator read.
	Written,
	/// The manifest moved past the version the operator read, so nothing was written. The
	/// holder may have changed; the operator reads again before deciding.
	Superseded,
}

impl ObjectLeaseStore {
	/// Asks the holder of `observed` to step down, on the exact version the operator read. The
	/// holder announces a step-down at its next manifest read and hands over at the deadline,
	/// then its release clears the request. A request already pending is left alone. Fails
	/// when the write's outcome is unknown; the operator reads the manifest again and repeats
	/// the request, which is idempotent.
	pub async fn request_release(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
	) -> anyhow::Result<OperatorWrite> {
		anyhow::ensure!(!observed.released, "the tenure is already released");
		anyhow::ensure!(
			observed.release_request == ReleaseRequest::None,
			"a release request is already pending"
		);
		let bytes = CoordinatorLeaseKey::new().serialize(data(
			&observed.tenure,
			false,
			ReleaseRequest::Pending,
			commit_id,
			observed.step_down_ms,
		))?;
		let mode = PutMode::Update(UpdateVersion {
			e_tag: Some(observed.version.clone()),
			version: None,
		});
		match self
			.store
			.put_opts(
				&CoordinatorLeaseKey::new().pack(),
				bytes.into(),
				mode.into(),
			)
			.await
		{
			Ok(_) => Ok(OperatorWrite::Written),
			Err(
				object_store::Error::Precondition { .. }
				| object_store::Error::AlreadyExists { .. },
			) => Ok(OperatorWrite::Superseded),
			Err(error) => Err(anyhow::Error::new(error)
				.context("the release request's outcome is unknown; read the lease and retry")),
		}
	}
}

/// Validate one GET; the fixed-size lease and bounded worker identity never need a large allocation.
async fn decode(result: object_store::Result<GetResult>) -> LeaseRead {
	let key = CoordinatorLeaseKey::new();
	let result = match result {
		Ok(result) => result,
		Err(object_store::Error::NotFound { .. }) => return LeaseRead::Missing,
		Err(_) => return LeaseRead::Unknown,
	};
	let Some(version) = result.meta.e_tag.clone().filter(|value| !value.is_empty()) else {
		return LeaseRead::Unknown;
	};
	if result.meta.size > 4096 {
		return LeaseRead::Unknown;
	}
	let modified_unix_ms = result.meta.last_modified.timestamp_millis();
	let Ok(bytes) = result.bytes().await else {
		return LeaseRead::Unknown;
	};
	if bytes.len() > 4096 {
		return LeaseRead::Unknown;
	}
	let Ok(data) = key.deserialize(bytes) else {
		return LeaseRead::Unknown;
	};
	let worker_id = WorkerId::from_bytes(data.worker_id);
	if data.coordinator_generation == 0 {
		return LeaseRead::Unknown;
	}
	LeaseRead::Found(ObservedLease {
		tenure: CoordinatorTenure {
			owner: WorkerIdentity {
				worker_id,
				instance_id: WorkerInstanceId::from_bytes(data.worker_instance_id),
			},
			coordinator_generation: data.coordinator_generation,
			claim_id: data.claim_id,
		},
		released: data.released,
		version,
		release_request: match data.release_request {
			wire::ReleaseRequest::None => ReleaseRequest::None,
			wire::ReleaseRequest::Pending => ReleaseRequest::Pending,
		},
		commit_id: data.commit_id,
		modified_unix_ms,
		step_down_ms: data.step_down_ms,
	})
}

#[async_trait::async_trait]
impl LeaseStore for ObjectLeaseStore {
	async fn read(&self) -> LeaseRead {
		decode(self.store.get(&CoordinatorLeaseKey::new().pack()).await).await
	}

	async fn read_if_changed(&self, known_version: &str) -> ManifestRead {
		let options = GetOptions {
			if_none_match: Some(known_version.to_owned()),
			..Default::default()
		};
		match self
			.store
			.get_opts(&CoordinatorLeaseKey::new().pack(), options)
			.await
		{
			Err(object_store::Error::NotModified { .. }) => ManifestRead::Unchanged,
			result => ManifestRead::Read(decode(result).await),
		}
	}

	async fn acquire(&self, attempt: &ClaimAttempt, guard: &dyn MutationGuard) {
		let previous = attempt
			.expected
			.as_ref()
			.map_or(0, |lease| lease.tenure.coordinator_generation);
		if previous.checked_add(1) != Some(attempt.tenure.coordinator_generation) {
			return;
		}
		self.write(
			data(
				&attempt.tenure,
				false,
				ReleaseRequest::None,
				attempt.tenure.claim_id,
				0,
			),
			attempt.expected.as_ref().map(|lease| lease.version.clone()),
			guard,
		)
		.await;
	}

	async fn release(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		guard: &dyn MutationGuard,
	) {
		if observed.released {
			return;
		}
		self.write(
			data(&observed.tenure, true, ReleaseRequest::None, commit_id, 0),
			Some(observed.version.clone()),
			guard,
		)
		.await;
	}

	async fn step_down(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		step_down_ms: u64,
		guard: &dyn MutationGuard,
	) {
		if observed.released || step_down_ms == 0 {
			return;
		}
		self.write(
			data(
				&observed.tenure,
				false,
				observed.release_request,
				commit_id,
				step_down_ms,
			),
			Some(observed.version.clone()),
			guard,
		)
		.await;
	}

	async fn withdraw_step_down(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		guard: &dyn MutationGuard,
	) {
		// The observed version need not show the announcement: a withdrawal on it still
		// fences an announcement that has not landed yet.
		if observed.released {
			return;
		}
		self.write(
			data(
				&observed.tenure,
				false,
				observed.release_request,
				commit_id,
				0,
			),
			Some(observed.version.clone()),
			guard,
		)
		.await;
	}
}

/// Encode the same tenure on release so an absent owner can never reset the generation floor.
fn data(
	tenure: &CoordinatorTenure,
	released: bool,
	request: ReleaseRequest,
	commit_id: [u8; 16],
	step_down_ms: u64,
) -> wire::Data {
	wire::Data {
		worker_id: *tenure.owner.worker_id.as_bytes(),
		worker_instance_id: *tenure.owner.instance_id.as_bytes(),
		coordinator_generation: tenure.coordinator_generation,
		claim_id: tenure.claim_id,
		released,
		release_request: match request {
			ReleaseRequest::None => wire::ReleaseRequest::None,
			ReleaseRequest::Pending => wire::ReleaseRequest::Pending,
		},
		commit_id,
		step_down_ms,
	}
}
