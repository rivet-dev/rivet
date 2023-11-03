use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "module-ns-instance-get")]
pub async fn handle(
	ctx: OperationContext<module::ns_instance_get::Request>,
) -> GlobalResult<module::ns_instance_get::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	let instance = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		SELECT instance_id
		FROM db_module.namespace_instances
		WHERE namespace_id = $1 AND key = $2
		",
		namespace_id,
		&ctx.key,
	)
	.await?;

	Ok(module::ns_instance_get::Response {
		instance: instance.map(
			|(instance_id,)| module::ns_instance_get::response::Instance {
				instance_id: Some(instance_id.into()),
			},
		),
	})
}
