use epoxy::ops::propose::{
	CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Proposal, ProposalResult,
};
use gas::prelude::*;
use universaldb::prelude::FormalKey;

use crate::{errors, keys, types::WebhookConfig, workflows};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: WebhookConfig,
}

#[operation]
pub async fn webhook_config_upsert(ctx: &OperationCtx, input: &Input) -> Result<()> {
	validate(ctx, &input.config).await?;

	let global_key = keys::GlobalDataKey::new(input.namespace_id, input.name.clone());

	let propose_res = ctx
		.op(epoxy::ops::propose::Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
						key: namespace::keys::subspace().pack(&global_key),
						expect_one_of: vec![None],
						new_value: Some(global_key.serialize(input.config.clone())?),
					}),
				}],
			},
			purge_cache: true,
			mutable: true,
			target_replicas: None,
		})
		.await?;

	match propose_res {
		ProposalResult::Committed => {}
		ProposalResult::ConsensusFailed { reason } => match reason {
			ConsensusFailedReason::ExpectedValueDoesNotMatch { .. } => {
				return Err(errors::Webhook::Conflict.build());
			}
			ConsensusFailedReason::PreparePhaseConsensusFailed => {
				bail!("epoxy propose failed: prepare phase consensus failed");
			}
			ConsensusFailedReason::AcceptPhaseConsensusFailed => {
				bail!("epoxy propose failed: accept phase consensus failed");
			}
			ConsensusFailedReason::StaleBallot => {
				bail!("epoxy propose failed: stale ballot");
			}
		},
	}

	let signal_res = ctx
		.signal(workflows::webhook::Update {
			config: input.config.clone(),
		})
		.to_workflow::<workflows::webhook::Workflow>()
		.tag("namespace_id", input.namespace_id)
		.tag("name", input.name.clone())
		.graceful_not_found()
		.send()
		.await?;

	if signal_res.is_none() {
		ctx.workflow(workflows::webhook::Input {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			config: input.config.clone(),
		})
		.tag("namespace_id", input.namespace_id)
		.tag("name", input.name.clone())
		.unique()
		.dispatch()
		.await?;
	}

	Ok(())
}

async fn validate(ctx: &OperationCtx, config: &WebhookConfig) -> Result<()> {
	let parsed_url = url::Url::parse(&config.url).map_err(|err| {
		errors::Webhook::Invalid {
			reason: format!("invalid url: {err}"),
		}
		.build()
	})?;

	let policy = rivet_pools::reqwest::outbound_policy(ctx.config()).await?;
	if let Err(reason) = policy.check_url(&parsed_url) {
		return Err(errors::Webhook::Invalid {
			reason: format!("invalid url: {reason}"),
		}
		.build());
	}

	for event_type in &config.subscriptions {
		if !event_type.is_webhook_safe() {
			return Err(errors::Webhook::EventTypeNotAllowed {
				event_type: event_type.as_str().to_string(),
			}
			.build());
		}
	}

	if config.headers.len() > 16 {
		return Err(errors::Webhook::Invalid {
			reason: "too many headers (max 16)".to_string(),
		}
		.build());
	}

	for (name, value) in &config.headers {
		if name.len() > 128 {
			return Err(errors::Webhook::Invalid {
				reason: "invalid header name: too long (max 128)".to_string(),
			}
			.build());
		}
		if let Err(err) = name.parse::<reqwest::header::HeaderName>() {
			return Err(errors::Webhook::Invalid {
				reason: format!("invalid header name: {err}"),
			}
			.build());
		}
		if value.len() > 4096 {
			return Err(errors::Webhook::Invalid {
				reason: "invalid header value: too long (max 4096)".to_string(),
			}
			.build());
		}
		if let Err(err) = value.parse::<reqwest::header::HeaderValue>() {
			return Err(errors::Webhook::Invalid {
				reason: format!("invalid header value: {err}"),
			}
			.build());
		}
	}

	Ok(())
}
