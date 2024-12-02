use anyhow::*;
use serde_json::json;
use sysinfo::System;
use tokio::{
	sync::{Mutex, OnceCell},
	task::JoinSet,
	time::Duration,
};
use toolchain::{meta, paths};

pub static JOIN_SET: OnceCell<Mutex<JoinSet<()>>> = OnceCell::const_new();

/// Get the global join set for telemetry futures.
async fn join_set() -> &'static Mutex<JoinSet<()>> {
	JOIN_SET
		.get_or_init(|| async { Mutex::new(JoinSet::new()) })
		.await
}

/// Waits for all telemetry events to finish.
pub async fn wait_all() {
	let mut join_set = join_set().await.lock().await;
	match tokio::time::timeout(Duration::from_secs(5), async move {
		while join_set.join_next().await.is_some() {}
	})
	.await
	{
		Result::Ok(_) => {}
		Err(_) => {
			println!("Timed out waiting for request to finish")
		}
	}
}

// This API key is safe to hardcode. It will not change and is intended to be public.
const POSTHOG_API_KEY: &str = "phc_6kfTNEAVw7rn1LA51cO3D69FefbKupSWFaM7OUgEpEo";

fn build_client() -> async_posthog::Client {
	async_posthog::client(POSTHOG_API_KEY)
}

/// Builds a new PostHog event with associated data.
///
/// This is slightly expensive, so it should not be used frequently.
pub async fn capture_event<F>(name: &str, mutate: Option<F>)
where
	F: FnOnce(&mut async_posthog::Event) -> Result<()>,
{
	let capture_res = capture_event_inner(name, mutate).await;
	if cfg!(debug_assertions) {
		if let Err(err) = capture_res {
			eprintln!("Failed to capture event in PostHog: {:?}", err);
		}
	}
}

async fn capture_event_inner<F>(name: &str, mutate: Option<F>) -> Result<()>
where
	F: FnOnce(&mut async_posthog::Event) -> Result<()>,
{
	// Check if telemetry disabled
	let (toolchain_instance_id, telemetry_disabled, api_endpoint) =
		meta::read_project(&paths::data_dir()?, |x| {
			let api_endpoint = x.cloud.as_ref().map(|cloud| cloud.api_endpoint.clone());
			(x.toolchain_instance_id, x.telemetry_disabled, api_endpoint)
		})
		.await?;

	if telemetry_disabled {
		return Ok(());
	}

	// Read project ID. If not signed in or fails to reach server, then ignore.
	let (project_id, project_name) = match toolchain::toolchain_ctx::try_load().await {
		Result::Ok(Some(ctx)) => (
			Some(ctx.project.game_id),
			Some(ctx.project.display_name.clone()),
		),
		Result::Ok(None) => (None, None),
		Err(_) => {
			// Ignore error
			(None, None)
		}
	};

	let distinct_id = format!("toolchain:{toolchain_instance_id}");

	let mut event = async_posthog::Event::new(name, &distinct_id);

	// Helps us understand what version of the CLI is being used.
	let version = json!({
		"git_sha": env!("VERGEN_GIT_SHA"),
		"git_branch": env!("VERGEN_GIT_BRANCH"),
		"build_semver": env!("CARGO_PKG_VERSION"),
		"build_timestamp": env!("VERGEN_BUILD_TIMESTAMP"),
		"build_target": env!("VERGEN_CARGO_TARGET_TRIPLE"),
		"build_debug": env!("VERGEN_CARGO_DEBUG"),
		"rustc_version": env!("VERGEN_RUSTC_SEMVER"),
	});

	// Add properties
	if let Some(project_id) = project_id {
		event.insert_prop(
			"$groups",
			&json!({
				"project_id": project_id,
			}),
		)?;
	}

	event.insert_prop(
		"$set",
		&json!({
			"name": project_name,
			"toolchain_instance_id": toolchain_instance_id,
			"api_endpoint": api_endpoint,
			"version": version,
			"project_id": project_id,
			"project_root": paths::project_root()?,
			"sys": {
				"name": System::name(),
				"kernel_version": System::kernel_version(),
				"os_version": System::os_version(),
				"host_name": System::host_name(),
				"cpu_arch": System::cpu_arch(),
			},
		}),
	)?;

	event.insert_prop("api_endpoint", api_endpoint)?;
	event.insert_prop("args", std::env::args().collect::<Vec<_>>())?;

	// Customize the event properties
	if let Some(mutate) = mutate {
		mutate(&mut event)?;
	}

	// Capture event
	join_set().await.lock().await.spawn(async move {
		match build_client().capture(event).await {
			Result::Ok(_) => {}
			Err(_) => {
				// Fail silently
			}
		}
	});

	Ok(())
}
