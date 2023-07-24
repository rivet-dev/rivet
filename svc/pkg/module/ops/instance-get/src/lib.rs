use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Instance {
	instance_id: Uuid,
	version_id: Uuid,
	create_ts: i64,
	destroy_ts: Option<i64>,

	driver_dummy: bool,

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
			i.destroy_ts,
			idd.instance_id IS NOT NULL AS driver_dummy,
			idv.instance_id IS NOT NULL AS driver_fly,
			idv.fly_app_id AS driver_fly_app_id
		FROM instances AS i
		LEFT JOIN instances_driver_dummy AS idd ON idd.instance_id = i.instance_id
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
				let driver = if instance.driver_dummy {
					backend::module::instance::Driver::Dummy(backend::module::instance::Dummy {})
				} else if instance.driver_fly {
					backend::module::instance::Driver::Fly(backend::module::instance::Fly {
						fly_app_id: instance.driver_fly_app_id,
					})
				} else {
					internal_panic!("instance has no driver")
				};

				GlobalResult::Ok(backend::module::Instance {
					instance_id: Some(instance.instance_id.into()),
					module_version_id: Some(instance.version_id.into()),
					create_ts: instance.create_ts,
					destroy_ts: instance.destroy_ts,
					driver: Some(driver),
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
