use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod matchmaker;

define_router! {
	routes: {
		"matchmaker": {
			GET: matchmaker::status(
				query: matchmaker::StatusQuery,
			),
		}
	},
}
