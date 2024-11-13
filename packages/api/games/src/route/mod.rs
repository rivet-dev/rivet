use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use uuid::Uuid;

pub mod envs;

define_router! {
	cors: |config| CorsConfigBuilder::hub(config).build(),
	routes: {
		"games" / Uuid / "environments" / Uuid / "tokens" / "service": {
			POST: envs::tokens::create_service(
				body: serde_json::Value
			),
		},
	},
}
