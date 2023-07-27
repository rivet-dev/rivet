use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use std::convert::TryInto;

#[worker(name = "module-version-create")]
async fn worker(
	ctx: OperationContext<module::msg::version_create::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let version_id = internal_unwrap!(ctx.version_id).as_uuid();

	rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(update_db(tx, ctx.ts(), (**ctx).clone()))
	})
	.await?;

	msg!([ctx] module::msg::version_create_complete(version_id) {
		version_id: ctx.version_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "module.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": ctx.creator_user_id.map(|x| x.as_uuid()),
					"module_id": internal_unwrap!(ctx.module_id).as_uuid(),
					"module_version_id": internal_unwrap!(ctx.version_id).as_uuid(),
				}))?),
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn update_db(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	now: i64,
	msg: module::msg::version_create::Message,
) -> GlobalResult<()> {
	let version_id = internal_unwrap!(msg.version_id).as_uuid();
	let module_id = internal_unwrap!(msg.module_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO versions (version_id, module_id, create_ts, creator_user_id, major, minor, patch)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		"
	))
	.bind(version_id)
	.bind(module_id)
	.bind(now)
	.bind(msg.creator_user_id.map(|x| x.as_uuid()))
	.bind(TryInto::<i64>::try_into(msg.major)?)
	.bind(TryInto::<i64>::try_into(msg.minor)?)
	.bind(TryInto::<i64>::try_into(msg.patch)?)
	.execute(&mut *tx)
	.await?;

	match internal_unwrap!(msg.image) {
		module::msg::version_create::message::Image::Docker(docker) => {
			sqlx::query(indoc!(
				"
                INSERT INTO versions_image_docker (version_id, image_tag)
                VALUES ($1, $2)
                "
			))
			.bind(version_id)
			.bind(&docker.image_tag)
			.execute(&mut *tx)
			.await?;
		}
	}

	for script in msg.scripts {
		sqlx::query(indoc!(
			"
            INSERT INTO scripts (version_id, name, request_schema, response_schema)
            VALUES ($1, $2, $3, $4)
            "
		))
		.bind(version_id)
		.bind(&script.name)
		.bind(&script.request_schema)
		.bind(&script.response_schema)
		.execute(&mut *tx)
		.await?;

		if script.callable.is_some() {
			sqlx::query(indoc!(
				"
                INSERT INTO scripts_callable (version_id, name)
                VALUES ($1, $2)
            "
			))
			.bind(version_id)
			.bind(&script.name)
			.execute(&mut *tx)
			.await?;
		}
	}

	Ok(())
}
