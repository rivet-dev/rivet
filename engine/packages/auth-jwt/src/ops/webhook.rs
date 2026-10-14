//! Leader-owned, linearizable namespace key-ring reads and compare-and-set mutations.
use crate::webhook::{Error, Ring};
use anyhow::{Context, Result, ensure};
use gas::prelude::*;

pub(crate) fn key(namespace_id: Id) -> Vec<u8> {
	crate::storage_keys::subspace().pack(&("webhook", namespace_id))
}

#[derive(Clone)]
pub struct Snapshot {
	pub ring: Ring,
	pub encoded: Vec<u8>,
	pub read_started: std::time::Instant,
}
impl std::fmt::Debug for Snapshot {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WebhookSnapshot")
			.field("ring", &self.ring)
			.finish_non_exhaustive()
	}
}

pub mod get {
	use super::*;
	#[derive(Debug)]
	pub struct Input {
		pub namespace_id: Id,
	}
	#[operation]
	pub async fn auth_webhook_get(ctx: &OperationCtx, input: &Input) -> Result<Option<Snapshot>> {
		ensure!(
			ctx.config().is_leader(),
			"webhook key reads are leader-only"
		);
		let read_started = std::time::Instant::now();
		let value = ctx
			.op(epoxy::ops::kv::get::Input {
				key: key(input.namespace_id),
				mode: epoxy::ops::kv::get::ReadMode::Linearizable {
					target_replicas: None,
				},
			})
			.await?
			.value;
		ensure!(
			read_started.elapsed() < crate::SIGNING_CACHE_LEASE,
			"webhook key read expired"
		);
		value
			.map(|value| {
				ensure!(value.mutable, "webhook key record must be mutable");
				let encoded = value.value.context("webhook key record was deleted")?;
				let ring = Ring::decode(input.namespace_id, &encoded)?;
				Ok(Snapshot {
					ring,
					encoded,
					read_started,
				})
			})
			.transpose()
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
	Ensure,
	Reconcile,
	ReconcileTopology,
	Rotate {
		expected_generation: u64,
	},
	Revoke {
		request_id: [u8; 16],
		expected_generation: u64,
		kids: Vec<String>,
	},
}

pub mod mutate {
	use super::*;
	#[derive(Debug)]
	pub struct Input {
		pub namespace_id: Id,
		pub change: Change,
	}

	#[operation]
	pub async fn auth_webhook_mutate(ctx: &OperationCtx, input: &Input) -> Result<Ring> {
		ensure!(
			ctx.config().is_leader(),
			"webhook key mutations are leader-only"
		);
		ensure!(
			ctx.config().auth_required()?.jwt.issuance_enabled()
				|| matches!(
					input.change,
					Change::Revoke { .. } | Change::Reconcile | Change::ReconcileTopology
				),
			"signing disabled"
		);
		if !ctx
			.op(namespace::ops::exists_authoritative::Input {
				namespace_id: input.namespace_id,
			})
			.await?
		{
			return Err(Error::NotFound.into());
		}
		let dc = ctx.config().dc_label();
		let config_generation = ctx.config().auth_required()?.jwt.issuer_generation();
		for _ in 0..16 {
			let snapshot = ctx
				.op(get::Input {
					namespace_id: input.namespace_id,
				})
				.await?;
			let now = rivet_util::timestamp::now();
			let successor = match snapshot.as_ref() {
				None if matches!(input.change, Change::Ensure) => {
					Ring::bootstrap(input.namespace_id, dc, config_generation, now)?
				}
				None => return Err(Error::NotFound.into()),
				Some(snapshot) => {
					let ring = &snapshot.ring;
					match &input.change {
						Change::Ensure | Change::Reconcile | Change::ReconcileTopology => {
							match if matches!(input.change, Change::ReconcileTopology) {
								ring.reconcile_topology(dc, config_generation, now)?
							} else {
								ring.reconcile(dc, config_generation, now)?
							} {
								Some(successor) => successor,
								None => return Ok(ring.clone()),
							}
						}
						Change::Rotate {
							expected_generation,
						} => {
							ensure!(
								ring.keys.leader_datacenter_id == dc
									&& ring.keys.leader_config_generation == config_generation,
								"webhook topology is not reconciled"
							);
							ring.rotate(*expected_generation, now)?
						}
						Change::Revoke {
							request_id,
							expected_generation,
							kids,
						} => {
							ensure!(
								ring.keys.leader_datacenter_id == dc
									&& ring.keys.leader_config_generation == config_generation,
								"webhook topology is not reconciled"
							);
							ring.revoke(*request_id, *expected_generation, kids, now)?
						}
					}
				}
			};
			if snapshot.as_ref().is_some_and(|s| s.ring == successor) {
				return Ok(successor);
			}
			if let Some(previous) = &snapshot {
				previous.ring.keys.validate_successor(&successor.keys)?;
			}
			let encoded = successor.encode()?;
			let result = ctx
				.op(epoxy::ops::propose::Input {
					proposal: epoxy::ops::propose::Proposal {
						commands: vec![epoxy::ops::propose::Command {
							kind: epoxy::ops::propose::CommandKind::CheckAndSetCommand(
								epoxy::ops::propose::CheckAndSetCommand {
									key: key(input.namespace_id),
									expect_one_of: vec![
										snapshot.as_ref().map(|s| s.encoded.clone()),
									],
									new_value: Some(encoded),
								},
							),
						}],
					},
					mutable: true,
					purge_cache: true,
					target_replicas: None,
				})
				.await?;
			match result {
				epoxy::ops::propose::ProposalResult::Committed => {
					if !matches!(
						input.change,
						Change::Ensure | Change::Reconcile | Change::ReconcileTopology
					) {
						return Ok(successor);
					}
				}
				epoxy::ops::propose::ProposalResult::ConsensusFailed {
					reason:
						epoxy::ops::propose::ConsensusFailedReason::ExpectedValueDoesNotMatch { .. },
				} => {}
				_ if matches!(
					input.change,
					Change::Ensure | Change::Reconcile | Change::ReconcileTopology
				) =>
				{
					// The rotation worker may preempt first-use admission. A fresh linearizable
					// read resolves accepted state before this idempotent reconciliation retries.
					// Explicit operator mutations preserve uncertain-commit errors instead.
					tokio::time::sleep(std::time::Duration::from_millis(5)).await;
				}
				_ => return Err(Error::Unavailable.into()),
			}
		}
		Err(Error::Unavailable.into())
	}
}

/// Dispatch first so an interrupted successful provision never lacks an idle rotation worker.
pub async fn ensure_workflow(ctx: &StandaloneCtx, namespace_id: Id) -> Result<()> {
	ctx.op(ensure_rotation::Input { namespace_id }).await
}
pub mod ensure_rotation {
	use super::*;
	#[derive(Debug)]
	pub struct Input {
		pub namespace_id: Id,
	}
	#[operation]
	pub async fn auth_webhook_ensure_rotation(ctx: &OperationCtx, input: &Input) -> Result<()> {
		ensure!(
			ctx.config().is_leader(),
			"webhook workflows are leader-owned"
		);
		ctx.workflow(crate::workflows::webhook_rotation::Input {
			namespace_id: input.namespace_id,
		})
		.tag("namespace_id", input.namespace_id)
		.unique()
		.dispatch()
		.await?;
		Ok(())
	}
}
