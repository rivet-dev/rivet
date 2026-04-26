use gas::prelude::*;

use crate::actor_sqlite;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MigrateSqliteV1ToV2Input {
	pub actor_id: Id,
	pub namespace_id: Id,
	pub name: String,
	pub protocol_version: u16,
}

#[activity(MigrateSqliteV1ToV2)]
pub async fn migrate_sqlite_v1_to_v2(
	ctx: &ActivityCtx,
	input: &MigrateSqliteV1ToV2Input,
) -> Result<actor_sqlite::MigrateV1ToV2Output> {
	let udb = ctx.udb()?;
	let db = (*udb).clone();

	actor_sqlite::migrate_v1_to_v2(
		db,
		actor_sqlite::MigrateV1ToV2Input {
			actor_id: input.actor_id,
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			protocol_version: input.protocol_version,
		},
	)
	.await
}
