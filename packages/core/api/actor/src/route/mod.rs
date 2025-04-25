use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

pub mod actors;
pub mod builds;
pub mod logs;
pub mod regions;
pub mod routes;

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalQuery {
	/// Slug of the project.
	///
	/// If provided, `environment` must also be provided.
	pub project: Option<String>,

	/// Slug of the environment.
	pub environment: Option<String>,
}

impl GlobalQuery {
	/// Returns both the project and environment.
	///
	/// Validates that the project can only be specified with the environment.
	pub fn project_and_env(&self) -> GlobalResult<(Option<&str>, Option<&str>)> {
		if let Some(environment) = &self.environment {
			Ok((self.project.as_deref(), Some(environment)))
		} else {
			// Don't allow just the project
			if self.project.is_some() {
				bail_with!(
					API_BAD_QUERY,
					parameter = "project",
					error = "Must provide both `project` and `environment` query together."
				)
			} else {
				Ok((None, None))
			}
		}
	}
}

define_router! {
	cors: |_config| CorsConfigBuilder::public().build(),
	routes: {
		"actors": {
			GET: actors::list_actors(
				query: actors::ListQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create(
				query: actors::GlobalEndpointTypeQuery,
				body: models::ActorsCreateActorRequest,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"actors" / "upgrade": {
			POST: actors::upgrade_all(
				query: GlobalQuery,
				body: models::ActorsUpgradeAllActorsRequest,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"actors" / Uuid: {
			GET: actors::get(
				query: actors::GlobalEndpointTypeQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},

			),
			DELETE: actors::destroy(
				query: actors::DeleteQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"actors" / Uuid / "upgrade": {
			POST: actors::upgrade(
				query: GlobalQuery,
				body: models::ActorsUpgradeActorRequest,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"actors" / "logs": {
			GET: logs::get_logs(
				query: logs::GetActorLogsQuery,
				opt_auth: true,
			),
		},


		"builds": {
			GET: builds::list(
				query: builds::ListQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"builds" / Uuid: {
			GET: builds::get(
				query: GlobalQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"builds" / Uuid / "tags": {
			PATCH: builds::patch_tags(
				query: GlobalQuery,
				body: models::BuildsPatchBuildTagsRequest,
				opt_auth: true,
			),
		},

		"builds" / "prepare": {
			POST: builds::create_build(
				query: GlobalQuery,
				body: models::BuildsPrepareBuildRequest,
				opt_auth: true,
			),
		},

		"builds" / Uuid / "complete": {
			POST: builds::complete_build(
				query: GlobalQuery,
				body: serde_json::Value,
				opt_auth: true,
			),
		},

		// MARK: Regions
		"regions": {
			GET: regions::list(
				query: GlobalQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},
		"regions" / "recommend": {
			GET: regions::recommend(
				query: regions::RecommendQuery,
				opt_auth: true,
			),
		},

		// MARK: Routes
		"routes": {
			GET: routes::list(
				query: routes::ListQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"routes" / String: {
			PUT: routes::update(
				query: GlobalQuery,
				body: models::RoutesUpdateRouteBody,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
			DELETE: routes::delete(
				query: GlobalQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		// MARK: Deprecated
		"regions" / "resolve": {
			GET: regions::recommend(
				query: regions::RecommendQuery,
				opt_auth: true,
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers": {
			GET: actors::list_servers_deprecated(
				query: actors::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create_deprecated(
				body: models::ServersCreateServerRequest,
				rate_limit: {
					buckets: [
						{ count: 1_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers" / Uuid: {
			GET: actors::get_deprecated(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},

			),
			DELETE: actors::destroy_deprecated(
				query: actors::DeleteQuery,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers" / Uuid / "logs" : {
			GET: logs::get_logs_deprecated(
				query: logs::GetActorLogsQuery,
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds": {
			GET: builds::list_deprecated(
				query: builds::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid: {
			GET: builds::get_deprecated(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "tags": {
			PATCH: builds::patch_tags_deprecated(body: models::ServersPatchBuildTagsRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / "prepare": {
			POST: builds::create_build_deprecated(body: models::ServersCreateBuildRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "complete": {
			POST: builds::complete_build_deprecated(body: serde_json::Value),
		},

		"games" / Uuid / "environments" / Uuid / "datacenters": {
			GET: regions::list_deprecated(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},
	},
}
