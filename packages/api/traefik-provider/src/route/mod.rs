use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod core;
pub mod game_guard;
pub mod tunnel;

define_router! {
	routes: {
		"config" / "core": {
			GET: core::config(
				query: core::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		},
		"config" / "tunnel": {
			GET: tunnel::config(
				query: tunnel::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		},
		"config" / "game-guard": {
			GET: game_guard::config(
				query: game_guard::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		}
	}
}
