use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "module-ns-instance-get")]
pub async fn handle(
	ctx: OperationContext<module::ns_instance_get::Request>,
) -> GlobalResult<module::ns_instance_get::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	let instance = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT instance_id
		FROM namespace_instances
		WHERE namespace_id = $1 AND key = $2
		"
	))
	.bind(namespace_id)
	.bind(&ctx.key)
	.fetch_optional(&ctx.crdb("db-module").await?)
	.await?;

	Ok(module::ns_instance_get::Response {
		instance: instance.map(
			|(instance_id,)| module::ns_instance_get::response::Instance {
				instance_id: Some(instance_id.into()),
			},
		),
	})
}
