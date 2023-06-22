use anyhow::Result;
use duct::cmd;
use serde_json::json;
use std::collections::HashMap;
use tokio::task::block_in_place;

use crate::context::ProjectContext;

const API_KEY: &str = "phc_6kfTNEAVw7rn1LA51cO3D69FefbKupSWFaM7OUgEpEo";

fn build_client() -> async_posthog::Client {
	async_posthog::client(API_KEY)
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

	// Fetch event data
	let git_rev = block_in_place(|| cmd!("git", "rev-parse", "HEAD").dir(ctx.path()).read()).ok();

	let git_remotes = block_in_place(|| cmd!("git", "remote", "--verbose").dir(ctx.path()).read())
		.ok()
		.map(|x| {
			x.split("\n")
				.map(|x| x.trim())
				.filter(|x| !x.is_empty())
				.map(|x| x.to_string())
				.collect::<Vec<_>>()
		});

	let services = ctx
		.all_services()
		.await
		.iter()
		.map(|x| (x.name(), json!({})))
		.collect::<HashMap<String, serde_json::Value>>();

	let uname = block_in_place(|| cmd!("uname", "-a").read()).ok();

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

	// Add property
	event.insert_prop(
		"$set",
		&json!({
			"ns_id": ctx.ns_id(),
			"ns_config": ctx.ns(),
			"git_rev": git_rev,
			"git_remotes": git_remotes,
			"services": services,
			"uname": uname,
			"os_release": os_release,
		}),
	)?;

	Ok(event)
}

pub async fn capture_event(_ctx: &ProjectContext, event: async_posthog::Event) -> Result<()> {
	build_client().capture(event).await?;
	Ok(())
}
