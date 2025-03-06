use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

mod events;
mod identities;

define_router! {
	cors: |config| CorsConfigBuilder::public().build(),
	routes: {
		// Identities
		"identities" / Uuid / "profile": {
			GET: identities::profile(
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "self" / "profile": {
			GET: identities::profile_self(
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
			POST: identities::update_profile(body: models::IdentityUpdateProfileRequest),
		},
		"identities" / "batch" / "handle": {
			GET: identities::handles(
				query: identities::IdentityIdsQuery,
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "batch" / "summary": {
			GET: identities::summaries(
				query: identities::IdentityIdsQuery,
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "self" / "profile" / "validate": {
			POST: identities::validate_profile(body: models::IdentityUpdateProfileRequest),
		},
		"identities" / "avatar-upload" / "prepare": {
			POST: identities::prepare_avatar_upload(body: models::IdentityPrepareAvatarUploadRequest),
		},
		"identities" / "avatar-upload" / Uuid / "complete": {
			POST: identities::complete_avatar_upload(body: serde_json::Value),
		},
		"identities" / "self" / "delete-request": {
			POST: identities::mark_deletion(body: serde_json::Value),
			DELETE: identities::unmark_deletion(),
		},

		// Events
		"events" / "live": {
			GET: events::events(),
		},
	},
}
