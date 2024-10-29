use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod actors;
pub mod builds;
pub mod dc;
pub mod logs;

define_router! {
	cors: |config| CorsConfigBuilder::hub(config).build(),
	routes: {
		// MARK: Actors
		"games" / Uuid / "environments" / Uuid / "actor": {
			GET: actors::list_actors(
				query: actors::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create(
				body: models::ActorCreateActorRequest,
				rate_limit: {
					buckets: [
						{ count: 1_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		// MARK: Servers (LEGACY)
		"games" / Uuid / "environments" / Uuid / "servers": {
			GET: actors::list_actors(
				query: actors::ListQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
			POST: actors::create(
				body: models::ActorCreateActorRequest,
				rate_limit: {
					buckets: [
						{ count: 1_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "servers" / Uuid: {
			GET: actors::get(
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
		"games" / Uuid / "environments" / Uuid / "servers" / Uuid / "logs" : {
			GET: logs::get_logs(
				query: logs::GetActorLogsQuery,
			),
		},

		// MARK: Logs
		"games" / Uuid / "environments" / Uuid / "actor" / Uuid / "logs" : {
			GET: logs::get_logs(
				query: logs::GetActorLogsQuery,
			),
		},

		// MARK: Builds
		"games" / Uuid / "environments" / Uuid / "builds": {
			GET: builds::list(
				query: builds::GetQuery,
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid: {
			GET: builds::get(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "tags": {
			PATCH: builds::patch_tags(body: models::ActorPatchBuildTagsRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / "prepare": {
			POST: builds::create_build(body: models::ActorCreateBuildRequest),
		},

		"games" / Uuid / "environments" / Uuid / "builds" / Uuid / "complete": {
			POST: builds::complete_build(body: serde_json::Value),
		},

		// MARK: Datacenters
		"games" / Uuid / "environments" / Uuid / "datacenters": {
			GET: dc::list(
				rate_limit: {
					buckets: [
						{ count: 60_000, bucket: duration::minutes(1) },
					],
				},
			),
		},
	},
}
