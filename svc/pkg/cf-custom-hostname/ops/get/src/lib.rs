use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct CustomHostname {
	identifier: Uuid,
	namespace_id: Uuid,
	hostname: String,
	challenge: Uuid,
	create_ts: i64,
	status: i32,
	subscription_id: Uuid,
}

#[operation(name = "cf-custom-hostname-get")]
async fn handle(
	ctx: OperationContext<cf_custom_hostname::get::Request>,
) -> GlobalResult<cf_custom_hostname::get::Response> {
	let identifiers = ctx
		.identifiers
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let custom_hostnames = sql_fetch_all!(
		[ctx, CustomHostname]
		"
		SELECT
			identifier,
			namespace_id,
			hostname,
			challenge,
			create_ts,
			status,
			subscription_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE identifier = ANY($1)
		",
		identifiers,
	)
	.await?;

	Ok(cf_custom_hostname::get::Response {
		custom_hostnames: custom_hostnames
			.into_iter()
			.map(|ch| backend::cf::CustomHostname {
				identifier: Some(ch.identifier.into()),
				namespace_id: Some(ch.namespace_id.into()),
				hostname: ch.hostname,
				challenge: Some(ch.challenge.into()),
				create_ts: ch.create_ts,
				status: ch.status,
				subscription_id: Some(ch.subscription_id.into()),
			})
			.collect::<Vec<_>>(),
	})
}
