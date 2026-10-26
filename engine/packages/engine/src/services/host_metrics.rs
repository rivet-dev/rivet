//! Entry point of the host hardware metrics service.

use anyhow::Result;
use rivet_service_manager::{Service, ServiceClass};
use tokio_util::sync::CancellationToken;

/// Reports host hardware on every worker. Only a storage worker reports and probes its data
/// directories. The service holds shutdown until a running disk probe removes its file.
pub fn service(storage: bool) -> Service {
	Service::new(
		"host_metrics",
		ServiceClass::Always,
		move |config, ctx| start(config, ctx, storage),
		true,
	)
}

pub async fn start(
	config: rivet_config::Config,
	ctx: rivet_engine_context::EngineContext,
	storage: bool,
) -> Result<()> {
	let hardware = &config.metrics.hardware;
	let settings = crate::host_metrics::Settings {
		directories: if storage {
			crate::host_metrics::data_directories(config.depot())
		} else {
			Vec::new()
		},
		probe_bytes: if hardware.disk_probe() {
			Some(hardware.disk_probe_bytes())
		} else {
			None
		},
	};

	let cancel = CancellationToken::new();
	let mut shutdown = ctx.shutdown();
	let cancel_on_shutdown = async {
		if let Err(error) = shutdown.recv().await {
			tracing::warn!(%error, "shutdown signal failed; stopping host metrics");
		}
		cancel.cancel();
	};
	let env = ctx.env().clone();
	let report =
		crate::host_metrics::run(env.clone(), env.fs(), env.host(), settings, cancel.clone());
	futures_util::future::join(report, cancel_on_shutdown).await;
	Ok(())
}
