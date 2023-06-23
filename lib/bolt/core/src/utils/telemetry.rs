use anyhow::Result;
use duct::cmd;
use serde_json::json;
use std::collections::HashMap;
use tokio::task::block_in_place;

use crate::context::ProjectContext;

// This API key is safe to hardcode. It will not change and is intended to be public.
const POSTHOG_API_KEY: &str = "phc_1lUNmul6sAdFzDK1VHXNrikCfD7ivQZSpf2yzrPvr4m";

fn build_client() -> async_posthog::Client {
	async_posthog::client(POSTHOG_API_KEY)
}

/// Builds a new PostHog event with associated data.
///
/// This is slightly expensive, so it should not be used frequently.
pub async fn build_event(ctx: &ProjectContext, name: &str) -> Result<async_posthog::Event> {
	// Build event
	//
	// We include both the cluster ID and the namespace ID in the distinct_id in case the config is
	// copied to a new namespace with a different name accidentally
	let distinct_id = format!("cluster:{}:{}", ctx.ns_id(), ctx.ns().cluster.id);
	let mut event = async_posthog::Event::new(name, &distinct_id);

	if !ctx.ns().rivet.telemetry.disable {
		// Helps us understand what version of the cluster is being used.
		let git_rev =
			block_in_place(|| cmd!("git", "rev-parse", "HEAD").dir(ctx.path()).read()).ok();

		// Helps us understand what fork of Rivet is being used.
		let git_remotes =
			block_in_place(|| cmd!("git", "remote", "--verbose").dir(ctx.path()).read())
				.ok()
				.map(|x| {
					x.split("\n")
						.map(|x| x.trim())
						.filter(|x| !x.is_empty())
						.map(|x| x.to_string())
						.collect::<Vec<_>>()
				});

		// Helps us understand what type of functionality people are adding that we need to add to
		// Rivet.
		let services = ctx
			.all_services()
			.await
			.iter()
			.map(|x| (x.name(), json!({})))
			.collect::<HashMap<String, serde_json::Value>>();

		// Helps us diagnose issues based on the host OS.
		let uname = block_in_place(|| cmd!("uname", "-a").read()).ok();

		// Helps us diagnose issues based on the host OS.
		let os_release = tokio::fs::read_to_string("/etc/os-release")
			.await
			.ok()
			.map(|x| {
				x.split("\n")
					.map(|x| x.trim())
					.filter_map(|x| x.split_once("="))
					.map(|(k, v)| (k.to_string(), v.to_string()))
					.collect::<HashMap<_, _>>()
			});

		// Add properties
		event.insert_prop(
			"$groups",
			&json!({
				"cluster_id": ctx.ns().cluster.id,
			}),
		)?;
		event.insert_prop(
			"$set",
			&json!({
				"ns_id": ctx.ns_id(),
				"cluster_id": ctx.ns().cluster.id,
				"ns_config": ctx.ns(),
				"bolt": {
					"git_rev": git_rev,
					"git_remotes": git_remotes,
					"uname": uname,
					"os_release": os_release,
					"services": services,
				},
			}),
		)?;
	}

	Ok(event)
}

pub async fn capture_event(ctx: &ProjectContext, event: async_posthog::Event) -> Result<()> {
	if !ctx.ns().rivet.telemetry.disable {
		build_client().capture(event).await?;
	}

	Ok(())
}
