use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use std::convert::TryInto;

#[worker(name = "module-version-create")]
async fn worker(
	ctx: &OperationContext<module::msg::version_create::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb().await?;

	let version_id = unwrap_ref!(ctx.version_id).as_uuid();

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
					"module_id": unwrap_ref!(ctx.module_id).as_uuid(),
					"module_version_id": unwrap_ref!(ctx.version_id).as_uuid(),
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
	let version_id = unwrap_ref!(msg.version_id).as_uuid();
	let module_id = unwrap_ref!(msg.module_id).as_uuid();

	sql_query!(
		[ctx, &mut **tx]
		"
		INSERT INTO db_module.versions (version_id, module_id, create_ts, creator_user_id, major, minor, patch)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		",
		version_id,
		module_id,
		now,
		msg.creator_user_id.map(|x| x.as_uuid()),
		TryInto::<i64>::try_into(msg.major)?,
		TryInto::<i64>::try_into(msg.minor)?,
		TryInto::<i64>::try_into(msg.patch)?,
	)
	.await?;

	match unwrap_ref!(msg.image) {
		module::msg::version_create::message::Image::Docker(docker) => {
			sql_query!(
				[ctx, &mut **tx]
				"
                INSERT INTO db_module.versions_image_docker (version_id, image_tag)
                VALUES ($1, $2)
                ",
				version_id,
				&docker.image_tag,
			)
			.await?;
		}
	}

	for script in msg.scripts {
		sql_query!(
			[ctx, &mut **tx]
			"
            INSERT INTO db_module.scripts (version_id, name, request_schema, response_schema)
            VALUES ($1, $2, $3, $4)
            ",
			version_id,
			&script.name,
			&script.request_schema,
			&script.response_schema,
		)
		.await?;

		if script.callable.is_some() {
			sql_query!(
				[ctx, &mut **tx]
				"
                INSERT INTO db_module.scripts_callable (version_id, name)
                VALUES ($1, $2)
            ",
				version_id,
				&script.name,
			)
			.await?;
		}
	}

	Ok(())
}
