use futures_util::FutureExt;
use gas::prelude::*;
use rivet_cache::CacheKey;
use serde::{Deserialize, Serialize};
use udb_util::{SERIALIZABLE, TxnExt};
use utoipa::ToSchema;

use crate::{errors, keys, types::RunnerKind};

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
			.tag("namespace_id", input.namespace_id)
			.send()
			.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	let insert_res = ctx
		.activity(InsertFdbInput {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			display_name: input.display_name.clone(),
			create_ts: ctx.create_ts(),
		})
		.await?;

	if let Err(error) = insert_res {
		ctx.msg(Failed { error })
			.tag("namespace_id", input.namespace_id)
			.send()
			.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	ctx.msg(CreateComplete {})
		.tag("namespace_id", input.namespace_id)
		.send()
		.await?;

	// Does nothing yet
	ctx.repeat(|ctx| {
		let namespace_id = input.namespace_id;

		async move {
			let update = ctx.listen::<Update>().await?;

			let res = ctx
				.activity(UpdateInput {
					namespace_id,
					update,
				})
				.await?;

			if let Ok(update_res) = &res {
				ctx.activity(PurgeCacheInput { namespace_id }).await?;

				if update_res.bump_autoscaler {
					ctx.msg(rivet_types::msgs::pegboard::BumpOutboundAutoscaler {})
						.send()
						.await?;
				}
			}

			ctx.msg(UpdateResult {
				res: res.map(|_| ()),
			})
			.tag("namespace_id", namespace_id)
			.send()
			.await?;

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
#[derive(Debug, Clone, Hash, ToSchema)]
#[schema(as = NamespacesUpdate)]
#[serde(rename_all = "snake_case")]
pub enum Update {
	UpdateRunnerKind { runner_kind: RunnerKind },
}

#[message("namespace_update_result")]
pub struct UpdateResult {
	pub res: Result<(), errors::Namespace>,
}

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
struct InsertFdbInput {
	namespace_id: Id,
	name: String,
	display_name: String,
	create_ts: i64,
}

#[activity(InsertFdb)]
async fn insert_fdb(
	ctx: &ActivityCtx,
	input: &InsertFdbInput,
) -> Result<std::result::Result<(), errors::Namespace>> {
	ctx.udb()?
		.run(|tx, _mc| {
			let namespace_id = input.namespace_id;
			let name = input.name.clone();
			let display_name = input.display_name.clone();

			async move {
				let txs = tx.subspace(keys::subspace());

				let name_idx_key = keys::ByNameKey::new(name.clone());

				if txs.exists(&name_idx_key, SERIALIZABLE).await? {
					return Ok(Err(errors::Namespace::NameNotUnique));
				}

				txs.write(&keys::NameKey::new(namespace_id), name)?;
				txs.write(&keys::DisplayNameKey::new(namespace_id), display_name)?;
				txs.write(&keys::CreateTsKey::new(namespace_id), input.create_ts)?;
				txs.write(&keys::RunnerKindKey::new(namespace_id), RunnerKind::Custom)?;

				// Insert idx
				txs.write(&name_idx_key, namespace_id)?;

				Ok(Ok(()))
			}
		})
		.custom_instrument(tracing::info_span!("namespace_create_tx"))
		.await
		.map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct UpdateInput {
	namespace_id: Id,
	update: Update,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct UpdateOutput {
	bump_autoscaler: bool,
}

#[activity(UpdateActivity)]
async fn update(
	ctx: &ActivityCtx,
	input: &UpdateInput,
) -> Result<std::result::Result<UpdateOutput, errors::Namespace>> {
	ctx
		.udb()?
		.run(|tx, _mc| {
			let namespace_id = input.namespace_id;
			let update = input.update.clone();

			async move {
				let txs = tx.subspace(keys::subspace());

				let bump_autoscaler = match update {
					Update::UpdateRunnerKind { runner_kind } => {
						let bump_autoscaler = match &runner_kind {
							RunnerKind::Outbound {
								url,
								slots_per_runner,
								..
							} => {
								// Validate url
								if let Err(err) = url::Url::parse(url) {
									return Ok(Err(errors::Namespace::InvalidUpdate {
										reason: format!("invalid outbound url: {err}"),
									}));
								}

								// Validate slots per runner
								if *slots_per_runner == 0 {
									return Ok(Err(errors::Namespace::InvalidUpdate {
										reason: "`slots_per_runner` cannot be 0".to_string(),
									}));
								}

								true
							}
							RunnerKind::Custom => {
								// Clear outbound data
								txs.delete_key_subspace(&rivet_types::keys::pegboard::ns::OutboundDesiredSlotsKey::subspace(namespace_id));

								false
							}
						};

						txs.write(&keys::RunnerKindKey::new(namespace_id), runner_kind)?;

						bump_autoscaler
					}
				};

				Ok(Ok(UpdateOutput { bump_autoscaler }))
			}
		})
		.custom_instrument(tracing::info_span!("namespace_create_tx"))
		.await
		.map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct PurgeCacheInput {
	namespace_id: Id,
}

#[activity(PurgeCache)]
async fn purge_cache(ctx: &ActivityCtx, input: &PurgeCacheInput) -> Result<()> {
	let res = ctx
		.op(internal::ops::cache::purge_global::Input {
			base_key: "namespace.get_global".to_string(),
			keys: vec![input.namespace_id.cache_key().into()],
		})
		.await;

	if let Err(err) = res {
		tracing::error!(?err, "failed to purge global namespace cache");
	}

	Ok(())
}
