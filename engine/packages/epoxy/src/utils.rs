use anyhow::{Result, bail};
use epoxy_protocol::protocol::{self, ReplicaId};
use std::collections::{BTreeSet, HashSet};
use std::fmt;
use universaldb::{Transaction, utils::IsolationLevel::*};

#[derive(Clone, Copy, Debug)]
pub enum QuorumType {
	Fast,
	Slow,
	All,
	Any,
}

impl fmt::Display for QuorumType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			QuorumType::Fast => write!(f, "fast"),
			QuorumType::Slow => write!(f, "slow"),
			QuorumType::All => write!(f, "all"),
			QuorumType::Any => write!(f, "any"),
		}
	}
}

/// Use this replica list for any action that requires a quorum.
pub fn get_quorum_members(config: &protocol::ClusterConfig) -> Vec<ReplicaId> {
	config
		.replicas
		.iter()
		.filter(|r| matches!(r.status, protocol::ReplicaStatus::Active))
		.map(|r| r.replica_id)
		.collect()
}

/// Returns the quorum members to use for a given kv operation. This supports scoped operations because, for
/// example, runner configs are often only enabled in a couple of explicitly coupled regions. If
/// `target_replicas` is provided, it validates that the scope is non-empty, includes the local
/// replica, and only contains active replicas. Otherwise it falls back to the full active quorum
/// from the cluster config.
///
/// Scoped proposals rely on a caller-side invariant: the same key must continue using the same
/// replica scope unless some higher-level reconfiguration step coordinates the membership change.
/// This function does not persist or enforce that per-key scope stability.
pub fn resolve_active_quorum_members(
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	target_replicas: Option<&[ReplicaId]>,
) -> Result<Vec<ReplicaId>> {
	match target_replicas {
		Some(target_replicas) => {
			let active = get_quorum_members(config)
				.into_iter()
				.collect::<HashSet<_>>();
			let validated = target_replicas.iter().copied().collect::<BTreeSet<_>>();

			if validated.is_empty() {
				bail!("target_replicas cannot be empty");
			}

			if !validated.contains(&replica_id) {
				bail!("target_replicas must include the local replica");
			}

			if !validated.iter().all(|replica| active.contains(replica)) {
				bail!("target_replicas contains an inactive or unknown replica");
			}

			Ok(validated.into_iter().collect())
		}
		None => {
			let replicas = get_quorum_members(config);
			if !replicas.contains(&replica_id) {
				bail!("local replica is not active in the current epoxy config");
			}
			Ok(replicas)
		}
	}
}

/// Use this replica list for any action that should still be sent to joining replicas.
pub fn get_all_replicas(config: &protocol::ClusterConfig) -> Vec<ReplicaId> {
	config.replicas.iter().map(|r| r.replica_id).collect()
}

fn calculate_slow_quorum_size(n: usize) -> usize {
	match n {
		0 => 0,
		_ => (n / 2) + 1,
	}
}

fn calculate_fast_quorum_size(n: usize) -> usize {
	match n {
		0 => 0,
		1 => 1,
		_ => {
			let slow_quorum = calculate_slow_quorum_size(n);
			n - ((slow_quorum - 1) / 2)
		}
	}
}

pub fn calculate_quorum(n: usize, q: QuorumType) -> usize {
	match q {
		QuorumType::Fast => calculate_fast_quorum_size(n),
		QuorumType::Slow => calculate_slow_quorum_size(n),
		QuorumType::All => n,
		QuorumType::Any => usize::from(n > 0),
	}
}

/// Calculates quorum size assuming the sender is excluded.
pub fn calculate_fanout_quorum(n: usize, q: QuorumType) -> usize {
	match q {
		QuorumType::Any => usize::from(n > 0),
		_ => calculate_quorum(n, q).saturating_sub(1),
	}
}

pub async fn read_config(
	tx: &Transaction,
	replica_id: ReplicaId,
) -> Result<protocol::ClusterConfig> {
	use universaldb::utils::FormalKey;

	let config_key = crate::keys::replica::ConfigKey;
	let subspace = crate::keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);

	match tx.get(&packed_key, Serializable).await? {
		Some(value) => Ok(config_key.deserialize(&value)?),
		None => bail!(
			"replica {} has not been configured yet, verify that the coordinator has reconfigured the cluster for this replica successfully",
			replica_id
		),
	}
}

#[cfg(test)]
mod tests {
	use super::{
		QuorumType, calculate_fanout_quorum, calculate_quorum, resolve_active_quorum_members,
	};
	use epoxy_protocol::protocol::{ClusterConfig, ReplicaConfig, ReplicaId, ReplicaStatus};

	#[test]
	fn quorum_sizes_match_expected_values_for_small_clusters() {
		let expected = [
			(1, 1, 1, 1, 1, 0, 0, 0, 1),
			(2, 2, 2, 2, 1, 1, 1, 1, 1),
			(3, 3, 2, 3, 1, 2, 1, 2, 1),
			(4, 3, 3, 4, 1, 2, 2, 3, 1),
			(5, 4, 3, 5, 1, 3, 2, 4, 1),
			(6, 5, 4, 6, 1, 4, 3, 5, 1),
			(7, 6, 4, 7, 1, 5, 3, 6, 1),
		];

		for (n, fast, slow, all, any, fanout_fast, fanout_slow, fanout_all, fanout_any) in expected
		{
			assert_eq!(
				calculate_quorum(n, QuorumType::Fast),
				fast,
				"fast quorum for n={n}"
			);
			assert_eq!(
				calculate_quorum(n, QuorumType::Slow),
				slow,
				"slow quorum for n={n}"
			);
			assert_eq!(
				calculate_quorum(n, QuorumType::All),
				all,
				"all quorum for n={n}"
			);
			assert_eq!(
				calculate_quorum(n, QuorumType::Any),
				any,
				"any quorum for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Fast),
				fanout_fast,
				"fanout fast quorum for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Slow),
				fanout_slow,
				"fanout slow quorum for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::All),
				fanout_all,
				"fanout all quorum for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Any),
				fanout_any,
				"fanout any quorum for n={n}"
			);
		}
	}

	#[test]
	fn quorum_invariants_hold_for_small_clusters() {
		for n in 1..=7 {
			let fast = calculate_quorum(n, QuorumType::Fast);
			let slow = calculate_quorum(n, QuorumType::Slow);
			let all = calculate_quorum(n, QuorumType::All);
			let any = calculate_quorum(n, QuorumType::Any);

			assert_eq!(all, n, "all quorum should include every replica for n={n}");
			assert_eq!(any, 1, "any quorum should require one replica for n={n}");
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Fast),
				fast.saturating_sub(1),
				"fanout fast quorum should exclude the sender for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Slow),
				slow.saturating_sub(1),
				"fanout slow quorum should exclude the sender for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::All),
				all.saturating_sub(1),
				"fanout all quorum should exclude the sender for n={n}"
			);
			assert_eq!(
				calculate_fanout_quorum(n, QuorumType::Any),
				1,
				"fanout any quorum should continue to target one remote response for n={n}"
			);

			if n >= 2 {
				assert!(
					slow * 2 > n,
					"slow quorum must be a strict majority for n={n}"
				);
				assert!(
					(2 * fast) + slow > 2 * n,
					"fast quorum must satisfy the Fast Paxos intersection invariant for n={n}"
				);
			}
		}
	}

	fn make_config(replicas: &[(ReplicaId, ReplicaStatus)]) -> ClusterConfig {
		ClusterConfig {
			coordinator_replica_id: replicas[0].0,
			epoch: 1,
			replicas: replicas
				.iter()
				.map(|(replica_id, status)| ReplicaConfig {
					replica_id: *replica_id,
					status: status.clone(),
					api_peer_url: String::new(),
					guard_url: String::new(),
				})
				.collect(),
		}
	}

	#[test]
	fn resolve_active_quorum_members_none_uses_all_active() {
		let config = make_config(&[
			(1, ReplicaStatus::Active),
			(2, ReplicaStatus::Active),
			(3, ReplicaStatus::Joining),
		]);
		let result = resolve_active_quorum_members(&config, 1, None).unwrap();
		assert_eq!(result, vec![1, 2]);
	}

	#[test]
	fn resolve_active_quorum_members_requires_local_replica_to_be_active() {
		let config = make_config(&[(1, ReplicaStatus::Learning), (2, ReplicaStatus::Active)]);
		let result = resolve_active_quorum_members(&config, 1, None);
		assert!(result.is_err());
	}

	#[test]
	fn resolve_active_quorum_members_scoped_subset() {
		let config = make_config(&[
			(1, ReplicaStatus::Active),
			(2, ReplicaStatus::Active),
			(3, ReplicaStatus::Active),
		]);
		let result = resolve_active_quorum_members(&config, 1, Some(&[1, 2])).unwrap();
		assert_eq!(result, vec![1, 2]);
	}

	#[test]
	fn resolve_active_quorum_members_empty_target_errors() {
		let config = make_config(&[(1, ReplicaStatus::Active)]);
		let result = resolve_active_quorum_members(&config, 1, Some(&[]));
		assert!(result.is_err());
	}

	#[test]
	fn resolve_active_quorum_members_missing_local_errors() {
		let config = make_config(&[(1, ReplicaStatus::Active), (2, ReplicaStatus::Active)]);
		let result = resolve_active_quorum_members(&config, 1, Some(&[2]));
		assert!(result.is_err());
	}

	#[test]
	fn resolve_active_quorum_members_inactive_replica_errors() {
		let config = make_config(&[(1, ReplicaStatus::Active), (2, ReplicaStatus::Learning)]);
		let result = resolve_active_quorum_members(&config, 1, Some(&[1, 2]));
		assert!(result.is_err());
	}
}
