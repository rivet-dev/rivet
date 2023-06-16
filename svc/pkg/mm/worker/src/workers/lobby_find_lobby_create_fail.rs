use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "mm-lobby-find-lobby-create-fail")]
async fn worker(ctx: OperationContext<mm::msg::lobby_create_fail::Message>) -> GlobalResult<()> {
	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();

	let error_code = match mm::msg::lobby_create_fail::ErrorCode::from_i32(ctx.error_code) {
		Some(mm::msg::lobby_create_fail::ErrorCode::LobbyCountOverMax) => {
			mm::msg::lobby_find_fail::ErrorCode::LobbyCountOverMax
		}
		Some(mm::msg::lobby_create_fail::ErrorCode::RegionNotEnabled) => {
			mm::msg::lobby_find_fail::ErrorCode::RegionNotEnabled
		}
		Some(mm::msg::lobby_create_fail::ErrorCode::StaleMessage) => {
			mm::msg::lobby_find_fail::ErrorCode::StaleMessage
		}
		Some(mm::msg::lobby_create_fail::ErrorCode::Unknown) | None => {
			tracing::warn!("unknown lobby create fail error code");
			mm::msg::lobby_find_fail::ErrorCode::Unknown
		}
	};

	// TODO: Is there a race condition here for new queries?

	// Attempt to complete all pending queries for this lobby
	let query_list = op!([ctx] mm_lobby_find_lobby_query_list {
		lobby_id: Some(lobby_id.into())
	})
	.await?;
	op!([ctx] mm_lobby_find_fail {
		query_ids: query_list.query_ids.clone(),
		error_code: error_code as i32,
		..Default::default()
	})
	.await?;

	Ok(())
}
