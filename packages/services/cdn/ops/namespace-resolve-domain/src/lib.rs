use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cdn-namespace-resolve-domain")]
async fn handle(
	ctx: OperationContext<cdn::namespace_resolve_domain::Request>,
) -> GlobalResult<cdn::namespace_resolve_domain::Response> {
	let namespaces = sql_fetch_all!(
		[ctx, (Uuid, String)]
		"
		SELECT namespace_id, domain
		FROM db_cdn.game_namespace_domains
		WHERE domain = ANY($1)
		",
		&ctx.domains,
	)
	.await?
	.into_iter()
	.map(
		|(namespace_id, domain)| cdn::namespace_resolve_domain::response::GameNamespace {
			namespace_id: Some(namespace_id.into()),
			domain,
		},
	)
	.collect::<Vec<_>>();

	Ok(cdn::namespace_resolve_domain::Response { namespaces })
}
