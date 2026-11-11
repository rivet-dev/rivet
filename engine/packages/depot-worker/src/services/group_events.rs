//! The events poll of this storage worker; see `group_events::poll`.

use crate::group_events::{GroupCommands, GroupEventsFeed, poll::PollConfig};
use anyhow::Result;
use rivet_env::Env;
use std::sync::Arc;
use tokio::sync::watch;

pub async fn start(
	env: Arc<dyn Env>,
	commands: Arc<dyn GroupCommands>,
	feed: Arc<GroupEventsFeed>,
	config: PollConfig,
	stop: watch::Receiver<bool>,
) -> Result<()> {
	crate::group_events::poll::run(env, commands, feed, config, stop).await
}
