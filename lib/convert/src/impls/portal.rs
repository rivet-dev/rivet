use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use rivet_portal_server::models;

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
			_ => backend::team::Publicity::Closed,
		}
	}
}

impl ApiFrom<models::NotificationUnregisterService>
	for user_notification_auth::unregister::request::Service
{
	fn api_from(value: models::NotificationUnregisterService) -> Self {
		match value {
			models::NotificationUnregisterService::Firebase => {
				user_notification_auth::unregister::request::Service::Firebase
			}
			_ => user_notification_auth::unregister::request::Service::Firebase,
		}
	}
}
