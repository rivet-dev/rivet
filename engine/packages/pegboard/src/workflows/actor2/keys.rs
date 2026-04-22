use epoxy::{
	ops::propose::{
		CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Proposal, ProposalResult,
	},
	protocol::ReplicaId,
};
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_data::converted::ActorByKeyKeyData;
use universaldb::options::StreamingMode;
use universaldb::prelude::*;

use crate::keys;

#[derive(Serialize, Deserialize)]
pub enum ReserveKeyOutput {
	Success,
	ForwardToDatacenter { dc_label: u16 },
	KeyExists { existing_actor_id: Id },
}

pub async fn reserve_key(
	ctx: &mut WorkflowCtx,
	namespace_id: Id,
	name: &str,
	key: &str,
	actor_id: Id,
	pool_name: &str,
) -> Result<ReserveKeyOutput> {
	let optimistic_reservation = ctx
		.activity(LookupKeyOptimisticInput {
			namespace_id,
			name: name.to_string(),
			key: key.to_string(),
			pool_name: pool_name.to_string(),
		})
		.await?;

	match optimistic_reservation {
		// Key found optimistically
		LookupKeyOptimisticOutput::Found(reservation_id) => {
			handle_existing_reservation(ctx, reservation_id, namespace_id, name, key, actor_id)
				.await
		}
		// Key not found optimistically
		LookupKeyOptimisticOutput::NotFound(new_reservation_id, target_replicas) => {
			if !target_replicas.contains(&ctx.config().epoxy_replica_id()) {
				let replica_id = target_replicas
					.into_iter()
					.next()
					.context("target_replicas is empty")?;
				let dc_label = u16::try_from(replica_id)?;

				return Ok(ReserveKeyOutput::ForwardToDatacenter { dc_label });
			}

			let proposal_result = ctx
				.activity(ProposeInput {
					namespace_id,
					name: name.to_string(),
					key: key.to_string(),
					new_reservation_id,
					actor_id,
					target_replicas,
				})
				.await?;

			match proposal_result {
				ProposalResult::Committed => {
					let output = ctx
						.activity(ReserveActorKeyInput {
							namespace_id,
							name: name.to_string(),
							key: key.to_string(),
							actor_id,
							create_ts: ctx.create_ts(),
						})
						.await?;
					match output {
						ReserveActorKeyOutput::Success => Ok(ReserveKeyOutput::Success),
						ReserveActorKeyOutput::ExistingActor { existing_actor_id } => {
							Ok(ReserveKeyOutput::KeyExists { existing_actor_id })
						}
					}
				}
				ProposalResult::ConsensusFailed {
					reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { current_value },
				} => {
					if let Some(current_value) = current_value {
						let existing_reservation_id = keys::epoxy::ns::ReservationByKeyKey::new(
							namespace_id,
							name.to_string(),
							key.to_string(),
						)
						.deserialize(&current_value)?;

						handle_existing_reservation(
							ctx,
							existing_reservation_id,
							namespace_id,
							name,
							key,
							actor_id,
						)
						.await
					} else {
						bail!("unreachable: current_value should exist")
					}
				}
				res => bail!("consensus failed: {res:?}"),
			}
		}
	}
}

async fn handle_existing_reservation(
	ctx: &mut WorkflowCtx,
	reservation_id: Id,
	namespace_id: Id,
	name: &str,
	key: &str,
	actor_id: Id,
) -> Result<ReserveKeyOutput> {
	if reservation_id.label() == ctx.config().dc_label() {
		let output = ctx
			.activity(ReserveActorKeyInput {
				namespace_id,
				name: name.to_string(),
				key: key.to_string(),
				actor_id,
				create_ts: ctx.create_ts(),
			})
			.await?;
		match output {
			ReserveActorKeyOutput::Success => Ok(ReserveKeyOutput::Success),
			ReserveActorKeyOutput::ExistingActor { existing_actor_id } => {
				Ok(ReserveKeyOutput::KeyExists { existing_actor_id })
			}
		}
	} else {
		Ok(ReserveKeyOutput::ForwardToDatacenter {
			dc_label: reservation_id.label(),
		})
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct LookupKeyOptimisticInput {
	namespace_id: Id,
	name: String,
	key: String,
	pool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LookupKeyOptimisticOutput {
	Found(Id),
	NotFound(Id, Vec<ReplicaId>),
}

#[activity(LookupKeyOptimistic)]
pub async fn lookup_key_optimistic(
	ctx: &ActivityCtx,
	input: &LookupKeyOptimisticInput,
) -> Result<LookupKeyOptimisticOutput> {
	let replicas = ctx
		.op(
			crate::ops::runner::list_runner_config_epoxy_replica_ids::Input {
				namespace_id: input.namespace_id,
				runner_name: input.pool_name.clone(),
			},
		)
		.await?
		.replicas;

	let reservation_key = keys::epoxy::ns::ReservationByKeyKey::new(
		input.namespace_id,
		input.name.clone(),
		input.key.clone(),
	);
	let value = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: keys::subspace().pack(&reservation_key),
			caching_behavior: epoxy::protocol::CachingBehavior::Optimistic,
			target_replicas: Some(replicas.clone()),
			save_empty: false,
		})
		.await?
		.value;
	if let Some(value) = value {
		let reservation_id = reservation_key.deserialize(&value)?;
		Ok(LookupKeyOptimisticOutput::Found(reservation_id))
	} else {
		let new_reservation_id = Id::new_v1(ctx.config().dc_label());
		Ok(LookupKeyOptimisticOutput::NotFound(
			new_reservation_id,
			replicas,
		))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ProposeInput {
	namespace_id: Id,
	name: String,
	key: String,
	new_reservation_id: Id,
	actor_id: Id,
	target_replicas: Vec<ReplicaId>,
}

#[activity(Propose)]
pub async fn propose(ctx: &ActivityCtx, input: &ProposeInput) -> Result<ProposalResult> {
	let reservation_key = keys::epoxy::ns::ReservationByKeyKey::new(
		input.namespace_id,
		input.name.clone(),
		input.key.clone(),
	);
	let reservation_value = reservation_key.serialize(input.new_reservation_id)?;

	let proposal_result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
						key: keys::subspace().pack(&reservation_key),
						expect_one_of: vec![None],
						new_value: Some(reservation_value),
					}),
				}],
			},
			mutable: false,
			purge_cache: false,
			target_replicas: Some(input.target_replicas.clone()),
		})
		.await?;

	Ok(proposal_result)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ReserveActorKeyInput {
	namespace_id: Id,
	name: String,
	key: String,
	actor_id: Id,
	create_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum ReserveActorKeyOutput {
	Success,
	ExistingActor { existing_actor_id: Id },
}

#[activity(ReserveActorKey)]
pub async fn reserve_actor_key(
	ctx: &ActivityCtx,
	input: &ReserveActorKeyInput,
) -> Result<ReserveActorKeyOutput> {
	let res = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			// Check if there are any actors that share the same key that are not destroyed
			let actor_key_subspace = keys::subspace().subspace(&keys::ns::ActorByKeyKey::subspace(
				input.namespace_id,
				input.name.clone(),
				input.key.clone(),
			));

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::Iterator,
					..(&actor_key_subspace).into()
				},
				Serializable,
			);

			while let Some(entry) = stream.try_next().await? {
				let (idx_key, data) = tx.read_entry::<keys::ns::ActorByKeyKey>(&entry)?;
				if !data.is_destroyed {
					return Ok(ReserveActorKeyOutput::ExistingActor {
						existing_actor_id: idx_key.actor_id,
					});
				}
			}

			// Write key
			tx.write(
				&keys::ns::ActorByKeyKey::new(
					input.namespace_id,
					input.name.clone(),
					input.key.clone(),
					input.create_ts,
					input.actor_id,
				),
				ActorByKeyKeyData {
					workflow_id: ctx.workflow_id(),
					is_destroyed: false,
				},
			)?;

			Ok(ReserveActorKeyOutput::Success)
		})
		.custom_instrument(tracing::info_span!("actor_reserve_key_tx"))
		.await?;

	Ok(res)
}
