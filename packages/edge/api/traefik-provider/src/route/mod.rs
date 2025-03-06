use api_helper::define_router;
use hyper::{Body, Request, Response};

pub mod game_guard;

define_router! {
	db_driver: chirp_workflow::db::DatabaseFdbSqliteNats,
	routes: {
		"config" / "game-guard": {
			GET: game_guard::config(
				query: game_guard::ConfigQuery,
				internal_endpoint: true,
				opt_auth: true,
			),
		}
	}
}
