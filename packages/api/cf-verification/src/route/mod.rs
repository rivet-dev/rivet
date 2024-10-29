use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_operation::prelude::*;

pub mod verification;

define_router! {
	routes: {
		".well-known" / "cf-custom-hostname-challenge" / Uuid: {
			GET: verification::verify_custom_hostname(
				opt_auth: true,
				returns_bytes: true,
			),
		},
	},
}
