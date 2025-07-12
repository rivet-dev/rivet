use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;
use std::time::Duration;

use crate::{
	assert,
	auth::{Auth, CheckOpts, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

// MARK: GET /actors/{}/logs
#[derive(Debug, Deserialize)]
pub struct GetActorLogsQuery {
	#[serde(flatten)]
	pub global: GlobalQuery,
	/// JSON-encoded user query expression for filtering logs
	pub query_json: Option<String>,
}

#[tracing::instrument(skip_all)]
pub async fn get_logs(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: GetActorLogsQuery,
) -> GlobalResult<models::ActorsGetActorLogsResponse> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: false,
				opt_auth: false,
			},
		)
		.await?;

	// Parse user query expression if provided
	let user_query_expr = if let Some(query_json) = &query.query_json {
		let expr = match serde_json::from_str::<clickhouse_user_query::QueryExpr>(query_json) {
			Ok(expr) => expr,
			Err(e) => {
				bail_with!(API_BAD_QUERY, error = e.to_string());
			}
		};
		Some(expr)
	} else {
		// No query provided, return empty result
		None
	};

	// Timestamp to start the query at
	let before_nts = util::timestamp::now() * 1_000_000;

	// Handle anchor
	let logs_res = if let Some(anchor) = watch_index.as_i64()? {
		let query_start = tokio::time::Instant::now();
		let user_query_expr_clone = user_query_expr.clone();

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
			let logs_res = ctx
				.op(pegboard::ops::actor::log::read_with_query::Input {
					env_id,
					count: 64,
					order_by: pegboard::ops::actor::log::read_with_query::Order::Desc,
					query: pegboard::ops::actor::log::read_with_query::Query::AfterNts(anchor),
					user_query_expr: user_query_expr_clone.clone(),
				})
				.await?;

			// Return logs
			if !logs_res.entries.is_empty() {
				break logs_res;
			}

			// Timeout cleanly
			if query_start.elapsed().as_millis() > util::watch::DEFAULT_TIMEOUT as u128 {
				break pegboard::ops::actor::log::read_with_query::Output {
					entries: Vec::new(),
				};
			}

			// Throttle request
			//
			// We don't use `tokio::time::interval` because if the request takes longer than 500
			// ms, we'll enter a tight loop of requests.
			tokio::time::sleep(Duration::from_millis(1000)).await;
		};

		// Since we're using watch, we don't want this request to return immediately if there's new
		// results. Add an artificial timeout in order to prevent a tight loop if there's a high
		// log frequency.
		tokio::time::sleep_until(query_start + Duration::from_secs(1)).await;

		logs_res
	} else {
		// Read most recent logs
		ctx.op(pegboard::ops::actor::log::read_with_query::Input {
			env_id,
			count: 256,
			order_by: pegboard::ops::actor::log::read_with_query::Order::Desc,
			query: pegboard::ops::actor::log::read_with_query::Query::BeforeNts(before_nts),
			user_query_expr: user_query_expr.clone(),
		})
		.await?
	};

	// Convert to old Output format for compatibility
	let logs_res = pegboard::ops::actor::log::read::Output {
		entries: logs_res
			.entries
			.into_iter()
			.map(|e| pegboard::ops::actor::log::read::LogEntry {
				ts: e.ts,
				message: e.message,
				stream_type: e.stream_type,
				actor_id: e.actor_id,
			})
			.collect(),
	};

	// Build actor_ids map for lookup
	let mut actor_id_to_index: std::collections::HashMap<Uuid, i32> =
		std::collections::HashMap::new();
	let mut unique_actor_ids: Vec<String> = Vec::new();

	// Collect unique actor IDs and map them to indices
	for entry in &logs_res.entries {
		if !actor_id_to_index.contains_key(&entry.actor_id) {
			actor_id_to_index.insert(entry.actor_id, unique_actor_ids.len() as i32);
			unique_actor_ids.push(entry.actor_id.to_string());
		}
	}

	// Convert logs
	let mut lines = logs_res
		.entries
		.iter()
		.map(|entry| base64::encode(&entry.message))
		.collect::<Vec<_>>();
	let mut timestamps = logs_res
		.entries
		.iter()
		// Is nanoseconds
		.map(|x| x.ts / 1_000_000)
		.map(util::timestamp::to_string)
		.collect::<Result<Vec<_>, _>>()?;
	let mut streams = logs_res
		.entries
		.iter()
		.map(|x| x.stream_type as i32)
		.collect::<Vec<_>>();
	let mut actor_indices = logs_res
		.entries
		.iter()
		.map(|x| *actor_id_to_index.get(&x.actor_id).unwrap_or(&0))
		.collect::<Vec<_>>();

	// Order desc
	lines.reverse();
	timestamps.reverse();
	streams.reverse();
	actor_indices.reverse();

	let watch_nts = logs_res.entries.first().map_or(before_nts, |x| x.ts);
	Ok(models::ActorsGetActorLogsResponse {
		actor_ids: unique_actor_ids,
		lines,
		timestamps,
		streams,
		actor_indices,
		watch: WatchResponse::new_as_model(watch_nts),
	})
}

pub async fn get_logs_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	server_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetActorLogsQuery,
) -> GlobalResult<models::ServersGetServerLogsResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;

	// Build a proper query expression for the single actor
	let query_expr = clickhouse_user_query::QueryExpr::StringEqual {
		property: "actor_id".to_string(),
		map_key: None,
		value: server_id.to_string(),
		case_insensitive: false,
	};
	let query_json = Some(serde_json::to_string(&query_expr)?);

	let logs_res = get_logs(ctx, watch_index, GetActorLogsQuery { global, query_json }).await?;
	Ok(models::ServersGetServerLogsResponse {
		lines: logs_res.lines,
		timestamps: logs_res.timestamps,
		// streams are not part of the deprecated response
		watch: logs_res.watch,
	})
}

// MARK: POST /actors/logs/export
#[tracing::instrument(skip_all)]
pub async fn export_logs(
	ctx: Ctx<Auth>,
	body: models::ActorsLogsExportRequest,
) -> GlobalResult<models::ActorsExportActorLogsResponse> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &GlobalQuery {
					project: body.project,
					environment: body.environment,
				},
				allow_service_token: false,
				opt_auth: false,
			},
		)
		.await?;

	// Parse user query expression if provided
	let user_query_expr = if let Some(query_json) = &body.query_json {
		let expr = match serde_json::from_str::<clickhouse_user_query::QueryExpr>(query_json) {
			Ok(expr) => expr,
			Err(e) => {
				bail_with!(API_BAD_QUERY, error = e.to_string());
			}
		};
		Some(expr)
	} else {
		// No query provided, return empty result
		return Ok(models::ActorsExportActorLogsResponse { url: String::new() });
	};

	// Read all logs (no limit)
	let logs_res = ctx
		.op(pegboard::ops::actor::log::read_with_query::Input {
			env_id,
			count: i64::MAX,
			order_by: pegboard::ops::actor::log::read_with_query::Order::Asc,
			query: pegboard::ops::actor::log::read_with_query::Query::All,
			user_query_expr,
		})
		.await?;

	// Format logs as plain text
	let mut output = String::new();
	for entry in &logs_res.entries {
		let timestamp = util::timestamp::to_string(entry.ts / 1_000_000)?;
		let stream_name = match entry.stream_type {
			x if x == (pegboard::types::LogsStreamType::StdOut as u8) => "stdout",
			x if x == (pegboard::types::LogsStreamType::StdErr as u8) => "stderr",
			x => bail!("unknown stream type {x}"),
		};
		let message = String::from_utf8_lossy(&entry.message);
		output.push_str(&format!("{} [{}] {}\n", timestamp, stream_name, message));
	}

	// Generate filename
	let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
	let filename = format!("actor_logs_export_{}.txt", timestamp);

	// Upload to S3
	let mime = "text/plain";
	let content_length = output.len();
	let upload_res = op!([ctx] upload_prepare {
		bucket: "bucket-actor-log-export".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: filename.clone(),
				mime: Some(mime.into()),
				content_length: content_length as u64,
				..Default::default()
			},
		],
		presigned_endpoint_kind: Some(backend::pkg::upload::prepare::EndpointKind::Internal as i32),
	})
	.await?;
	let upload_id = unwrap!(upload_res.upload_id).as_uuid();

	let presigned_req = unwrap!(upload_res.presigned_requests.first());

	// Upload the content
	let res = reqwest::Client::new()
		.put(&presigned_req.url)
		.body(output)
		.header(reqwest::header::CONTENT_TYPE, mime)
		.header(reqwest::header::CONTENT_LENGTH, content_length)
		.send()
		.await?;

	if !res.status().is_success() {
		let status = res.status();
		let text = res.text().await;
		tracing::error!(?status, ?text, "failed to upload logs");
		bail!("failed to upload logs");
	}

	op!([ctx] upload_complete {
		upload_id: upload_res.upload_id,
		bucket: Some("bucket-actor-log-export".into()),
	})
	.await?;

	// Generate download URL (valid for 1 hour)
	let s3_client = s3_util::Client::with_bucket_and_endpoint(
		ctx.config(),
		"bucket-actor-log-export",
		s3_util::EndpointKind::External,
	)
	.await?;
	let presigned_req = s3_client
		.get_object()
		.bucket(s3_client.bucket())
		.key(format!("{upload_id}/{filename}"))
		.presigned(
			s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
				.expires_in(Duration::from_secs(60 * 60))
				.build()?,
		)
		.await?;

	Ok(models::ActorsExportActorLogsResponse {
		url: presigned_req.uri().to_string(),
	})
}
