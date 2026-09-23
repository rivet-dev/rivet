use anyhow::{Result, bail};
use epoxy_protocol::protocol::{self, CommittedValue};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;

use crate::{http_client, utils};

#[derive(Debug)]
pub struct Input {
	pub key: Vec<u8>,
}

#[derive(Debug)]
pub struct Output {
	pub value: Option<CommittedValue>,
}

/// Reads every reachable active replica and returns the newest committed value.
///
/// Unlike `get_optimistic`, this waits for the bounded fanout to finish instead of returning the
/// first response. The fanout is best-effort, not a read quorum: if a newer replica is isolated,
/// this can return an older committed value. That availability-first behavior can delay revocation
/// for callers using this read; it does not prove linearizable freshness.
#[operation]
pub async fn epoxy_kv_get_latest(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let config = ctx
		.op(crate::ops::read_cluster_config::Input {})
		.await?
		.config;
	let local_replica_id = ctx.config().epoxy_replica_id();
	let replicas = utils::resolve_active_quorum_members(&config, local_replica_id, None)?;

	let local_read = tokio::time::timeout(
		crate::consts::REQUEST_TIMEOUT,
		ctx.op(super::get_local::Input {
			replica_id: local_replica_id,
			key: input.key.clone(),
		}),
	);
	let remote_reads = http_client::fanout_to_replicas(
		local_replica_id,
		&replicas,
		utils::QuorumType::All,
		|replica_id| {
			let ctx = ctx.clone();
			let config = config.clone();
			let key = input.key.clone();
			async move {
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(&ctx)?,
					&config,
					protocol::Request {
						from_replica_id: local_replica_id,
						to_replica_id: replica_id,
						kind: protocol::RequestKind::KvGetRequest(protocol::KvGetRequest {
							key,
							caching_behavior: protocol::CachingBehavior::SkipCache,
						}),
					},
				)
				.await?;
				match response.kind {
					protocol::ResponseKind::KvGetResponse(response) => Ok(response.value),
					_ => bail!("unexpected response type for KV get request"),
				}
			}
		},
	);

	let (local_read, remote_reads) = tokio::join!(local_read, remote_reads);
	let remote_reads = remote_reads?;
	let mut reached = remote_reads.len();
	let mut values = remote_reads.into_iter().flatten().collect::<Vec<_>>();
	match local_read {
		Ok(Ok(value)) => {
			reached += 1;
			values.extend(value);
		}
		Ok(Err(error)) => {
			tracing::warn!(?error, %local_replica_id, "failed reading local Epoxy replica");
		}
		Err(error) => {
			tracing::warn!(?error, %local_replica_id, "timed out reading local Epoxy replica");
		}
	}

	if reached == 0 {
		bail!("no active Epoxy replica was reachable");
	}

	Ok(Output {
		value: select_latest(values)?,
	})
}

fn select_latest(
	values: impl IntoIterator<Item = CommittedValue>,
) -> Result<Option<CommittedValue>> {
	let mut latest: Option<CommittedValue> = None;
	for candidate in values {
		let Some(current) = latest.as_ref() else {
			latest = Some(candidate);
			continue;
		};
		if candidate.version > current.version {
			latest = Some(candidate);
		} else if candidate.version == current.version
			&& (candidate.value != current.value || candidate.mutable != current.mutable)
		{
			bail!(
				"Epoxy replicas returned different committed values at version {}",
				candidate.version
			);
		}
	}
	Ok(latest)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn committed(version: u64, value: &[u8]) -> CommittedValue {
		CommittedValue {
			value: Some(value.to_vec()),
			version,
			mutable: true,
		}
	}

	#[test]
	fn selects_the_newest_reachable_version() {
		let latest = select_latest([
			committed(2, b"generation-2"),
			committed(1, b"generation-1"),
			committed(3, b"generation-3"),
		])
		.unwrap()
		.unwrap();
		assert_eq!(latest.version, 3);
		assert_eq!(latest.value.as_deref(), Some(b"generation-3".as_slice()));
	}

	#[test]
	fn rejects_divergent_values_at_the_same_version() {
		assert!(select_latest([committed(2, b"left"), committed(2, b"right")]).is_err());
	}
}
