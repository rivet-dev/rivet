use anyhow::{Result, bail};
use epoxy_protocol::protocol::{self, ReplicaId};
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
	use super::{QuorumType, calculate_fanout_quorum, calculate_quorum};

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

		for (n, fast, slow, all, any, fanout_fast, fanout_slow, fanout_all, fanout_any) in
			expected
		{
			assert_eq!(calculate_quorum(n, QuorumType::Fast), fast, "fast quorum for n={n}");
			assert_eq!(calculate_quorum(n, QuorumType::Slow), slow, "slow quorum for n={n}");
			assert_eq!(calculate_quorum(n, QuorumType::All), all, "all quorum for n={n}");
			assert_eq!(calculate_quorum(n, QuorumType::Any), any, "any quorum for n={n}");
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
				assert!(slow * 2 > n, "slow quorum must be a strict majority for n={n}");
				assert!(
					(2 * fast) + slow > 2 * n,
					"fast quorum must satisfy the Fast Paxos intersection invariant for n={n}"
				);
			}
		}
	}
}
