use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_cloud_server::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{assert, auth::Auth};

struct LobbyInfo {
	summary: models::LogsLobbySummary,
	run: Option<LobbyRunInfo>,
}

struct LobbyRunInfo {
	dispatched_job_id: Option<String>,
}

// MARK: GET /games/{}/namespaces/{}/logs/lobbies
#[derive(Debug, Serialize, Deserialize)]
pub struct ListNamespaceLobbiesQuery {
	pub before_create_ts: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn list_lobbies(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ListNamespaceLobbiesQuery,
) -> GlobalResult<models::ListNamespaceLobbiesResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;
	let _game_ns = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	// Fetch lobby list
	let list_res = op!([ctx] mm_lobby_history {
		namespace_id: Some(namespace_id.into()),
		before_create_ts: query.before_create_ts
			.map(|ts| ts.timestamp_millis())
			.unwrap_or_else(util::timestamp::now),
		count: 64,
	})
	.await?;
	let lobby_ids = list_res
		.lobby_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch lobby data
	let lobbies = fetch_lobby_logs(&ctx, lobby_ids)
		.await?
		.into_iter()
		.map(|x| x.summary)
		.collect::<Vec<_>>();

	Ok(models::ListNamespaceLobbiesResponse { lobbies })
}

// MARK: GET /games/{}/namespaces/{}/logs/lobbies/{}
pub async fn get_lobby(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	lobby_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetNamespaceLobbyResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;
	let _game_ns = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	let lobby = unwrap!(fetch_lobby_logs(&ctx, vec![lobby_id])
		.await?
		.into_iter()
		.next());

	let metrics = if let Some(run) = lobby.run {
		let now = util::timestamp::now();
		if let Some(dispatched_job_id) = run.dispatched_job_id {
			let metrics_res = op!([ctx] job_run_metrics_log {
				start: now - util::duration::minutes(15),
				end: now,
				step: 15000,
				metrics: vec![job_run::metrics_log::request::Metric {
					job: dispatched_job_id.clone(),
					task: "game".to_owned(),
				}],
			})
			.await;

			// Metrics job fails gracefully, errors are not propagated
			if let Err(err) = &metrics_res {
				tracing::error!(?err, ?dispatched_job_id, "metrics fetch failed");
			}

			metrics_res
				.ok()
				.and_then(|res| res.metrics.first().cloned())
		} else {
			None
		}
	} else {
		None
	};

	Ok(models::GetNamespaceLobbyResponse {
		lobby: lobby.summary,
		perf_lists: Vec::new(),
		metrics: metrics.map(ApiTryInto::try_into).transpose()?,

		// Deprecated
		stdout_presigned_urls: Vec::new(),
		stderr_presigned_urls: Vec::new(),
	})
}

async fn fetch_lobby_logs(ctx: &Ctx<Auth>, lobby_ids: Vec<Uuid>) -> GlobalResult<Vec<LobbyInfo>> {
	// Fetch lobbies and re-sort them
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: lobby_ids
			.iter()
			.map(|x| Into::into(*x))
			.collect(),
		include_stopped: true,
	})
	.await?;
	let mut lobbies = lobby_res.lobbies.iter().collect::<Vec<_>>();
	lobbies.sort_by_key(|x| -(x.create_ts));

	let run_ids = lobbies
		.iter()
		.filter_map(|x| x.run_id)
		.map(|x| x.as_uuid())
		.collect::<HashSet<Uuid>>();

	// Get runs
	let run_res = op!([ctx] job_run_get {
		run_ids: run_ids
			.clone()
			.into_iter()
			.map(|x| x.into())
			.collect(),
	})
	.await?;

	// Fetch job statuses and lobby group data
	let lobby_group_res = op!([ctx] mm_config_lobby_group_get {
		lobby_group_ids: lobbies
			.iter()
			.filter_map(|x| x.lobby_group_id)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Map lobbies
	let lobbies = lobbies
		.iter()
		.map(|lobby| {
			let run = if let Some(run) = run_res.runs.iter().find(|run| run.run_id == lobby.run_id)
			{
				run
			} else {
				tracing::error!(?lobby, "missing run for lobby");
				bail!("missing run for lobby");
			};
			let run_meta = unwrap_ref!(run.run_meta);

			GlobalResult::Ok(LobbyInfo {
				summary: models::LogsLobbySummary {
					lobby_id: unwrap_ref!(lobby.lobby_id).as_uuid().to_string(),
					namespace_id: unwrap_ref!(lobby.namespace_id).as_uuid().to_string(),
					lobby_group_name_id: {
						let lobby_group = unwrap!(lobby_group_res
							.lobby_groups
							.iter()
							.find(|x| x.lobby_group_id.as_ref() == lobby.lobby_group_id.as_ref()));

						lobby_group.name_id.to_owned()
					},
					region_id: unwrap_ref!(lobby.region_id).as_uuid().to_string(),
					create_ts: util::timestamp::to_chrono(lobby.create_ts)?,
					start_ts: run.start_ts.map(util::timestamp::to_chrono).transpose()?,
					ready_ts: lobby.ready_ts.map(util::timestamp::to_chrono).transpose()?,
					status: if let Some(stop_ts) = lobby.stop_ts {
						let (failed, exit_code) = match unwrap_ref!(run_meta.kind) {
							backend::job::run_meta::Kind::Nomad(nomad) => {
								(nomad.failed.unwrap_or(false), nomad.exit_code.unwrap_or(0))
							}
						};

						models::LogsLobbyStatus::Stopped(models::LogsLobbyStatusStopped {
							stop_ts: util::timestamp::to_chrono(stop_ts)?,
							failed,
							exit_code: exit_code.try_into()?,
						})
					} else {
						models::LogsLobbyStatus::Running(models::Unit {})
					},
				},
				run: Some(LobbyRunInfo {
					dispatched_job_id: match unwrap_ref!(run_meta.kind) {
						backend::job::run_meta::Kind::Nomad(nomad) => {
							nomad.dispatched_job_id.clone()
						}
					},
				}),
			})
		})
		.filter_map(|x| match x {
			Ok(x) => Some(x),
			Err(err) => {
				tracing::error!(?err, "failed to build lobby info");
				None
			}
		})
		.collect::<Vec<_>>();

	Ok(lobbies)
}
