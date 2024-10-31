use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod clusters;
pub mod login;

define_router! {
	routes: {
		"login": {
			POST: login::login(
				body: models::AdminLoginRequest,
				internal_endpoint: true,
			),
		},

		"clusters": {
			GET: clusters::list(
				internal_endpoint: true,
			),
			POST: clusters::create(
				body: models::AdminClustersCreateClusterRequest,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "servers": {
			GET: clusters::servers::list(
				query: clusters::servers::ServerFilterQuery,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "servers" / "taint": {
			POST: clusters::servers::taint(
				query: clusters::servers::ServerFilterQuery,
				body: serde_json::Value,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "servers" / "destroy": {
			POST: clusters::servers::destroy(
				query: clusters::servers::ServerFilterQuery,
				body: serde_json::Value,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "servers" / "lost": {
			GET: clusters::servers::list_lost(
				query: clusters::servers::ServerFilterQuery,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "servers" / "prune": {
			POST: clusters::servers::prune(
				query: clusters::servers::ServerFilterQuery,
				body: serde_json::Value,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "datacenters": {
			GET: clusters::datacenters::list(
				internal_endpoint: true,
			),
			POST: clusters::datacenters::create(
				body: models::AdminClustersCreateDatacenterRequest,
				internal_endpoint: true,
			),
		},

		"clusters" / Uuid / "datacenters" / Uuid : {
			PATCH: clusters::datacenters::update(
				body: models::AdminClustersUpdateDatacenterRequest,
				internal_endpoint: true,
			),
		},

	},
}
