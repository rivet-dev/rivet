use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Module {
	module_id: Uuid,
	name_id: String,
	team_id: Uuid,
	create_ts: i64,
	publicity: i64,
}

#[operation(name = "module-get")]
pub async fn handle(
	ctx: OperationContext<module::get::Request>,
) -> GlobalResult<module::get::Response> {
	let module_ids = ctx
		.module_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let modules = sqlx::query_as::<_, Module>(indoc!(
		"
		SELECT
			module_id,
			name_id,
			team_id,
			create_ts,
			publicity
		FROM modules
		WHERE module_id = ANY($1)
		"
	))
	.bind(module_ids)
	.fetch_all(&ctx.crdb("db-module").await?)
	.await?;

	Ok(module::get::Response {
		modules: modules
			.into_iter()
			.map(|module| backend::module::Module {
				module_id: Some(module.module_id.into()),
				name_id: module.name_id,
				team_id: Some(module.team_id.into()),
				create_ts: module.create_ts,
				publicity: module.publicity as i32,
			})
			.collect::<Vec<_>>(),
	})
}
