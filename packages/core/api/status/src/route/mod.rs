use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod actor_isolate;
pub mod matchmaker;

define_router! {
	routes: {
		"matchmaker": {
			GET: matchmaker::status(
				query: matchmaker::StatusQuery,
			),
		},
		"actor" / "isolate": {
			GET: actor_isolate::status(
				query: actor_isolate::StatusQuery,
			),
		},
	},
}
