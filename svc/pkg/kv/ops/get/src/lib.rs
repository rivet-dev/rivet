use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct KvPair {
	namespace_id: Uuid,
	key: String,
	value: String,
}

impl From<KvPair> for kv::get::response::Key {
	fn from(value: KvPair) -> kv::get::response::Key {
		kv::get::response::Key {
			namespace_id: Some(value.namespace_id.into()),
			key: value.key,
			value: value.value.as_bytes().to_owned(),
		}
	}
}

#[operation(name = "kv-get")]
async fn handle(ctx: OperationContext<kv::get::Request>) -> GlobalResult<kv::get::Response> {
	// Collect keys into a hashmap of <namespace id, keys>
	let mut namespace_ids = Vec::new();
	let mut key_strs = Vec::new();
	for key in &ctx.keys {
		namespace_ids.push(unwrap_ref!(key.namespace_id).as_uuid());
		key_strs.push(key.key.as_str());
	}

	let values = sql_fetch_all!(
		[ctx, KvPair]
		"
		SELECT kv.namespace_id, kv.key, kv.value::STRING
		FROM unnest($1, $2) AS q (namespace_id, key)
		INNER JOIN db_kv.kv ON kv.namespace_id = q.namespace_id AND kv.key = q.key
		",
		&namespace_ids,
		&key_strs,
	)
	.await?
	.into_iter()
	.map(Into::<kv::get::response::Key>::into)
	.collect::<Vec<_>>();

	Ok(kv::get::Response { values })
}
