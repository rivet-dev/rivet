use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_job_server::models;

pub mod run;

define_router! {
	routes: {
		"runs" / "cleanup": {
			POST: run::cleanup(
				body: models::CleanupRequest,
				internal_endpoint: true,
			),
		},
	},
}
