mod common;

use anyhow::Result;
use common::{
	THREE_REPLICAS, TestCtx,
	utils::{check_and_set_mutable, set_if_absent, set_mutable, write_v2_committed_value},
};
use epoxy::{
	keys,
	ops::kv::get::{Input, ReadMode},
};
use epoxy_protocol::protocol::{self, CommittedValue};

// Read tests begin with a stable, active scope. Membership bootstrap is covered separately;
// avoid waiting on coordinator status messages when testing quorum/read failures.
async fn cluster() -> Result<TestCtx> {
	let cluster = TestCtx::new_replica_only_with(THREE_REPLICAS).await?;
	let config = protocol::ClusterConfig {
		coordinator_replica_id: 1,
		epoch: 1,
		replicas: THREE_REPLICAS
			.iter()
			.map(|&replica_id| protocol::ReplicaConfig {
				replica_id,
				status: protocol::ReplicaStatus::Active,
				api_peer_url: cluster.api_peer_url(replica_id),
				guard_url: cluster.api_peer_url(replica_id),
			})
			.collect(),
	};
	for &replica in THREE_REPLICAS {
		cluster
			.get_ctx(replica)
			.udb()?
			.txn("test_stable_read_scope", |tx| {
				let config = config.clone();
				async move {
					tx.with_subspace(keys::subspace(replica))
						.write(&keys::ConfigKey, config)?;
					Ok(())
				}
			})
			.await?;
	}
	Ok(cluster)
}

async fn pending(
	ctx: &gas::prelude::TestCtx,
	replica: u64,
	key: &[u8],
	value: Option<Vec<u8>>,
	version: u64,
) -> Result<()> {
	ctx.udb()?
		.txn("test_pending_read_value", |tx| {
			let value = value.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(replica));
				let ballot = protocol::Ballot {
					counter: 7,
					replica_id: 1,
				};
				tx.write(&keys::KvBallotKey::new(key.to_vec()), ballot.clone())?;
				tx.write(
					&keys::KvAccepted2Key::new(key.to_vec()),
					protocol::AcceptedValue {
						value,
						version,
						mutable: true,
						ballot,
					},
				)?;
				Ok(())
			}
		})
		.await
}

fn input(key: &[u8], mode: ReadMode) -> Input {
	Input {
		key: key.to_vec(),
		mode,
	}
}
fn linearizable() -> ReadMode {
	ReadMode::Linearizable {
		target_replicas: None,
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn recovers_accepted_values_and_tombstones_with_owner_unavailable() {
	let mut cluster = cluster().await.unwrap();
	let result: Result<()> = async {
		for key in [b"accepted-value".as_slice(), b"accepted-deletion"] {
			for replica in THREE_REPLICAS {
				write_v2_committed_value(
					cluster.get_ctx(*replica),
					*replica,
					key,
					CommittedValue {
						value: Some(b"old".to_vec()),
						version: 1,
						mutable: true,
					},
				)
				.await?;
				pending(
					cluster.get_ctx(*replica),
					*replica,
					key,
					if key == b"accepted-deletion" {
						None
					} else {
						Some(b"new".to_vec())
					},
					2,
				)
				.await?;
			}
			let owner = cluster
				.get_ctx(2)
				.op(input(key, ReadMode::LocalCommitted { replica_id: 1 }))
				.await?;
			assert!(owner.pending_write);
			assert_eq!(owner.value.unwrap().version, 1);
			let stale = cluster
				.get_ctx(2)
				.op(input(key, ReadMode::LatestReachable))
				.await?;
			assert_eq!(stale.value.unwrap().version, 1);
		}
		cluster.stop_replica(1, false).await?;
		for (key, expected) in [
			(b"accepted-value".as_slice(), Some(b"new".to_vec())),
			(b"accepted-deletion".as_slice(), None),
		] {
			let recovered = cluster
				.get_ctx(2)
				.op(input(key, linearizable()))
				.await?
				.value
				.unwrap();
			assert_eq!(recovered.version, 2);
			assert_eq!(recovered.value, expected);
			let again = cluster
				.get_ctx(3)
				.op(input(key, linearizable()))
				.await?
				.value
				.unwrap();
			assert_eq!(again, recovered);
		}
		Ok(())
	}
	.await;
	cluster.shutdown().await.unwrap();
	result.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_observes_completed_write_and_linearizable_reads_require_quorum() {
	let mut cluster = cluster().await.unwrap();
	let result: Result<()> = async {
		let key = b"completed-owner-write";
		let config = cluster.get_ctx(1).config();
		let mut versions = rivet_build_meta::compiled_runtime_protocols();
		versions.epoxy.set_override_version(3);
		config.set_protocols(versions);
		assert!(
			cluster
				.get_ctx(1)
				.op(input(key, ReadMode::LocalCommitted { replica_id: 1 }))
				.await
				.is_err()
		);
		config.set_protocols(rivet_build_meta::compiled_runtime_protocols());
		set_if_absent(cluster.get_ctx(1), b"immutable", b"fixed")
			.await?
			.resolve()?;
		let immutable = cluster
			.get_ctx(2)
			.op(input(
				b"immutable",
				ReadMode::OptimisticImmutable {
					caching_behavior: protocol::CachingBehavior::Optimistic,
					target_replicas: None,
					save_empty: false,
				},
			))
			.await?
			.value
			.unwrap();
		assert_eq!(immutable.value, Some(b"fixed".to_vec()));
		assert!(!immutable.mutable);
		set_mutable(cluster.get_ctx(1), key, b"revoked")
			.await?
			.resolve()?;
		let owner = cluster
			.get_ctx(2)
			.op(input(key, ReadMode::LocalCommitted { replica_id: 1 }))
			.await?;
		assert!(!owner.pending_write);
		assert_eq!(owner.value.unwrap().value, Some(b"revoked".to_vec()));
		// Absence is also established through a promise quorum.
		assert!(
			cluster
				.get_ctx(2)
				.op(input(b"absent", linearizable()))
				.await?
				.value
				.is_none()
		);
		// A read promise must not prevent the first subsequent write.
		set_mutable(cluster.get_ctx(2), b"absent", b"created")
			.await?
			.resolve()?;
		assert_eq!(
			cluster
				.get_ctx(3)
				.op(input(b"absent", linearizable()))
				.await?
				.value
				.unwrap()
				.value,
			Some(b"created".to_vec())
		);
		cluster.stop_replica(2, false).await?;
		cluster.stop_replica(3, false).await?;
		assert!(
			cluster
				.get_ctx(1)
				.op(input(key, linearizable()))
				.await
				.is_err()
		);
		assert!(
			cluster
				.get_ctx(1)
				.op(input(key, ReadMode::LocalCommitted { replica_id: 1 }))
				.await?
				.value
				.is_some()
		);
		let interrupted = b"interrupted-cas-bootstrap";
		let failed = check_and_set_mutable(
			cluster.get_ctx(1),
			interrupted,
			vec![None],
			Some(b"next".to_vec()),
		)
		.await?;
		assert!(failed.resolve().is_err());
		// Even without a quorum, the coordinator must remember the in-flight CAS.
		assert!(
			cluster
				.get_ctx(1)
				.op(input(
					interrupted,
					ReadMode::LocalCommitted { replica_id: 1 }
				))
				.await?
				.pending_write
		);

		Ok(())
	}
	.await;
	cluster.shutdown().await.unwrap();
	result.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_divergence_and_reads_monotonically_during_writes() {
	let mut cluster = cluster().await.unwrap();
	let result: Result<()> = async {
		let divergent = b"divergent";
		for replica in THREE_REPLICAS {
			write_v2_committed_value(
				cluster.get_ctx(*replica),
				*replica,
				divergent,
				CommittedValue {
					value: Some(vec![*replica as u8]),
					version: 1,
					mutable: true,
				},
			)
			.await?;
		}
		assert!(
			cluster
				.get_ctx(1)
				.op(input(divergent, linearizable()))
				.await
				.is_err()
		);
		assert!(
			cluster
				.get_ctx(1)
				.op(input(
					b"scope",
					ReadMode::Linearizable {
						target_replicas: Some(vec![2, 3])
					}
				))
				.await
				.is_err()
		);
		let key = b"concurrent-reads";
		set_mutable(cluster.get_ctx(1), key, &[0])
			.await?
			.resolve()?;
		let writer = async {
			for i in 1..=10 {
				loop {
					if let Ok(out) = set_mutable(cluster.get_ctx(1), key, &[i]).await {
						if out.resolve().is_ok() {
							break;
						}
					}
					tokio::task::yield_now().await;
				}
			}
		};
		let reader = async {
			let mut last = 0;
			for _ in 0..10 {
				if let Ok(out) = cluster.get_ctx(2).op(input(key, linearizable())).await {
					let value = out.value.unwrap();
					assert!(value.version >= last);
					last = value.version;
				}
			}
		};
		tokio::time::timeout(std::time::Duration::from_secs(30), async {
			tokio::join!(writer, reader);
		})
		.await?;
		assert_eq!(
			cluster
				.get_ctx(3)
				.op(input(key, linearizable()))
				.await?
				.value
				.unwrap()
				.value,
			Some(vec![10])
		);
		Ok(())
	}
	.await;
	cluster.shutdown().await.unwrap();
	result.unwrap();
}
