use std::time::Duration;

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
		let ns_id = unwrap_ref!(lobby.namespace_id);

		let ns_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![*ns_id],
		})
		.await?;
		let ns_data = unwrap!(ns_res.namespaces.first());
		let ns_game_id = unwrap_ref!(ns_data.game_id).as_uuid();

		// Validate this lobby belongs to this game
		ensure_eq!(ns_game_id, game_id, "lobby does not belong to game");

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
	let game = unwrap!(namespaces_res.games.first());

	let request_id = Uuid::new_v4();
	let res = msg!([ctx] mm::msg::lobby_history_export(request_id) -> mm::msg::lobby_history_export_complete {
		request_id: Some(request_id.into()),
		namespace_ids: game.namespace_ids.clone(),
		query_start: body.query_start,
		query_end: body.query_end,
	})
	.await?;
	let upload_id = unwrap_ref!(res.upload_id).as_uuid();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id.into()],
	})
	.await?;
	let upload = unwrap!(upload_res.uploads.first(), "upload not found");

	let proto_provider = unwrap!(
		backend::upload::Provider::from_i32(upload.provider),
		"invalid upload provider"
	);
	let provider = match proto_provider {
		backend::upload::Provider::Minio => s3_util::Provider::Minio,
		backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
		backend::upload::Provider::Aws => s3_util::Provider::Aws,
	};
	let s3_client = s3_util::Client::from_env_opt(
		"bucket-lobby-history-export",
		provider,
		s3_util::EndpointKind::External,
	)
	.await?;
	let presigned_req = s3_client
		.get_object()
		.bucket(s3_client.bucket())
		.key(format!("{upload_id}/export.csv"))
		.presigned(
			s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
				.expires_in(Duration::from_secs(60 * 60))
				.build()?,
		)
		.await?;

	Ok(models::ExportMatchmakerLobbyHistoryResponse {
		url: presigned_req.uri().to_string(),
	})
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

	// Get start ts
	// If no watch: read logs
	// If watch:
	//   loop read logs from index
	//	 wait until start ts + 1 second

	// Determine stream type
	let stream_type = match models::LogStream::from(query.stream.as_str()) {
		models::LogStream::StdOut => backend::job::log::StreamType::StdOut,
		models::LogStream::StdErr => backend::job::log::StreamType::StdErr,
		_ => {
			bail_with!(
				API_BAD_QUERY_PARAMETER,
				parameter = "stream",
				error = r#"Must be one of "std_out" or "std_err""#,
			);
		}
	};

	// Timestamp to start the query at
	let before_ts = util::timestamp::now() * 1_000_000;

	// Get run ID
	let run_id = if let Some(x) = get_run_id(&ctx, game_id, lobby_id).await? {
		x
	} else {
		// Throttle request if watching. This is effectively polling until the lobby is ready.
		if watch_index.to_consumer()?.is_some() {
			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		// Return empty logs
		return Ok(models::GetLobbyLogsResponse {
			lines: Vec::new(),
			timestamps: Vec::new(),
			watch: convert::watch_response(WatchResponse::new(before_ts)),
		});
	};

	// Handle anchor
	let logs_res = if let Some(anchor) = watch_index.as_i64()? {
		let query_start = tokio::time::Instant::now();

		// Poll for new logs
		let logs_res = loop {
			// Read logs after the timestamp
			//
			// We read descending in order to get at most 256 of the most recent logs. If we used
			// asc, we would be paginating through all the logs which would likely fall behind
			// actual stream and strain the database.
			//
			// We return fewer logs than the non-anchor request since this will be called
			// frequently and should not return a significant amount of logs.
			let logs_res = op!([ctx] @dont_log_body job_log_read {
				run_id: Some(run_id.into()),
				task: util_job::RUN_MAIN_TASK_NAME.into(),
				stream_type: stream_type as i32,
				count: 64,
				order_asc: false,
				query: Some(job_log::read::request::Query::AfterTs(anchor))

			})
			.await?;

			// Return logs
			if !logs_res.entries.is_empty() {
				break logs_res;
			}

			// Throttle request
			//
			// We don't use `tokio::time::interval` because if the request takes longer than 500
			// ms, we'll enter a tight loop of requests.
			tokio::time::sleep(Duration::from_millis(500)).await;
		};

		// Since we're using watch, we don't want this request to return immediately if there's new
		// results. Add an artificial timeout in order to prevent a tight loop if there's a high
		// log frequency.
		tokio::time::sleep_until(query_start + Duration::from_secs(1)).await;

		logs_res
	} else {
		// Read most recent logs
		let logs_res = op!([ctx] @dont_log_body job_log_read {
			run_id: Some(run_id.into()),
			task: util_job::RUN_MAIN_TASK_NAME.into(),
			stream_type: stream_type as i32,
			count: 256,
			order_asc: false,
			query: Some(job_log::read::request::Query::BeforeTs(before_ts)),
		})
		.await?;

		logs_res
	};

	// Convert logs
	let mut lines = logs_res
		.entries
		.iter()
		.map(|entry| base64::encode(&entry.message))
		.collect::<Vec<_>>();
	let mut timestamps = logs_res
		.entries
		.iter()
		.map(|x| x.ts / 1_000_000)
		.map(util::timestamp::to_chrono)
		.collect::<Result<Vec<_>, _>>()?;

	// Order desc
	lines.reverse();
	timestamps.reverse();

	let watch_ts = logs_res.entries.first().map_or(before_ts, |x| x.ts);
	Ok(models::GetLobbyLogsResponse {
		lines,
		timestamps,
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
		models::LogStream::StdOut => backend::job::log::StreamType::StdOut,
		models::LogStream::StdErr => backend::job::log::StreamType::StdErr,
		models::LogStream::Unknown(_) => {
			bail_with!(API_BAD_BODY, error = r#"Invalid "stream""#,);
		}
	};

	let run_id = if let Some(x) = get_run_id(&ctx, game_id, lobby_id).await? {
		x
	} else {
		bail_with!(MATCHMAKER_LOBBY_NOT_STARTED);
	};

	// Export history
	let request_id = Uuid::new_v4();
	let res = msg!([ctx] job_log::msg::export(request_id) -> job_log::msg::export_complete {
		request_id: Some(request_id.into()),
		run_id: Some(run_id.into()),
		task: util_job::RUN_MAIN_TASK_NAME.into(),
		stream_type: stream_type as i32,
	})
	.await?;
	let upload_id = unwrap_ref!(res.upload_id).as_uuid();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id.into()],
	})
	.await?;
	let upload = unwrap!(upload_res.uploads.first(), "upload not found");

	let filename = match stream_type {
		backend::job::log::StreamType::StdOut => "stdout.txt",
		backend::job::log::StreamType::StdErr => "stderr.txt",
	};

	let proto_provider = unwrap!(
		backend::upload::Provider::from_i32(upload.provider),
		"invalid upload provider"
	);
	let provider = match proto_provider {
		backend::upload::Provider::Minio => s3_util::Provider::Minio,
		backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
		backend::upload::Provider::Aws => s3_util::Provider::Aws,
	};
	let s3_client = s3_util::Client::from_env_opt(
		"bucket-job-log-export",
		provider,
		s3_util::EndpointKind::External,
	)
	.await?;
	let presigned_req = s3_client
		.get_object()
		.bucket(s3_client.bucket())
		.key(format!("{upload_id}/{filename}"))
		.presigned(
			s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
				.expires_in(Duration::from_secs(60 * 60))
				.build()?,
		)
		.await?;

	Ok(models::ExportLobbyLogsResponse {
		url: presigned_req.uri().to_string(),
	})
}

async fn get_run_id(ctx: &Ctx<Auth>, game_id: Uuid, lobby_id: Uuid) -> GlobalResult<Option<Uuid>> {
	// Fetch lobbies and re-sort them
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		include_stopped: true,
	})
	.await?;
	let lobby = unwrap_with!(lobby_res.lobbies.first(), MATCHMAKER_LOBBY_NOT_FOUND);

	// Validate lobby belongs to game
	let namespace_id = unwrap_ref!(lobby.namespace_id).as_uuid();
	let _game_ns = assert::namespace_for_game(ctx, game_id, namespace_id).await?;

	// Lookup run ID if exists
	let run_id = if let Some(x) = lobby.run_id {
		x.as_uuid()
	} else {
		tracing::info!("no lobby run id, returning empty logs");
		return Ok(None);
	};

	Ok(Some(run_id))
}
