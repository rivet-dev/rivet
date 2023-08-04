use std::collections::HashSet;

use futures_util::StreamExt;
use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("{{pkg}}-{{name}}").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("{{pkg}}-{{name}}");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"{{pkg}}-{{name}}".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	
	todo!();

	Ok(())
}
