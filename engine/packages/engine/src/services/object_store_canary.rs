//! Entry point of the `object-store-canary` command: probes the configured object storage
//! and serves the canary's metrics on the configured metrics port until shutdown.

use anyhow::{Context, Result};
use rivet_env::{Env, capabilities::shutdown::Shutdown};
use std::{sync::Arc, time::Duration};

use crate::object_store_canary::{self, Args, Labels, REQUEST_TIMEOUT};

const CLIENT_TIMEOUT_MARGIN: Duration = Duration::from_secs(5);

pub async fn start(
	env: Arc<dyn Env>,
	config: rivet_config::Config,
	args: Args,
	shutdown: Shutdown,
) -> Result<()> {
	let backend = &config.object_storage().primary;
	let store_label = args
		.store
		.clone()
		.unwrap_or_else(|| object_store_canary::backend_label(backend).to_owned());
	let labels = Labels::new(store_label, args.region.clone())?;
	// The client's own timeout sits past the canary deadline, so a slow request is
	// recorded as the canary's timeout rather than as a transport error.
	let store = rivet_engine_context::object_storage::create_canary_object_store(
		backend,
		REQUEST_TIMEOUT + CLIENT_TIMEOUT_MARGIN,
	)?;

	rivet_env::select_biased!("the canary is useless once its metrics endpoint stops",
		result = rivet_metrics_server::run_standalone(config.clone()) => {
			result.context("object-store canary metrics server stopped")
		},
		result = object_store_canary::run(env, store, labels, &args.prefix, shutdown) => result,
	)
}
