use crate::{
	Topology,
	heartbeat_gc::{Collector, Config},
};
use anyhow::Result;
use object_store::ObjectStore;
use rivet_env::Env;
use std::sync::Arc;

/// Runs the heartbeat collector until the caller drops this future. One collector per cluster
/// suffices, so the owner is an elected singleton scope (`depot_worker` hosts it under the
/// storage owner's election). Dropping this future cancels its listing, deletes and timers; an
/// already-sent conditional delete remains safe if a replacement collector starts first, and
/// nothing needs reconciling afterwards because the next pass re-lists every record.
pub async fn start(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	topology: Arc<Topology>,
	config: Config,
) -> Result<()> {
	Collector::new(store, config).run(env, topology).await
}
