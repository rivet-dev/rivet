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
		sqlx::query_as::<_, CustomHostname>(indoc!(
			"
		SELECT identifier, namespace_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE namespace_id = ANY($1) AND status = $2
		"
		))
		.bind(&namespace_ids)
		.bind(backend::cf::custom_hostname::Status::Pending as i32)
		.fetch_all(&ctx.crdb().await?)
		.await?
	} else {
		sqlx::query_as::<_, CustomHostname>(indoc!(
			"
		SELECT identifier, namespace_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE namespace_id = ANY($1)
		"
		))
		.bind(&namespace_ids)
		.fetch_all(&ctx.crdb().await?)
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
