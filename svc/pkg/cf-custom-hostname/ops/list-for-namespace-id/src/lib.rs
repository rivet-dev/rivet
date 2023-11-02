use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct CustomHostname {
	namespace_id: Uuid,
	identifier: Uuid,
}

#[operation(name = "cf-custom-hostname-list-for-namespace-id")]
async fn handle(
	ctx: OperationContext<cf_custom_hostname::list_for_namespace_id::Request>,
) -> GlobalResult<cf_custom_hostname::list_for_namespace_id::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let custom_hostnames = if ctx.pending_only {
		sql_fetch_all!(
			[ctx, CustomHostname]
			"
		SELECT identifier, namespace_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE namespace_id = ANY($1) AND status = $2
		",
			&namespace_ids,
			backend::cf::custom_hostname::Status::Pending as i32,
		)
		.await?
	} else {
		sql_fetch_all!(
			[ctx, CustomHostname]
			"
		SELECT identifier, namespace_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE namespace_id = ANY($1)
		",
			&namespace_ids,
		)
		.await?
	};

	Ok(cf_custom_hostname::list_for_namespace_id::Response {
		namespaces: namespace_ids
			.into_iter()
			.map(|namespace_id| {
				let identifiers = custom_hostnames
					.iter()
					.filter(|ch| ch.namespace_id == namespace_id)
					.map(|ch| ch.identifier.into())
					.collect::<Vec<_>>();

				cf_custom_hostname::list_for_namespace_id::response::Namespace {
					namespace_id: Some(namespace_id.into()),
					identifiers,
				}
			})
			.collect::<Vec<_>>(),
	})
}
