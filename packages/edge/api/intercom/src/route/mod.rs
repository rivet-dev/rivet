use api_helper::define_router;
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod pegboard;

define_router! {
	db_driver: chirp_workflow::db::DatabaseFdbSqliteNats,
	routes: {
		"pegboard" / "image" / Uuid / "prewarm": {
			POST: pegboard::prewarm_image(
				internal_endpoint: true,
				opt_auth: true,
				body: models::EdgeIntercomPegboardPrewarmImageRequest,
			),
		},

		"pegboard" / "client" / Uuid / "toggle-drain": {
			POST: pegboard::toggle_drain_client(
				internal_endpoint: true,
				opt_auth: true,
				body: models::EdgeIntercomPegboardToggleClientDrainRequest,
			),
		}
	},
}
