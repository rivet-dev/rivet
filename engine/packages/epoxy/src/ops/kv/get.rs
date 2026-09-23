//! Reads expose consistency guarantees; no mode silently falls back to a weaker one.
mod linearizable;

use anyhow::{Result, bail};
use epoxy_protocol::protocol::{self, CommittedValue, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;

use crate::http_client;

#[derive(Debug, Clone)]
pub enum ReadMode {
	/// Committed state at one replica. `pending_write` prevents an owner read from hiding
	/// an unfinished proposal. This mode alone makes no freshness guarantee.
	LocalCommitted { replica_id: ReplicaId },
	/// Newest committed value among reachable replicas, possibly stale.
	LatestReachable,
	/// Resolve accepted state through consensus. Scope must match the key's write scope.
	Linearizable {
		target_replicas: Option<Vec<ReplicaId>>,
	},
	/// Only for values that never change after creation.
	OptimisticImmutable {
		caching_behavior: protocol::CachingBehavior,
		target_replicas: Option<Vec<ReplicaId>>,
		save_empty: bool,
	},
}

pub struct Input {
	pub key: Vec<u8>,
	pub mode: ReadMode,
}
impl std::fmt::Debug for Input {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Input")
			.field("key_bytes", &self.key.len())
			.field("mode", &self.mode)
			.finish()
	}
}

pub struct Output {
	pub value: Option<CommittedValue>,
	pub pending_write: bool,
}
impl std::fmt::Debug for Output {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Output")
			.field("version", &self.value.as_ref().map(|v| v.version))
			.field("pending_write", &self.pending_write)
			.finish_non_exhaustive()
	}
}

#[operation]
pub async fn epoxy_kv_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Bound the entire operation, including configuration reads and contention retries.
	tokio::time::timeout(std::time::Duration::from_secs(10), read(ctx, input)).await?
}

async fn read(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let value = match &input.mode {
		ReadMode::LocalCommitted { replica_id } => {
			let config = ctx
				.op(crate::ops::read_cluster_config::Input {})
				.await?
				.config;
			let state = read_state(ctx, &config, *replica_id, input.key.clone(), None).await?;
			return Ok(Output {
				value: state.committed,
				pending_write: state.accepted.is_some(),
			});
		}
		ReadMode::LatestReachable => {
			ctx.op(super::get_latest::Input {
				key: input.key.clone(),
			})
			.await?
			.value
		}
		ReadMode::Linearizable { target_replicas } => {
			linearizable::read(ctx, &input.key, target_replicas.as_deref()).await?
		}
		ReadMode::OptimisticImmutable {
			caching_behavior,
			target_replicas,
			save_empty,
		} => {
			// Validate before the local cache can short-circuit the scoped read.
			let config = ctx
				.op(crate::ops::read_cluster_config::Input {})
				.await?
				.config;
			crate::utils::resolve_active_quorum_members(
				&config,
				ctx.config().epoxy_replica_id(),
				target_replicas.as_deref(),
			)?;
			super::get_optimistic::read(
				ctx,
				&super::get_optimistic::Input {
					replica_id: ctx.config().epoxy_replica_id(),
					key: input.key.clone(),
					caching_behavior: caching_behavior.clone(),
					target_replicas: target_replicas.clone(),
					save_empty: *save_empty,
				},
			)
			.await?
		}
	};
	Ok(Output {
		value,
		pending_write: false,
	})
}

async fn read_state(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	key: Vec<u8>,
	ballot: Option<protocol::Ballot>,
) -> Result<protocol::KvReadStateResponse> {
	let response = tokio::time::timeout(
		std::time::Duration::from_secs(2),
		http_client::send_message(
			&ApiCtx::new_from_operation(ctx)?,
			config,
			protocol::Request {
				from_replica_id: ctx.config().epoxy_replica_id(),
				to_replica_id: replica_id,
				kind: protocol::RequestKind::KvReadStateRequest(protocol::KvReadStateRequest {
					key,
					ballot,
					epoch: config.epoch,
				}),
			},
		),
	)
	.await??;
	match response.kind {
		protocol::ResponseKind::KvReadStateResponse(state) => Ok(state),
		_ => bail!("unexpected Epoxy read-state response"),
	}
}
