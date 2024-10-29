use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_portal_server::models;

mod games;

define_router! {
	cors: |config| CorsConfigBuilder::hub(config).build(),
	routes: {
		// Games
		"games": {
			GET: games::get_suggested_games(),
		},
		"games" / String / "profile": {
			GET: games::profile(),
		},
	},
}
