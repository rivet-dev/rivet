use api_helper::define_router;
use hyper::{Body, Request, Response};
use uuid::Uuid;

pub mod pegboard;

define_router! {
	routes: {
		"pegboard" / "client" / Uuid / "registered": {
			POST: pegboard::client_registered(
				body: serde_json::Value,
				internal_endpoint: true,
				opt_auth: true,
			),
		}
	},
}
