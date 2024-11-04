use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use serde::Deserialize;
use uuid::Uuid;

pub mod actors;
pub mod builds;
pub mod dc;
pub mod logs;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GlobalQuery {
	Nothing,
	Environment {
		/// Slug of the environment.
		environment: String,
	},
	ProjectAndEnvironment {
		/// Slug of the project.
		project: String,
		/// Slug of the environment.
		environment: String,
	},
}

impl GlobalQuery {
	pub fn project(&self) -> Option<&str> {
		match self {
			Self::ProjectAndEnvironment { project, .. } => Some(&project),
			_ => None,
		}
	}

	pub fn environment(&self) -> Option<&str> {
		match self {
			Self::ProjectAndEnvironment { environment, .. } | Self::Environment { environment } => {
				Some(&environment)
			}
			_ => None,
		}
	}
}

define_router! {
	cors: |config| CorsConfigBuilder::hub(config).build(),
	routes: {
		"actors": {
			GET: actors::list_actors(
				query: actors::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create(
				query: GlobalQuery,
				body: models::ActorCreateActorRequest,
				rate_limit: {
					buckets: [
						{ count: 1_000, bucket: duration::minutes(1) },
					],
				},
			),
		},


		"actors" / Uuid: {
			GET: actors::get(
				query: GlobalQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},

			),
			DELETE: actors::destroy(
				query: actors::DeleteQuery,
				rate_limit: {
					buckets: [
						{ count: 10_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"actors" / Uuid / "logs" : {
			GET: logs::get_logs(
				query: logs::GetActorLogsQuery,
			),
		},


		"builds": {
			GET: builds::list(
				query: builds::ListQuery,
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
				body: models::ActorPatchBuildTagsRequest
			),
		},

		"builds" / "prepare": {
			POST: builds::create_build(
				query: GlobalQuery,
				body: models::ActorCreateBuildRequest
			),
		},

		"builds" / Uuid / "complete": {
			POST: builds::complete_build(
				query: GlobalQuery,
				body: serde_json::Value
			),
		},

		// MARK: Datacenters
		"datacenters": {
			GET: dc::list(
				query: GlobalQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		// MARK: Deprecated
		"games" / Uuid / "environments" / Uuid / "servers": {
			GET: actors::list_actors_deprecated(
				query: actors::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create_deprecated(
				body: models::ActorCreateActorRequest,
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
			PATCH: builds::patch_tags_deprecated(body: models::ActorPatchBuildTagsRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / "prepare": {
			POST: builds::create_build_deprecated(body: models::ActorCreateBuildRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "complete": {
			POST: builds::complete_build_deprecated(body: serde_json::Value),
		},

		"games" / Uuid / "environments" / Uuid / "datacenters": {
			GET: dc::list_deprecated(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},
	},
}
