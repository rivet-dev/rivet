use gas::prelude::*;
use rivet_types::runner_configs::{RunnerConfig, RunnerConfigKind};
use universaldb::{options::MutationType, utils::IsolationLevel::*};

use crate::{errors, keys, utils::runner_config_variant};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: RunnerConfig,
}

#[operation]
pub async fn pegboard_runner_config_upsert(ctx: &OperationCtx, input: &Input) -> Result<bool> {
	struct UpsertOutput {
		endpoint_config_changed: bool,
		serverless_runner_created: bool,
	}

	let res: UpsertOutput = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(namespace::keys::subspace());

			let runner_config_key =
				keys::runner_config::DataKey::new(input.namespace_id, input.name.clone());

			// Check if config changed (for serverless, compare URL and headers)
			let output = if let Some(existing_config) =
				tx.read_opt(&runner_config_key, Serializable).await?
			{
				// Delete previous index
				tx.delete(&keys::runner_config::ByVariantKey::new(
					input.namespace_id,
					runner_config_variant(&existing_config),
					input.name.clone(),
				));

				// Check if serverless endpoint config changed
				match (&existing_config.kind, &input.config.kind) {
					(
						RunnerConfigKind::Serverless {
							url: old_url,
							headers: old_headers,
							..
						},
						RunnerConfigKind::Serverless {
							url: new_url,
							headers: new_headers,
							..
						},
					) => UpsertOutput {
						endpoint_config_changed: old_url != new_url || old_headers != new_headers,
						serverless_runner_created: false,
					},
					(RunnerConfigKind::Normal { .. }, RunnerConfigKind::Serverless { .. }) => {
						// Config type changed to serverless
						UpsertOutput {
							endpoint_config_changed: true,
							serverless_runner_created: true,
						}
					}
					_ => {
						// Not serverless
						UpsertOutput {
							endpoint_config_changed: true,
							serverless_runner_created: false,
						}
					}
				}
			} else {
				// New config
				UpsertOutput {
					endpoint_config_changed: true,
					serverless_runner_created: matches!(
						input.config.kind,
						RunnerConfigKind::Serverless { .. }
					),
				}
			};

			// Write new config
			tx.write(&runner_config_key, input.config.clone())?;
			tx.write(
				&keys::runner_config::ByVariantKey::new(
					input.namespace_id,
					runner_config_variant(&input.config),
					input.name.clone(),
				),
				input.config.clone(),
			)?;

			match &input.config.kind {
				RunnerConfigKind::Normal { .. } => {}
				RunnerConfigKind::Serverless {
					url,
					headers,
					slots_per_runner,
					..
				} => {
					// Validate url
					if let Err(err) = url::Url::parse(url) {
						return Ok(Err(errors::RunnerConfig::Invalid {
							reason: format!("invalid serverless url: {err}"),
						}));
					}

					if headers.len() > 16 {
						return Ok(Err(errors::RunnerConfig::Invalid {
							reason: "too many headers (max 16)".to_string(),
						}));
					}

					for (n, v) in headers {
						if n.len() > 128 {
							return Ok(Err(errors::RunnerConfig::Invalid {
								reason: format!("invalid header name: too long (max 128)"),
							}));
						}
						if let Err(err) = n.parse::<reqwest::header::HeaderName>() {
							return Ok(Err(errors::RunnerConfig::Invalid {
								reason: format!("invalid header name: {err}"),
							}));
						}
						if v.len() > 4096 {
							return Ok(Err(errors::RunnerConfig::Invalid {
								reason: format!("invalid header value: too long (max 4096)"),
							}));
						}
						if let Err(err) = v.parse::<reqwest::header::HeaderValue>() {
							return Ok(Err(errors::RunnerConfig::Invalid {
								reason: format!("invalid header value: {err}"),
							}));
						}
					}

					// Validate slots per runner
					if *slots_per_runner == 0 {
						return Ok(Err(errors::RunnerConfig::Invalid {
							reason: "`slots_per_runner` cannot be 0".to_string(),
						}));
					}

					// Sets desired count to 0 if it doesn't exist
					let tx = tx.with_subspace(rivet_types::keys::pegboard::subspace());
					tx.atomic_op(
						&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::new(
							input.namespace_id,
							input.name.clone(),
						),
						&0i64.to_le_bytes(),
						MutationType::Add,
					);
				}
			}

			Ok(Ok(output))
		})
		.custom_instrument(tracing::info_span!("runner_config_upsert_tx"))
		.await?
		.map_err(|err| err.build())?;

	// Bump autoscaler
	if res.serverless_runner_created {
		ctx.workflow(crate::workflows::serverless::pool::Input {
			namespace_id: input.namespace_id,
			runner_name: input.name.clone(),
		})
		.tag("runner_name", input.name.clone())
		.tag("namespace_id", input.namespace_id)
		.unique()
		.dispatch()
		.await?;
	} else if input.config.affects_autoscaler() {
		// Maybe scale it
		ctx.signal(crate::workflows::serverless::pool::BumpConfig {})
			.to_workflow::<crate::workflows::serverless::pool::Workflow>()
			.tag("namespace_id", input.namespace_id)
			.tag("runner_name", input.name.clone())
			.send()
			.await?;
	}

	Ok(res.endpoint_config_changed)
}
