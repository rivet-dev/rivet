use gas::prelude::*;
use rivet_key_data::converted::ActorNameKeyData;
use rivet_types::actors::CrashPolicy;
use udb_util::{SERIALIZABLE, TxnExt};

use super::State;

use crate::{errors, keys};

const MAX_INPUT_SIZE: usize = util::file_size::mebibytes(4) as usize;

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ValidateInput {
	pub namespace_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub input: Option<String>,
}

#[activity(Validate)]
pub async fn validate(
	ctx: &ActivityCtx,
	input: &ValidateInput,
) -> Result<std::result::Result<(), errors::Actor>> {
	let ns_res = ctx
		.op(namespace::ops::get_global::Input {
			namespace_id: input.namespace_id,
		})
		.await?;

	if ns_res.is_none() {
		return Ok(Err(errors::Actor::NamespaceNotFound));
	}

	if input
		.input
		.as_ref()
		.map(|x| x.len() > MAX_INPUT_SIZE)
		.unwrap_or_default()
	{
		return Ok(Err(errors::Actor::InputTooLarge {
			max_size: MAX_INPUT_SIZE,
		}));
	}

	if let Some(k) = &input.key {
		if k.is_empty() {
			return Ok(Err(errors::Actor::EmptyKey));
		}
		if k.len() > 1024 {
			return Ok(Err(errors::Actor::KeyTooLarge {
				max_size: 1024,
				key_preview: util::safe_slice(k, 0, 1024).to_string(),
			}));
		}
	}

	Ok(Ok(()))
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct InitStateAndUdbInput {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub namespace_id: Id,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,
	pub create_ts: i64,
}

#[activity(InitStateAndFdb)]
pub async fn insert_state_and_fdb(ctx: &ActivityCtx, input: &InitStateAndUdbInput) -> Result<()> {
	let mut state = ctx.state::<Option<State>>()?;

	*state = Some(State::new(
		input.name.clone(),
		input.key.clone(),
		input.namespace_id,
		input.runner_name_selector.clone(),
		input.crash_policy,
		input.create_ts,
	));

	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			txs.write(
				&keys::actor::CreateTsKey::new(input.actor_id),
				input.create_ts,
			)?;
			txs.write(
				&keys::actor::WorkflowIdKey::new(input.actor_id),
				ctx.workflow_id(),
			)?;

			Ok(())
		})
		.custom_instrument(tracing::info_span!("actor_insert_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct AddIndexesAndSetCreateCompleteInput {
	pub actor_id: Id,
}

#[activity(AddIndexesAndSetCreateComplete)]
pub async fn add_indexes_and_set_create_complete(
	ctx: &ActivityCtx,
	input: &AddIndexesAndSetCreateCompleteInput,
) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	// Set create complete
	state.create_complete_ts = Some(util::timestamp::now());

	// Populate indexes
	ctx.udb()?
		.run(|tx, _mc| {
			let namespace_id = state.namespace_id;
			let name = state.name.clone();
			let create_ts = state.create_ts;
			async move {
				let txs = tx.subspace(keys::subspace());

				// Populate indexes
				txs.write(
					&keys::ns::ActiveActorKey::new(
						namespace_id,
						name.clone(),
						create_ts,
						input.actor_id,
					),
					ctx.workflow_id(),
				)?;

				txs.write(
					&keys::ns::AllActorKey::new(
						namespace_id,
						name.clone(),
						create_ts,
						input.actor_id,
					),
					ctx.workflow_id(),
				)?;

				// Write name into namespace actor names list with empty metadata (if it doesn't already exist)
				let name_key = keys::ns::ActorNameKey::new(namespace_id, name.clone());
				if !txs.exists(&name_key, SERIALIZABLE).await? {
					txs.write(
						&name_key,
						ActorNameKeyData {
							metadata: serde_json::Map::new(),
						},
					)?;
				}

				// NOTE: keys::ns::ActorByKeyKey is written in actor_keys.rs when reserved by epoxy

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("actor_populate_indexes_tx"))
		.await?;

	Ok(())
}
