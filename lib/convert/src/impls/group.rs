use proto::backend::{self, pkg::*};
use rivet_group_server::models;
use rivet_operation::prelude::*;

use crate::ApiFrom;

impl ApiFrom<backend::team::Publicity> for models::GroupPublicity {
	fn api_from(value: backend::team::Publicity) -> models::GroupPublicity {
		match value {
			backend::team::Publicity::Open => models::GroupPublicity::Open,
			backend::team::Publicity::Closed => models::GroupPublicity::Closed,
		}
	}
}

impl ApiFrom<models::GroupPublicity> for backend::team::Publicity {
	fn api_from(value: models::GroupPublicity) -> backend::team::Publicity {
		match value {
			models::GroupPublicity::Open => backend::team::Publicity::Open,
			models::GroupPublicity::Closed => backend::team::Publicity::Closed,
			models::GroupPublicity::Unknown(_) => backend::team::Publicity::Closed,
		}
	}
}

impl ApiFrom<team::profile_validate::response::Error> for models::ValidationError {
	fn api_from(value: team::profile_validate::response::Error) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}
