use anyhow::Context;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;

/// Hydrates ephemeral hibernating request ids before a start command reaches envoy.
/// Stored serverful commands keep this empty because request ids can change while
/// queued commands wait for envoy reconnect.
pub(crate) async fn hydrate_command_wrapper(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	command_wrapper: &mut protocol::CommandWrapper,
) -> Result<()> {
	if let protocol::Command::CommandStartActor(start) = &mut command_wrapper.inner {
		let actor_id =
			Id::parse(&command_wrapper.checkpoint.actor_id).context("invalid command actor id")?;
		if let Some(fence) = &start.sqlite_fence {
			let (pages, cache_misses) = depot::startup::assemble(
				depot::types::BucketId::from_gas_id(namespace_id),
				&command_wrapper.checkpoint.actor_id,
				depot::types::DatabaseBranchId::from_uuid(fence.branch_id.parse()?),
				fence.head_txid,
				fence.db_size_pages,
			)
			.await;
			start.sqlite_startup = Some(protocol::ActorSqliteStartup {
				fence: fence.clone(),
				page_limit: depot::startup::page_limit(),
				cache_misses,
				pages: pages
					.into_iter()
					.map(|(pgno, bytes)| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(bytes),
					})
					.collect(),
			});
		}
		start.hibernating_requests = ctx
			.op(pegboard::ops::actor::hibernating_request::list::Input { actor_id })
			.await?
			.into_iter()
			.map(|request| protocol::HibernatingRequest {
				gateway_id: request.gateway_id,
				request_id: request.request_id,
			})
			.collect();
	}

	Ok(())
}
