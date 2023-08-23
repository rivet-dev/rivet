use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::backend::{self, pkg::*};
use rivet_cloud_server::models;
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::{assert, auth::Auth, convert};

// MARK: DELETE /games/{}/matchmaker/lobby/{}
pub async fn delete_lobby(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	lobby_id: Uuid,
) -> GlobalResult<models::DeleteMatchmakerLobbyResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	// Do this before getting the lobby for race conditions
	let mut complete_sub = subscribe!([ctx] mm::msg::lobby_cleanup_complete(lobby_id)).await?;

	let lobby_get_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		include_stopped: false,
	})
	.await?;

	let did_remove = if let Some(lobby) = lobby_get_res.lobbies.first() {
		// Get the namespace
		let ns_id = internal_unwrap!(lobby.namespace_id);

		let ns_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![*ns_id],
		})
		.await?;
		let ns_data = internal_unwrap_owned!(ns_res.namespaces.first());
		let ns_game_id = internal_unwrap!(ns_data.game_id).as_uuid();

		// Validate this lobby belongs to this game
		internal_assert_eq!(ns_game_id, game_id, "lobby does not belong to game");

		// Remove the lobby
		msg!([ctx] mm::msg::lobby_stop(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await?;

		// Wait for cleanup message
		complete_sub.next().await?;

		true
	} else {
		tracing::info!("lobby not found");
		false
	};

	Ok(models::DeleteMatchmakerLobbyResponse { did_remove })
}

// MARK: POST /games/{}/matchmaker/lobby/export-history
pub async fn export_history(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::ExportMatchmakerLobbyHistoryRequest,
) -> GlobalResult<models::ExportMatchmakerLobbyHistoryResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	let namespaces_res = op!([ctx] game_namespace_list {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = internal_unwrap_owned!(namespaces_res.games.first());

	let request_id = Uuid::new_v4();
	let res = msg!([ctx] mm::msg::lobby_history_export(request_id) -> mm::msg::lobby_history_export_complete {
		request_id: Some(request_id.into()),
		namespace_ids: game.namespace_ids.clone(),
		query_start: body.query_start,
		query_end: body.query_end,
	})
	.await?;
	let upload_id = internal_unwrap!(res.upload_id).as_uuid();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id.into()],
	})
	.await?;
	let _upload = internal_unwrap_owned!(upload_res.uploads.first(), "upload not found");

	let url = format!(
		"https://cdn.{}/lobby-history-export/{}/export.csv",
		util::env::domain_main(),
		upload_id,
	);

	Ok(models::ExportMatchmakerLobbyHistoryResponse { url })
}

// MARK: GET /games/{}/matchmaker/lobbies/{}/logs
#[derive(Debug, Deserialize)]
pub struct GetLobbyLogsQuery {
	pub stream: String,
}

pub async fn get_lobby_logs(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	lobby_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetLobbyLogsQuery,
) -> GlobalResult<models::GetLobbyLogsResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	// Determine stream type
	let stream_type = match models::LogStream::from(query.stream.as_str()) {
		models::LogStream::StdOut => backend::nomad_log::StreamType::StdOut,
		models::LogStream::StdErr => backend::nomad_log::StreamType::StdErr,
		_ => {
			panic_with!(
				API_BAD_QUERY_PARAMETER,
				parameter = "stream",
				error = r#"Must be one of "std_out" or "std_err""#,
			);
		}
	};

	let alloc_id = if let Some(x) = get_alloc_id(&ctx, game_id, lobby_id).await? {
		x
	} else {
		panic_with!(MATCHMAKER_LOBBY_NOT_STARTED);
	};

	// Handle anchor
	if let Some(anchor) = watch_index.to_consumer()? {
		// Fetch logs
		let stream_type_param = match stream_type {
			backend::nomad_log::StreamType::StdOut => "stdout",
			backend::nomad_log::StreamType::StdErr => "stderr",
		};
		let log_tail = tail_all!([ctx, anchor, chirp_client::TailAllConfig::wait()] nomad_log::msg::entries(&alloc_id, util_job::GAME_TASK_NAME, stream_type_param)).await?;

		// Sort entries by timestamp
		let mut entries = log_tail
			.messages
			.iter()
			.flat_map(|x| x.entries.iter())
			.collect::<Vec<_>>();
		entries.sort_by_key(|x| x.ts);

		// Split lines and timestamps
		let mut lines = Vec::with_capacity(log_tail.messages.len());
		let mut timestamps = Vec::with_capacity(log_tail.messages.len());
		for entry in &entries {
			lines.push(base64::encode(&entry.message));
			timestamps.push(util::timestamp::to_chrono(entry.ts)?);
		}

		let update_ts = log_tail
			.messages
			.iter()
			.map(|x| x.msg_ts())
			.max()
			.unwrap_or_else(util::timestamp::now);
		return Ok(models::GetLobbyLogsResponse {
			lines,
			timestamps,
			watch: convert::watch_response(WatchResponse::new(update_ts)),
		});
	}

	// Read logs
	let before_ts = util::timestamp::now();
	let logs_res = op!([ctx] @dont_log_body nomad_log_read {
		alloc: alloc_id.clone(),
		task: util_job::GAME_TASK_NAME.into(),
		stream_type: stream_type as i32,
		count: 256,
		query: Some(nomad_log::read::request::Query::BeforeTs(nomad_log::read::request::TimestampQuery {
			ts: before_ts,
			idx: 0,
		})),
	})
	.await?;

	let watch_ts = logs_res.entries.last().map_or(before_ts, |x| x.ts);
	Ok(models::GetLobbyLogsResponse {
		lines: logs_res
			.entries
			.iter()
			.map(|entry| base64::encode(&entry.message))
			.collect(),
		timestamps: logs_res
			.entries
			.iter()
			.map(|x| x.ts)
			.map(util::timestamp::to_chrono)
			.collect::<Result<Vec<_>, _>>()?,
		watch: convert::watch_response(WatchResponse::new(watch_ts)),
	})
}

// MARK: POST /games/{}/matchmaker/lobbies/{}/logs/export
pub async fn export_lobby_logs(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	lobby_id: Uuid,
	body: models::ExportLobbyLogsRequest,
) -> GlobalResult<models::ExportLobbyLogsResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	let stream_type = match body.stream {
		models::LogStream::StdOut => backend::nomad_log::StreamType::StdOut,
		models::LogStream::StdErr => backend::nomad_log::StreamType::StdErr,
		models::LogStream::Unknown(_) => {
			panic_with!(API_BAD_BODY, error = r#"Invalid "stream""#,);
		}
	};

	let alloc_id = if let Some(x) = get_alloc_id(&ctx, game_id, lobby_id).await? {
		x
	} else {
		panic_with!(MATCHMAKER_LOBBY_NOT_STARTED);
	};

	// Export history
	let request_id = Uuid::new_v4();
	let res = msg!([ctx] nomad_log::msg::export(request_id) -> nomad_log::msg::export_complete {
		request_id: Some(request_id.into()),
		alloc: alloc_id,
		task: util_job::GAME_TASK_NAME.into(),
		stream_type: stream_type as i32,
	})
	.await?;
	let upload_id = internal_unwrap!(res.upload_id).as_uuid();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id.into()],
	})
	.await?;
	let _upload = internal_unwrap_owned!(upload_res.uploads.first(), "upload not found");

	let filename = match stream_type {
		backend::nomad_log::StreamType::StdOut => "stdout.txt",
		backend::nomad_log::StreamType::StdErr => "stderr.txt",
	};
	let url = format!(
		"https://cdn.{}/nomad-log-export/{upload_id}/{filename}",
		util::env::domain_main()
	);

	Ok(models::ExportLobbyLogsResponse { url })
}

async fn get_alloc_id(
	ctx: &Ctx<Auth>,
	game_id: Uuid,
	lobby_id: Uuid,
) -> GlobalResult<Option<String>> {
	// Fetch lobbies and re-sort them
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		include_stopped: true,
	})
	.await?;
	let lobby = unwrap_with_owned!(lobby_res.lobbies.first(), MATCHMAKER_LOBBY_NOT_FOUND);

	// Validate lobby belongs to game
	let namespace_id = internal_unwrap!(lobby.namespace_id).as_uuid();
	let _game_ns = assert::namespace_for_game(ctx, game_id, namespace_id).await?;

	// Lookup run ID if exists
	let run_id = if let Some(x) = lobby.run_id {
		x.as_uuid()
	} else {
		tracing::info!("no lobby run id, returning empty logs");
		return Ok(None);
	};

	// Get run
	let run_res = op!([ctx] job_run_get {
		run_ids: vec![run_id.into()],
	})
	.await?;
	let run = internal_unwrap_owned!(run_res.runs.first());
	let run_meta = internal_unwrap!(run.run_meta);

	let alloc_id = if let Some(backend::job::run_meta::Kind::Nomad(nomad)) = &run_meta.kind {
		if let Some(alloc_id) = &nomad.alloc_id {
			alloc_id
		} else {
			tracing::info!("no run alloc id, returning empty logs");
			return Ok(None);
		}
	} else {
		internal_panic!("lobby is not a nomad job");
	};

	Ok(Some(alloc_id.clone()))
}
