use proto::{
	backend::{self, pkg::*},
	perf,
};
use rivet_cloud_server::models;
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
) -> GlobalResult<models::AnalyticsLobbySummary> {
	Ok(models::AnalyticsLobbySummary {
		lobby_id: internal_unwrap!(lobby.lobby_id).to_string(),
		lobby_group_name_id: lobby_group.name_id.clone(),
		lobby_group_id: internal_unwrap!(lobby.lobby_group_id).to_string(),
		region_id: internal_unwrap!(lobby.region_id).to_string(),
		create_ts: util::timestamp::to_chrono(lobby.create_ts)?,
		is_ready: lobby.ready_ts.is_some(),
		is_idle: player_count.total_player_count == 0,
		is_closed: lobby.is_closed,
		is_outdated,
		max_players_normal: lobby.max_players_normal as i32,
		max_players_direct: lobby.max_players_direct as i32,
		max_players_party: lobby.max_players_party as i32,
		total_player_count: player_count.total_player_count as i32,
		registered_player_count: player_count.registered_player_count as i32,
	})
}

impl ApiTryFrom<mm::lobby_runtime_aggregate::response::RegionTierTime>
	for models::RegionTierMetrics
{
	type Error = GlobalError;

	fn try_from(
		value: mm::lobby_runtime_aggregate::response::RegionTierTime,
	) -> GlobalResult<Self> {
		let uptime_in_seconds = util::div_up!(value.total_time, 1_000);

		Ok(models::RegionTierMetrics {
			namespace_id: internal_unwrap!(value.namespace_id).to_string(),
			region_id: internal_unwrap!(value.region_id).to_string(),
			tier_name_id: value.tier_name_id,
			lobby_group_name_id: value.lobby_group_name_id,
			uptime: uptime_in_seconds,
		})
	}
}

impl ApiTryFrom<backend::game::Game> for models::GameHandle {
	type Error = GlobalError;

	fn try_from(value: backend::game::Game) -> GlobalResult<Self> {
		Ok(models::GameHandle {
			game_id: internal_unwrap!(value.game_id).to_string(),
			name_id: value.name_id.to_owned(),
			display_name: value.display_name.to_owned(),
			logo_url: util::route::game_logo(
				value.logo_upload_id.map(|x| *x),
				value.logo_file_name.as_ref(),
			),
			banner_url: util::route::game_banner(
				value.banner_upload_id.map(|x| *x),
				value.banner_file_name.as_ref(),
			),
		})
	}
}

impl ApiTryFrom<perf::SvcPerf> for models::SvcPerf {
	type Error = GlobalError;

	fn try_from(value: perf::SvcPerf) -> GlobalResult<Self> {
		Ok(models::SvcPerf {
			svc_name: value.svc_name.to_owned(),
			ts: util::timestamp::to_chrono(value.ts)?,
			duration: value.duration,
			req_id: value.req_id.map(|req_id| (*req_id).to_string()),
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

impl ApiFrom<job_run::metrics_log::response::Metrics> for models::SvcMetrics {
	fn api_from(value: job_run::metrics_log::response::Metrics) -> models::SvcMetrics {
		models::SvcMetrics {
			job: value.job,
			cpu: value.cpu,
			memory: value.memory.into_iter().map(|v| v as i64).collect(),
			memory_max: value.memory_max.into_iter().map(|v| v as i64).collect(),
			allocated_memory: value.allocated_memory as i64,
		}
	}
}

impl ApiTryFrom<perf::Span> for models::LogsPerfSpan {
	type Error = GlobalError;

	fn try_from(value: perf::Span) -> GlobalResult<Self> {
		Ok(models::LogsPerfSpan {
			label: value.label.to_owned(),
			start_ts: util::timestamp::to_chrono(value.start_ts)?,
			finish_ts: value
				.finish_ts
				.map(util::timestamp::to_chrono)
				.transpose()?,
			req_id: value.req_id.map(|req_id| (*req_id).to_string()),
		})
	}
}

impl ApiTryFrom<perf::Mark> for models::LogsPerfMark {
	type Error = GlobalError;

	fn try_from(value: perf::Mark) -> GlobalResult<Self> {
		Ok(models::LogsPerfMark {
			label: value.label.to_owned(),
			ts: util::timestamp::to_chrono(value.ts)?,
			ray_id: value.ray_id.map(|ray_id| (*ray_id).to_string()),
			req_id: value.req_id.map(|ray_id| (*ray_id).to_string()),
		})
	}
}

impl ApiTryFrom<models::UploadPrepareFile> for backend::upload::PrepareFile {
	type Error = GlobalError;

	fn try_from(value: models::UploadPrepareFile) -> GlobalResult<Self> {
		internal_assert!(value.content_length >= 0);

		Ok(backend::upload::PrepareFile {
			path: value.path,
			mime: value.content_type,
			content_length: value.content_length as u64,
			..Default::default()
		})
	}
}

impl ApiTryFrom<backend::upload::PresignedUploadRequest> for models::UploadPresignedRequest {
	type Error = GlobalError;

	fn try_from(value: backend::upload::PresignedUploadRequest) -> GlobalResult<Self> {
		Ok(models::UploadPresignedRequest {
			path: value.path,
			url: value.url,
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

impl ApiFrom<game::version_validate::response::Error> for models::ValidationError {
	fn api_from(value: game::version_validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

mod openapi {
	use proto::backend::pkg::*;
	use rivet_api::models;
	use rivet_operation::prelude::*;

	use crate::ApiFrom;

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
}

impl ApiFrom<backend::region::Tier> for models::RegionTier {
	fn api_from(value: backend::region::Tier) -> models::RegionTier {
		models::RegionTier {
			tier_name_id: value.tier_name_id.to_owned(),
			rivet_cores_numerator: value.rivet_cores_numerator as i32,
			rivet_cores_denominator: value.rivet_cores_denominator as i32,
			cpu: value.cpu as i64,
			memory: value.memory as i64,
			disk: value.disk as i64,
			bandwidth: value.bandwidth as i64,
		}
	}
}

impl ApiTryFrom<backend::game::Namespace> for models::NamespaceSummary {
	type Error = GlobalError;

	fn try_from(value: backend::game::Namespace) -> GlobalResult<Self> {
		Ok(models::NamespaceSummary {
			namespace_id: internal_unwrap!(value.namespace_id).to_string(),
			create_ts: util::timestamp::to_chrono(value.create_ts)?,
			display_name: value.display_name,
			version_id: internal_unwrap!(value.version_id).to_string(),
			name_id: value.name_id,
		})
	}
}

impl ApiTryFrom<backend::game::Version> for models::VersionSummary {
	type Error = GlobalError;

	fn try_from(value: backend::game::Version) -> GlobalResult<Self> {
		Ok(models::VersionSummary {
			version_id: internal_unwrap!(value.version_id).to_string(),
			create_ts: util::timestamp::to_chrono(value.create_ts)?,
			display_name: value.display_name,
		})
	}
}

impl ApiTryFrom<models::CdnVersionMiddleware> for backend::cdn::Middleware {
	type Error = GlobalError;

	fn try_from(value: models::CdnVersionMiddleware) -> GlobalResult<Self> {
		let kind = match value.kind {
			models::CdnVersionMiddlewareKind::CustomHeaders(custom_headers) => {
				backend::cdn::middleware::Kind::CustomHeaders(custom_headers.try_into()?)
			}
		};

		Ok(backend::cdn::Middleware { kind: Some(kind) })
	}
}

impl ApiTryFrom<models::CdnVersionCustomHeadersMiddleware>
	for backend::cdn::CustomHeadersMiddleware
{
	type Error = GlobalError;

	fn try_from(value: models::CdnVersionCustomHeadersMiddleware) -> GlobalResult<Self> {
		Ok(backend::cdn::CustomHeadersMiddleware {
			headers: value
				.headers
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<models::CdnVersionHeader> for backend::cdn::custom_headers_middleware::Header {
	type Error = GlobalError;

	fn try_from(value: models::CdnVersionHeader) -> GlobalResult<Self> {
		Ok(backend::cdn::custom_headers_middleware::Header {
			name: value.name,
			value: value.value,
		})
	}
}

impl ApiFrom<backend::team::Publicity> for models::GroupPublicity {
	fn api_from(value: backend::team::Publicity) -> models::GroupPublicity {
		match value {
			backend::team::Publicity::Open => models::GroupPublicity::Open,
			backend::team::Publicity::Closed => models::GroupPublicity::Closed,
		}
	}
}

impl ApiFrom<backend::team::dev_team::DevStatus> for models::GroupStatus {
	fn api_from(value: backend::team::dev_team::DevStatus) -> Self {
		use backend::team::dev_team::DevStatus::*;

		match value {
			SetupIncomplete => models::GroupStatus::SetupIncomplete,
			Active => models::GroupStatus::Active,

			PaymentFailed => models::GroupStatus::PaymentFailed,

			SpendingLimitReached => models::GroupStatus::SpendingLimitReached,
		}
	}
}
