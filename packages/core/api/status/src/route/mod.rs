use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod actor;
pub mod matchmaker;

define_router! {
	routes: {
		"matchmaker": {
			GET: matchmaker::status(
				query: matchmaker::StatusQuery,
			),
		},
		"actor": {
			GET: actor::status(
				query: actor::StatusQuery,
			),
		},
	},
}
