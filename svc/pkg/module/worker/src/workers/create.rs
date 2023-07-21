use chirp_worker::prelude::*;
use lazy_static::lazy_static;
use proto::backend::pkg::*;
use rand::{seq::IteratorRandom, Rng};
use serde_json::json;

#[worker(name = "module-create")]
async fn worker(ctx: OperationContext<module::msg::create::Message>) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let module_id = internal_unwrap!(ctx.module_id).as_uuid();
	let team_id = internal_unwrap!(ctx.team_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO modules (module_id, name_id, team_id, create_ts)
		VALUES ($1, $2, $3, $4)
		"
	))
	.bind(&module_id)
	.bind(&ctx.name_id)
	.bind(&team_id)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	msg!([ctx] module::msg::create_complete(module_id) {
		module_id: ctx.module_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "module.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": ctx.creator_user_id.map(|x| x.as_uuid()),
					"module_id": module_id,
					"name_id": ctx.name_id,
				}))?),
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}
