//! The cluster's dynamic automatic balancing switch. Each change writes only that field through
//! the shared conditional cluster-config update, so unrelated settings survive retries; the
//! running coordinator applies it at its next config change.

use anyhow::{Context, Result};
use clap::Subcommand;
use rivet_config::DynamicConfigUpdate;
use rivet_engine_context::EngineContext;
use serde_json::{Value, json};

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Read the durable override and this command's resolved setting.
	Show,
	/// Enable automatic balancing without restarting the coordinator.
	Enable,
	/// Disable new automatic balancing; in-flight changes and failover still finish.
	Disable,
	/// Remove the dynamic override and use each process's startup setting.
	Clear,
}

/// The dynamic config change `command` makes, or none for `show`.
fn patch(command: &Command) -> Option<DynamicConfigUpdate> {
	let value = match command {
		Command::Show => return None,
		Command::Enable => Some(true),
		Command::Disable => Some(false),
		Command::Clear => None,
	};
	Some(DynamicConfigUpdate {
		depot_balancing_enabled: Some(value),
		..Default::default()
	})
}

pub(crate) async fn execute(ctx: &EngineContext, command: Command) -> Result<Value> {
	let store = ctx.object_stores().primary.clone();
	if let Some(update) = patch(&command) {
		rivet_cluster_config::update_dynamic_config(
			ctx.env().clone(),
			store.clone(),
			ctx.config(),
			&update,
		)
		.await?;
	}
	let snapshot = rivet_cluster_config::fetch_config(store.as_ref())
		.await?
		.context("cluster config is absent")?;
	let dynamic = snapshot
		.dynamic
		.as_ref()
		.map(|bytes| serde_json::from_slice::<DynamicConfigUpdate>(bytes))
		.transpose()?
		.unwrap_or_default();
	let override_enabled = dynamic.depot_balancing_enabled.flatten();
	let effective_enabled = override_enabled.unwrap_or(ctx.config().depot().balancing.enabled);
	// The effective value is this command's view; the running coordinator's metrics show
	// whether it applied the switch.
	Ok(json!({
		"override": override_enabled,
		"effectiveEnabled": effective_enabled,
		"coordinatorObservationRequired": true,
	}))
}


