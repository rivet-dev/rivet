use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;
use std::time::Duration;

use crate::{assert, auth::Auth};

// MARK: GET /servers/{server_id}/logs
#[derive(Debug, Deserialize)]
pub struct GetServerLogsQuery {
	pub stream: models::CloudGamesLogStream,
}

pub async fn get_logs(
	ctx: Ctx<Auth>,
	server_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetServerLogsQuery,
) -> GlobalResult<models::ServersGetServerLogsResponse> {
	let game_id = ctx.auth().check_game_service_or_cloud_token().await?;

	// ctx.auth()
	// 	.check_game_read_or_admin(ctx.op_ctx(), game_id)
	// 	.await?;

	// Get start ts
	// If no watch: read logs
	// If watch:
	//   loop read logs from index
	//	 wait until start ts + 1 second

	// Determine stream type
	let stream_type = match query.stream {
		models::CloudGamesLogStream::StdOut => backend::job::log::StreamType::StdOut,
		models::CloudGamesLogStream::StdErr => backend::job::log::StreamType::StdErr,
	};

	// Timestamp to start the query at
	let before_nts = util::timestamp::now() * 1_000_000;

	// Validate server belongs to game
	let _game_ns = assert::server_for_game(&ctx, server_id, game_id).await?;

	// // Get run ID
	// let server_id = if let Some(x) = get_server_id(&ctx, game_id, lobby_id).await? {
	// 	x
	// } else {
	// 	// Throttle request if watching. This is effectively polling until the lobby is ready.
	// 	if watch_index.to_consumer()?.is_some() {
	// 		tokio::time::sleep(Duration::from_secs(1)).await;
	// 	}

	// 	// Return empty logs
	// 	return Ok(models::CloudGamesGetLobbyLogsResponse {
	// 		lines: Vec::new(),
	// 		timestamps: Vec::new(),
	// 		watch: WatchResponse::new_as_model(before_nts),
	// 	});
	// };

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
			let logs_res = op!([ctx] @dont_log_body ds_log_read {
				server_id: Some(server_id.into()),
				stream_type: stream_type as i32,
				count: 64,
				order_asc: false,
				query: Some(ds_log::read::request::Query::AfterNts(anchor))

			})
			.await?;

			let list_res = op!([ctx] @dont_log_body ds_log_read {
				server_id: Some(server_id.into()),
				stream_type: stream_type as i32,
				count: 64,
				order_asc: false,
				query: Some(ds_log::read::request::Query::AfterNts(anchor))
			})
			.await?;

			// Return logs
			if !logs_res.entries.is_empty() {
				break logs_res;
			}

			// Timeout cleanly
			if query_start.elapsed().as_millis() > util::watch::DEFAULT_TIMEOUT as u128 {
				break ds_log::read::Response {
					entries: Vec::new(),
				};
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

		op!([ctx] @dont_log_body ds_log_read {
			server_id: Some(server_id.into()),
			stream_type: stream_type as i32,
			count: 256,
			order_asc: false,
			query: Some(ds_log::read::request::Query::BeforeNts(before_nts)),
		})
		.await?
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
		.map(|x| x.nts / 1_000_000)
		.map(util::timestamp::to_string)
		.collect::<Result<Vec<_>, _>>()?;

	// Order desc
	lines.reverse();
	timestamps.reverse();

	let watch_nts = logs_res.entries.first().map_or(before_nts, |x| x.nts);
	Ok(models::ServersGetServerLogsResponse {
		lines,
		timestamps,
		watch: WatchResponse::new_as_model(watch_nts),
	})
}
