use proto::{
	backend::{self, pkg::*},
	perf,
};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiTryFrom, ApiTryInto};

pub mod cloudflare;
pub mod namespace;
pub mod version;

pub fn analytics_lobby_summary_from_lobby(
	lobby: backend::matchmaker::Lobby,
	player_count: mm::lobby_player_count::response::Lobby,
	lobby_group: &backend::matchmaker::LobbyGroup,
	is_outdated: bool,
) -> GlobalResult<models::CloudLobbySummaryAnalytics> {
	Ok(models::CloudLobbySummaryAnalytics {
		lobby_id: unwrap_ref!(lobby.lobby_id).as_uuid(),
		lobby_group_name_id: lobby_group.name_id.clone(),
		lobby_group_id: unwrap_ref!(lobby.lobby_group_id).as_uuid(),
		region_id: unwrap_ref!(lobby.region_id).as_uuid(),
		create_ts: util::timestamp::to_string(lobby.create_ts)?,
		is_ready: lobby.ready_ts.is_some(),
		is_idle: player_count.total_player_count == 0,
		is_closed: lobby.is_closed,
		is_outdated,
		max_players_normal: lobby.max_players_normal.try_into()?,
		max_players_direct: lobby.max_players_direct.try_into()?,
		max_players_party: lobby.max_players_party.try_into()?,
		total_player_count: player_count.total_player_count.try_into()?,
		registered_player_count: player_count.registered_player_count.try_into()?,
	})
}

// TODO: Remove
// impl ApiTryFrom<mm::lobby_runtime_aggregate::response::RegionTierTime>
// 	for models::CloudRegionTierExpenses
// {
// 	type Error = GlobalError;

// 	fn try_from(
// 		value: mm::lobby_runtime_aggregate::response::RegionTierTime,
// 	) -> GlobalResult<Self> {
// 		let uptime_in_seconds = util::div_up!(value.total_time, 1_000);

// 		Ok(models::CloudRegionTierExpenses {
// 			namespace_id: unwrap_ref!(value.namespace_id).as_uuid(),
// 			region_id: unwrap_ref!(value.region_id).as_uuid(),
// 			tier_name_id: value.tier_name_id,
// 			lobby_group_name_id: value.lobby_group_name_id,
// 			uptime: uptime_in_seconds,
// 		})
// 	}
// }

impl ApiTryFrom<backend::game::Game> for models::GameHandle {
	type Error = GlobalError;

	fn try_from(value: backend::game::Game) -> GlobalResult<Self> {
		Ok(models::GameHandle {
			game_id: unwrap_ref!(value.game_id).as_uuid(),
			name_id: value.name_id.to_owned(),
			display_name: value.display_name.to_owned(),
			logo_url: util::route::game_logo(&value),
			banner_url: util::route::game_banner(&value),
		})
	}
}

impl ApiTryFrom<perf::SvcPerf> for models::CloudSvcPerf {
	type Error = GlobalError;

	fn try_from(value: perf::SvcPerf) -> GlobalResult<Self> {
		Ok(models::CloudSvcPerf {
			svc_name: value.context_name.to_owned(),
			ts: util::timestamp::to_string(value.ts)?,
			duration: value.duration,
			req_id: value.req_id.map(|req_id| (*req_id)),
			spans: value
				.spans
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<Result<Vec<_>, _>>()?,
			marks: value
				.marks
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<Result<Vec<_>, _>>()?,
		})
	}
}

impl ApiTryFrom<job_run::metrics_log::response::Metrics> for models::CloudSvcMetrics {
	type Error = GlobalError;

	fn try_from(value: job_run::metrics_log::response::Metrics) -> GlobalResult<Self> {
		Ok(models::CloudSvcMetrics {
			job: value.job,
			cpu: value.cpu.into_iter().map(|v| v as f64).collect::<Vec<_>>(),
			memory: value
				.memory
				.into_iter()
				.map(|v| v as f64)
				.collect::<Vec<_>>(),
			allocated_memory: Some(value.allocated_memory as f64),
		})
	}
}

impl ApiTryFrom<perf::Span> for models::CloudLogsPerfSpan {
	type Error = GlobalError;

	fn try_from(value: perf::Span) -> GlobalResult<Self> {
		Ok(models::CloudLogsPerfSpan {
			label: value.label.to_owned(),
			start_ts: util::timestamp::to_string(value.start_ts)?,
			finish_ts: value
				.finish_ts
				.map(util::timestamp::to_string)
				.transpose()?,
			req_id: value.req_id.map(|req_id| (*req_id)),
		})
	}
}

impl ApiTryFrom<perf::Mark> for models::CloudLogsPerfMark {
	type Error = GlobalError;

	fn try_from(value: perf::Mark) -> GlobalResult<Self> {
		Ok(models::CloudLogsPerfMark {
			label: value.label.to_owned(),
			ts: util::timestamp::to_string(value.ts)?,
			ray_id: value.ray_id.map(|ray_id| (*ray_id)),
			req_id: value.req_id.map(|ray_id| (*ray_id)),
		})
	}
}

impl ApiTryFrom<backend::upload::PresignedUploadRequest> for models::UploadPresignedRequest {
	type Error = GlobalError;

	fn try_from(value: backend::upload::PresignedUploadRequest) -> GlobalResult<Self> {
		Ok(models::UploadPresignedRequest {
			path: value.path,
			url: value.url,
			byte_offset: value.byte_offset as i64,
			content_length: value.content_length as i64,
		})
	}
}

impl ApiTryFrom<models::UploadPrepareFile> for backend::upload::PrepareFile {
	type Error = GlobalError;

	fn try_from(value: models::UploadPrepareFile) -> GlobalResult<Self> {
		ensure_with!(
			value.content_length >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`file.content_length` out of bounds"
		);

		Ok(backend::upload::PrepareFile {
			path: value.path,
			mime: value.content_type,
			content_length: value.content_length as u64,
			..Default::default()
		})
	}
}

impl ApiFrom<team::validate::response::Error> for models::ValidationError {
	fn api_from(value: team::validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiFrom<game::validate::response::Error> for models::ValidationError {
	fn api_from(value: game::validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiFrom<game::namespace_validate::response::Error> for models::ValidationError {
	fn api_from(value: game::namespace_validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiFrom<game::version_validate::response::Error> for models::ValidationError {
	fn api_from(value: game::version_validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiFrom<game::token_development_validate::response::Error> for models::ValidationError {
	fn api_from(
		value: game::token_development_validate::response::Error,
	) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiFrom<mm_config::namespace_config_validate::response::Error> for models::ValidationError {
	fn api_from(
		value: mm_config::namespace_config_validate::response::Error,
	) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

impl ApiTryFrom<backend::region::Tier> for models::CloudRegionTier {
	type Error = GlobalError;

	fn try_from(value: backend::region::Tier) -> GlobalResult<Self> {
		Ok(models::CloudRegionTier {
			tier_name_id: value.tier_name_id.to_owned(),
			rivet_cores_numerator: value.rivet_cores_numerator.try_into()?,
			rivet_cores_denominator: value.rivet_cores_denominator.try_into()?,
			cpu: value.cpu.try_into()?,
			memory: value.memory.try_into()?,
			disk: value.disk.try_into()?,
			bandwidth: value.bandwidth.try_into()?,

			price_per_second: 0,
		})
	}
}

impl ApiFrom<backend::region::UniversalRegion> for models::CloudUniversalRegion {
	fn api_from(value: backend::region::UniversalRegion) -> Self {
		use backend::region::UniversalRegion::*;

		match value {
			Unknown => models::CloudUniversalRegion::Unknown,
			Local => models::CloudUniversalRegion::Local,
			Amsterdam => models::CloudUniversalRegion::Amsterdam,
			Atlanta => models::CloudUniversalRegion::Atlanta,
			Bangalore => models::CloudUniversalRegion::Bangalore,
			Dallas => models::CloudUniversalRegion::Dallas,
			Frankfurt => models::CloudUniversalRegion::Frankfurt,
			London => models::CloudUniversalRegion::London,
			Mumbai => models::CloudUniversalRegion::Mumbai,
			Newark => models::CloudUniversalRegion::Newark,
			NewYorkCity => models::CloudUniversalRegion::NewYorkCity,
			SanFrancisco => models::CloudUniversalRegion::SanFrancisco,
			Singapore => models::CloudUniversalRegion::Singapore,
			Sydney => models::CloudUniversalRegion::Sydney,
			Tokyo => models::CloudUniversalRegion::Tokyo,
			Toronto => models::CloudUniversalRegion::Toronto,
			WashingtonDc => models::CloudUniversalRegion::WashingtonDc,
			Chicago => models::CloudUniversalRegion::Chicago,
			Paris => models::CloudUniversalRegion::Paris,
			Seattle => models::CloudUniversalRegion::Seattle,
			SaoPaulo => models::CloudUniversalRegion::SaoPaulo,
			Stockholm => models::CloudUniversalRegion::Stockholm,
			Chennai => models::CloudUniversalRegion::Chennai,
			Osaka => models::CloudUniversalRegion::Osaka,
			Milan => models::CloudUniversalRegion::Milan,
			Miami => models::CloudUniversalRegion::Miami,
			Jakarta => models::CloudUniversalRegion::Jakarta,
			LosAngeles => models::CloudUniversalRegion::LosAngeles,
		}
	}
}
