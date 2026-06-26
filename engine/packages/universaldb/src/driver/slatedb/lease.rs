use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use slatedb::object_store::{
	Error as ObjectStoreError, ObjectStore, ObjectStoreExt, PutMode, PutOptions, PutResult,
	UpdateVersion,
	path::Path as ObjectStorePath,
};

use super::database::SlateDbLeaseConfig;

const LEASE_OBJECT: &str = "LEADER_LEASE";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseBody {
	pub leader_id: String,
	pub epoch: u64,
	pub expires_at_ms: u64,
	pub nats_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseState {
	pub body: LeaseBody,
	version: UpdateVersion,
}

impl LeaseState {
	pub fn version(&self) -> &UpdateVersion {
		&self.version
	}
}

#[derive(Clone)]
pub struct SlateDbLease {
	object_store: Arc<dyn ObjectStore>,
	lease_path: ObjectStorePath,
	config: SlateDbLeaseConfig,
}

impl SlateDbLease {
	pub fn new(
		object_store: Arc<dyn ObjectStore>,
		db_path: ObjectStorePath,
		config: SlateDbLeaseConfig,
		) -> Self {
		SlateDbLease {
			object_store,
			lease_path: db_path.join(LEASE_OBJECT),
			config,
		}
	}

	pub fn lease_path(&self) -> &ObjectStorePath {
		&self.lease_path
	}

	pub async fn read_current(&self) -> Result<Option<LeaseState>> {
		let result = match self.object_store.get(&self.lease_path).await {
			Ok(result) => result,
			Err(ObjectStoreError::NotFound { .. }) => return Ok(None),
			Err(err) => return Err(err).context("failed to read SlateDB leader lease"),
		};
		let version = UpdateVersion {
			e_tag: result.meta.e_tag.clone(),
			version: result.meta.version.clone(),
		};
		let bytes = result
			.bytes()
			.await
			.context("failed to read SlateDB leader lease body")?;
		let body = serde_json::from_slice(&bytes).context("failed to decode SlateDB leader lease")?;

		Ok(Some(LeaseState { body, version }))
	}

	pub async fn try_acquire(
		&self,
		leader_id: impl Into<String>,
		nats_subject: impl Into<String>,
		now_ms: u64,
	) -> Result<Option<LeaseState>> {
		let leader_id = leader_id.into();
		let nats_subject = nats_subject.into();
		let current = self.read_current().await?;

		let (epoch, mode) = match current {
			Some(current) if current.body.expires_at_ms > now_ms => return Ok(None),
			Some(current) => (
				current.body.epoch.saturating_add(1),
				PutMode::Update(current.version),
			),
			None => (1, PutMode::Create),
		};

		let body = LeaseBody {
			leader_id,
			epoch,
			expires_at_ms: now_ms.saturating_add(self.config.ttl_ms),
			nats_subject,
		};
		self.write(body, mode).await
	}

	pub async fn renew(&self, state: &LeaseState, now_ms: u64) -> Result<Option<LeaseState>> {
		let mut body = state.body.clone();
		body.expires_at_ms = now_ms.saturating_add(self.config.ttl_ms);
		self.write(body, PutMode::Update(state.version.clone())).await
	}

	pub async fn release(&self, state: &LeaseState, now_ms: u64) -> Result<Option<LeaseState>> {
		let mut body = state.body.clone();
		body.expires_at_ms = now_ms;
		self.write(body, PutMode::Update(state.version.clone())).await
	}

	async fn write(&self, body: LeaseBody, mode: PutMode) -> Result<Option<LeaseState>> {
		let payload = serde_json::to_vec(&body).context("failed to encode SlateDB leader lease")?;
		let result = match self
			.object_store
			.put_opts(&self.lease_path, payload.into(), PutOptions::from(mode))
			.await
		{
			Ok(result) => result,
			Err(ObjectStoreError::AlreadyExists { .. } | ObjectStoreError::Precondition { .. }) => {
				return Ok(None);
			}
			Err(err) => return Err(err).context("failed to write SlateDB leader lease"),
		};

		Ok(Some(LeaseState {
			body,
			version: version_from_put(result),
		}))
	}
}

fn version_from_put(result: PutResult) -> UpdateVersion {
	UpdateVersion {
		e_tag: result.e_tag,
		version: result.version,
	}
}

#[cfg(test)]
mod tests {
	use slatedb::object_store::{ObjectStore, memory::InMemory, path::Path as ObjectStorePath};

	use super::*;

	fn test_lease() -> SlateDbLease {
		let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
		SlateDbLease::new(
			store,
			ObjectStorePath::from("test-db"),
			SlateDbLeaseConfig {
				ttl_ms: 100,
				heartbeat_ms: 50,
				nats_subject: None,
			},
		)
	}

	#[tokio::test]
	async fn acquire_empty_lease_writes_body() {
		let lease = test_lease();

		let state = lease
			.try_acquire("leader-a", "udb.leader-a", 1_000)
			.await
			.unwrap()
			.unwrap();

		assert_eq!(state.body.leader_id, "leader-a");
		assert_eq!(state.body.epoch, 1);
		assert_eq!(state.body.expires_at_ms, 1_100);
		assert_eq!(state.body.nats_subject, "udb.leader-a");
		assert_eq!(lease.read_current().await.unwrap().unwrap().body, state.body);
	}

	#[tokio::test]
	async fn acquire_live_lease_loses_without_overwrite() {
		let lease = test_lease();
		let state = lease
			.try_acquire("leader-a", "udb.leader-a", 1_000)
			.await
			.unwrap()
			.unwrap();

		let contender = lease
			.try_acquire("leader-b", "udb.leader-b", 1_050)
			.await
			.unwrap();

		assert!(contender.is_none());
		assert_eq!(lease.read_current().await.unwrap().unwrap().body, state.body);
	}

	#[tokio::test]
	async fn expired_lease_takeover_bumps_epoch() {
		let lease = test_lease();
		lease
			.try_acquire("leader-a", "udb.leader-a", 1_000)
			.await
			.unwrap()
			.unwrap();

		let state = lease
			.try_acquire("leader-b", "udb.leader-b", 1_101)
			.await
			.unwrap()
			.unwrap();

		assert_eq!(state.body.leader_id, "leader-b");
		assert_eq!(state.body.epoch, 2);
		assert_eq!(state.body.expires_at_ms, 1_201);
	}

	#[tokio::test]
	async fn stale_renew_fails_after_takeover() {
		let lease = test_lease();
		let stale = lease
			.try_acquire("leader-a", "udb.leader-a", 1_000)
			.await
			.unwrap()
			.unwrap();
		let current = lease
			.try_acquire("leader-b", "udb.leader-b", 1_101)
			.await
			.unwrap()
			.unwrap();

		assert!(lease.renew(&stale, 1_120).await.unwrap().is_none());
		assert_eq!(lease.read_current().await.unwrap().unwrap().body, current.body);
	}

	#[tokio::test]
	async fn renew_and_release_use_cas_version() {
		let lease = test_lease();
		let state = lease
			.try_acquire("leader-a", "udb.leader-a", 1_000)
			.await
			.unwrap()
			.unwrap();

		let renewed = lease.renew(&state, 1_050).await.unwrap().unwrap();
		assert_eq!(renewed.body.epoch, 1);
		assert_eq!(renewed.body.expires_at_ms, 1_150);

		let released = lease.release(&renewed, 1_060).await.unwrap().unwrap();
		assert_eq!(released.body.expires_at_ms, 1_060);

		let takeover = lease
			.try_acquire("leader-b", "udb.leader-b", 1_061)
			.await
			.unwrap()
			.unwrap();
		assert_eq!(takeover.body.leader_id, "leader-b");
		assert_eq!(takeover.body.epoch, 2);
	}
}
