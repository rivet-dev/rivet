use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

pub mod actors;
pub mod logs;

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
	db_driver: chirp_workflow::db::DatabaseFdbSqliteNats,
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

		"actors" / Uuid / "logs": {
			GET: logs::get_logs(
				query: logs::GetActorLogsQuery,
				opt_auth: true,
			),
		},
	},
}
