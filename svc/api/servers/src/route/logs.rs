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

// MARK: GET /games/{}/environments/{}/servers/{}/logs
#[derive(Debug, Deserialize)]
pub struct GetServerLogsQuery {
	pub stream: models::CloudGamesLogStream,
}

pub async fn get_logs(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	server_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetServerLogsQuery,
) -> GlobalResult<models::ServersGetServerLogsResponse> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, false)
		.await?;

	// Validate server belongs to game
	assert::server_for_game(&ctx, server_id, game_id).await?;

	// Determine stream type
	let stream_type = match query.stream {
		models::CloudGamesLogStream::StdOut => backend::job::log::StreamType::StdOut,
		models::CloudGamesLogStream::StdErr => backend::job::log::StreamType::StdErr,
	};

	// Timestamp to start the query at
	let before_nts = util::timestamp::now() * 1_000_000;

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
