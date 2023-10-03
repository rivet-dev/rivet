use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "kv-write")]
async fn worker(ctx: &OperationContext<kv::msg::write::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	let directory_str = util_kv::key_directory(&ctx.key);

	let (updated, is_deleted) = if let Some(value) = &ctx.value {
		let value = serde_json::from_slice::<serde_json::Value>(value)?;

		if !value.is_null() {
			// Write value if not null
			(
				upsert_value(
					ctx.ts(),
					&crdb,
					namespace_id,
					&ctx.key,
					value,
					directory_str,
				)
				.await?,
				false,
			)
		} else {
			// Delete value if null
			(delete_value(&crdb, namespace_id, &ctx.key).await?, true)
		}
	} else {
		// Delete value
		(delete_value(&crdb, namespace_id, &ctx.key).await?, true)
	};

	if updated {
		msg!([ctx] kv::msg::update(namespace_id, &ctx.key) {
			namespace_id: ctx.namespace_id,
			key: ctx.key.clone(),
			value: if is_deleted { None } else { ctx.value.clone() },
		})
		.await?;
	}

	Ok(())
}

async fn upsert_value(
	now: i64,
	crdb: &CrdbPool,
	namespace_id: Uuid,
	key: &str,
	value: serde_json::Value,
	directory: &str,
) -> GlobalResult<bool> {
	let query = sqlx::query(indoc!(
		"
		UPSERT INTO db_kv.kv (namespace_id, key, value, update_ts, directory)
		VALUES ($1, $2, $3, $4, $5)
		"
	))
	.bind(namespace_id)
	.bind(key)
	.bind(&value)
	.bind(now)
	.bind(directory)
	.execute(crdb)
	.await?;

	Ok(query.rows_affected() == 1)
}

async fn delete_value(crdb: &CrdbPool, namespace_id: Uuid, key: &str) -> GlobalResult<bool> {
	let query = sqlx::query(indoc!(
		"
		DELETE FROM db_kv.kv
		WHERE namespace_id = $1 AND key = $2
		"
	))
	.bind(namespace_id)
	.bind(key)
	.execute(crdb)
	.await?;

	Ok(query.rows_affected() == 1)
}
