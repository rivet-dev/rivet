use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub display_name: String,
}

#[workflow]
pub async fn namespace(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	let validation_res = ctx
		.activity(ValidateInput {
			name: input.name.clone(),
			display_name: input.display_name.clone(),
		})
		.await?;

	if let Err(error) = validation_res {
		ctx.msg(Failed { error })
			.topic(("namespace_id", input.namespace_id))
			.send()
			.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	let insert_res = ctx
		.activity(InsertDbInput {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			display_name: input.display_name.clone(),
			create_ts: ctx.create_ts(),
		})
		.await?;

	if let Err(error) = insert_res {
		ctx.msg(Failed { error })
			.topic(("namespace_id", input.namespace_id))
			.send()
			.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	ctx.msg(CreateComplete {})
		.topic(("namespace_id", input.namespace_id))
		.send()
		.await?;

	// Does nothing yet
	ctx.repeat(|ctx| {
		async move {
			ctx.listen::<Update>().await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[message("namespace_create_complete")]
pub struct CreateComplete {}

#[message("namespace_failed")]
pub struct Failed {
	pub error: errors::Namespace,
}

#[signal("namespace_update")]
pub struct Update {}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ValidateInput {
	pub name: String,
	pub display_name: String,
}

#[activity(Validate)]
pub async fn validate(
	ctx: &ActivityCtx,
	input: &ValidateInput,
) -> Result<std::result::Result<(), errors::Namespace>> {
	if !ctx.config().is_leader() {
		return Ok(Err(errors::Namespace::NotLeader));
	}

	if input.name.is_empty() {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "name too short".to_string(),
		}));
	}

	if input.name.len() > util::check::MAX_IDENT_LEN {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "name too long".to_string(),
		}));
	}

	if !util::check::ident(&input.name) {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "invalid name".to_string(),
		}));
	}

	if input.display_name.is_empty() {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "display name too short".to_string(),
		}));
	}

	if input.display_name.len() > util::check::MAX_DISPLAY_NAME_LONG_LEN {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "display name too long".to_string(),
		}));
	}

	if !util::check::display_name_long(&input.display_name) {
		return Ok(Err(errors::Namespace::FailedToCreate {
			reason: "invalid display name".to_string(),
		}));
	}

	Ok(Ok(()))
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	namespace_id: Id,
	name: String,
	display_name: String,
	create_ts: i64,
}

#[activity(InsertDb)]
async fn insert_db(
	ctx: &ActivityCtx,
	input: &InsertDbInput,
) -> Result<std::result::Result<(), errors::Namespace>> {
	ctx.udb()?
		.run(|tx| {
			let namespace_id = input.namespace_id;
			let name = input.name.clone();
			let display_name = input.display_name.clone();

			async move {
				let tx = tx.with_subspace(keys::subspace());

				let name_idx_key = keys::ByNameKey::new(name.clone());

				if tx.exists(&name_idx_key, Serializable).await? {
					return Ok(Err(errors::Namespace::NameNotUnique));
				}

				tx.write(&keys::NameKey::new(namespace_id), name)?;
				tx.write(&keys::DisplayNameKey::new(namespace_id), display_name)?;
				tx.write(&keys::CreateTsKey::new(namespace_id), input.create_ts)?;

				// Insert idx
				tx.write(&name_idx_key, namespace_id)?;

				Ok(Ok(()))
			}
		})
		.custom_instrument(tracing::info_span!("namespace_create_tx"))
		.await
		.map_err(Into::into)
}
