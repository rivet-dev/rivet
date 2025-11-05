use epoxy::{
	ops::propose::{CommandError, ProposalResult},
	protocol,
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
	name: String,
	key: String,
	actor_id: Id,
) -> Result<ReserveKeyOutput> {
	let optimistic_reservation_id = ctx
		.activity(LookupKeyOptimisticInput {
			namespace_id,
			name: name.clone(),
			key: key.clone(),
		})
		.await?;

	if let Some(reservation_id) = optimistic_reservation_id {
		// Key found optimistically

		handle_existing_reservation(ctx, reservation_id, namespace_id, name, key, actor_id).await
	} else {
		// Key not found optimistically

		let new_reservation_id = ctx.activity(GenerateReservationIdInput {}).await?;

		let proposal_result = ctx
			.activity(ProposeInput {
				namespace_id,
				name: name.clone(),
				key: key.clone(),
				new_reservation_id,
				actor_id,
			})
			.await?;

		match proposal_result {
			ProposalResult::Committed => {
				let output = ctx
					.activity(ReserveActorKeyInput {
						namespace_id,
						name: name.clone(),
						key: key.clone(),
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
			ProposalResult::ConsensusFailed => {
				bail!("consensus failed")
			}
			ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
				current_value,
			}) => {
				if let Some(current_value) = current_value {
					let existing_reservation_id = keys::epoxy::ns::ReservationByKeyKey::new(
						namespace_id,
						name.clone(),
						key.clone(),
					)
					.deserialize(&current_value)?;

					handle_existing_reservation(
						ctx,
						existing_reservation_id,
						namespace_id,
						name.clone(),
						key.clone(),
						actor_id,
					)
					.await
				} else {
					bail!("unreachable: current_value should exist")
				}
			}
		}
	}
}

async fn handle_existing_reservation(
	ctx: &mut WorkflowCtx,
	reservation_id: Id,
	namespace_id: Id,
	name: String,
	key: String,
	actor_id: Id,
) -> Result<ReserveKeyOutput> {
	if reservation_id.label() == ctx.config().dc_label() {
		let output = ctx
			.activity(ReserveActorKeyInput {
				namespace_id,
				name: name.clone(),
				key: key.clone(),
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
}

#[activity(LookupKeyOptimistic)]
pub async fn lookup_key_optimistic(
	ctx: &ActivityCtx,
	input: &LookupKeyOptimisticInput,
) -> Result<Option<Id>> {
	let reservation_key = keys::epoxy::ns::ReservationByKeyKey::new(
		input.namespace_id,
		input.name.clone(),
		input.key.clone(),
	);
	let value = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: keys::subspace().pack(&reservation_key),
		})
		.await?
		.value;
	if let Some(value) = value {
		let reservation_id = reservation_key.deserialize(&value)?;
		Ok(Some(reservation_id))
	} else {
		Ok(None)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct GenerateReservationIdInput {}

#[activity(GenerateReservationId)]
pub async fn generate_reservation_id(
	ctx: &ActivityCtx,
	input: &GenerateReservationIdInput,
) -> Result<Id> {
	Ok(Id::new_v1(ctx.config().dc_label()))
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ProposeInput {
	namespace_id: Id,
	name: String,
	key: String,
	new_reservation_id: Id,
	actor_id: Id,
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
			proposal: protocol::Proposal {
				commands: vec![protocol::Command {
					kind: protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
						key: keys::subspace().pack(&reservation_key),
						expect_one_of: vec![None],
						new_value: Some(reservation_value),
					}),
				}],
			},
			purge_cache: false,
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
