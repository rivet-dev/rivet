use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Instance {
	instance_id: Uuid,
	version_id: Uuid,
	create_ts: i64,

	driver_fly: bool,
	driver_fly_app_id: Option<String>,
}

#[operation(name = "module-instance-get")]
pub async fn handle(
	ctx: OperationContext<module::instance_get::Request>,
) -> GlobalResult<module::instance_get::Response> {
	let instance_ids = ctx
		.instance_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let instances = sqlx::query_as::<_, Instance>(indoc!(
		"
		SELECT
			i.instance_id,
			i.version_id,
			i.create_ts,
			idv.instance_id IS NOT NULL AS driver_fly,
			idv.app_id AS driver_fly_app_id
		FROM instances AS i
		LEFT JOIN instances_driver_fly AS idv ON idv.instance_id = i.instance_id
		WHERE i.instance_id = ANY($1)
		"
	))
	.bind(&instance_ids)
	.fetch_all(&ctx.crdb("db-module").await?)
	.await?;

	Ok(module::instance_get::Response {
		instances: instances
			.into_iter()
			.map(|instance| {
				req_assert!(instance.driver_fly, "instance is not a driver fly instance");
				GlobalResult::Ok(backend::module::Instance {
					instance_id: instance.instance_id.to_string(),
					version_id: instance.version_id.to_string(),
					create_ts: instance.create_ts,
					driver: Some(backend::module::instance::Driver::Fly(backend::module::instance::Fly {
						app_id: instance.driver_fly_app_id
					})
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?
	})
}
