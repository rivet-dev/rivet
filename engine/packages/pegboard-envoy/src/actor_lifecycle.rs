use anyhow::{Context, Result, ensure};
use gas::prelude::{Id, StandaloneCtx, util::timestamp};
use rivet_envoy_protocol as protocol;
use sqlite_storage::open::OpenConfig;

use crate::{conn::Conn, sqlite_runtime};

pub async fn start_actor(
	ctx: &StandaloneCtx,
	conn: &Conn,
	checkpoint: &protocol::ActorCheckpoint,
	start: &mut protocol::CommandStartActor,
) -> Result<()> {
	let actor_id = Id::parse(&checkpoint.actor_id).context("invalid start actor id")?;
	let actor_id_string = actor_id.to_string();

	ensure!(start.sqlite_startup_data.is_none());
	ensure!(start.preloaded_kv.is_none());

	let hibernating_requests = ctx
		.op(pegboard::ops::actor::hibernating_request::list::Input { actor_id })
		.await?;
	start.hibernating_requests = hibernating_requests
		.into_iter()
		.map(|x| protocol::HibernatingRequest {
			gateway_id: x.gateway_id,
			request_id: x.request_id,
		})
		.collect();

	let db = ctx.udb()?;
	start.preloaded_kv = pegboard::actor_kv::preload::fetch_preloaded_kv(
		&db,
		ctx.config().pegboard(),
		actor_id,
		conn.namespace_id,
		&start.config.name,
	)
	.await?;

	// Open SQLite to produce startup data for the envoy. The open is
	// fire-and-forget from the connection's perspective. The SqliteEngine's
	// takeover path on next open and the lenient `ensure_local_open` cache
	// catch-up handle ownership transitions.
	let sqlite_open = conn
		.sqlite_engine
		.open(&actor_id_string, OpenConfig::new(timestamp::now()))
		.await?;
	start.sqlite_startup_data = Some(sqlite_runtime::protocol_sqlite_startup_data(sqlite_open));

	Ok(())
}
