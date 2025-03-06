use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct CustomHostname {
	identifier: Uuid,
	hostname: String,
}

#[operation(name = "cf-custom-hostname-resolve-hostname")]
async fn handle(
	ctx: OperationContext<cf_custom_hostname::resolve_hostname::Request>,
) -> GlobalResult<cf_custom_hostname::resolve_hostname::Response> {
	let custom_hostnames = sql_fetch_all!(
		[ctx, CustomHostname]
		"
		SELECT
			identifier,
			hostname
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE hostname = ANY($1)
		",
		&ctx.hostnames,
	)
	.await?;

	Ok(cf_custom_hostname::resolve_hostname::Response {
		custom_hostnames: custom_hostnames
			.into_iter()
			.map(
				|ch| cf_custom_hostname::resolve_hostname::response::CustomHostname {
					hostname: ch.hostname,
					identifier: Some(ch.identifier.into()),
				},
			)
			.collect::<Vec<_>>(),
	})
}
