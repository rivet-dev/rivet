use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct LobbyGroup {
	lobby_group_id: Uuid,
	name_id: String,
}

impl From<LobbyGroup> for mm_config::lobby_group_get::response::LobbyGroup {
	fn from(value: LobbyGroup) -> Self {
		mm_config::lobby_group_get::response::LobbyGroup {
			lobby_group_id: Some(value.lobby_group_id.into()),
			name_id: value.name_id,
		}
	}
}

#[operation(name = "mm-config-lobby-group-get")]
async fn handle(
	ctx: OperationContext<mm_config::lobby_group_get::Request>,
) -> GlobalResult<mm_config::lobby_group_get::Response> {
	let lobby_group_ids = ctx
		.lobby_group_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let lobby_groups = sql_fetch_all!(
		[ctx, LobbyGroup]
		"
		SELECT name_id, lobby_group_id
		FROM db_mm_config.lobby_groups AS lg
		WHERE lobby_group_id = ANY($1)
		",
		lobby_group_ids,
	)
	.await?;

	Ok(mm_config::lobby_group_get::Response {
		lobby_groups: lobby_groups.into_iter().map(Into::into).collect::<Vec<_>>(),
	})
}
