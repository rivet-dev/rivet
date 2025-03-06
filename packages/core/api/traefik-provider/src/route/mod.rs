use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod core;
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
	}
}
