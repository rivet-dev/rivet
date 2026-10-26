//! Always-scoped config follower. S3 is authoritative and periodic refresh picks up changes.
use crate::{
	cluster_config_machine::types::{Purpose, Settings},
	runtime::ConfigRuntime,
};
use anyhow::Result;
use rivet_config::Config;

/// Follows the stored cluster config by periodic refresh until process shutdown.
#[tracing::instrument(skip_all)]
pub async fn start(config: Config, ctx: rivet_engine_context::EngineContext) -> Result<()> {
	let runtime = ConfigRuntime::start(
		ctx.env().clone(),
		ctx.object_stores().primary.clone(),
		config,
		Purpose::Follow,
		Settings::default(),
	)?;
	let result = async {
        let mut snapshots = runtime.subscribe();
        let mut shutdown = ctx.shutdown();
        loop {
            rivet_env::select_biased!("process shutdown closes the config follower before it reads another snapshot",
                result = shutdown.recv() => {result?; return Ok(());},
                changed = snapshots.changed() => {
                    changed?;
                    let snapshot = snapshots.borrow_and_update();
                    if let Some(error) = &snapshot.diagnostic {
                        tracing::warn!(%error, "cluster config refresh failed; retaining last valid settings");
                    }
                    if let Some(result) = &snapshot.result {return result.clone().map_err(anyhow::Error::msg);}
                },
            );
        }
    }.await;
	let closed = runtime.close().await;
	result.and(closed)
}
