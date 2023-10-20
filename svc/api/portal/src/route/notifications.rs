use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use rivet_portal_server::models;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

// MARK: POST /notifications/register
pub async fn register(
	ctx: Ctx<Auth>,
	body: models::RegisterNotificationsRequest,
) -> GlobalResult<models::RegisterNotificationsResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	match body.service {
		models::NotificationRegisterService::Firebase(registration) => {
			// TODO: Validate access key JWT
			ensure!(registration.access_key.len() <= 256, "access key too long");

			op!([ctx] user_notification_auth_register {
				user_id: Some(user_ent.user_id.into()),
				registration: Some(user_notification_auth::register::request::Registration::Firebase(
					user_notification_auth::register::request::FirebaseRegistration {
						access_key: registration.access_key,
					},
				)),
			})
			.await?;
		}
	}

	Ok(models::RegisterNotificationsResponse {})
}

// MARK: DELETE /notifications/register
#[derive(Debug, Serialize, Deserialize)]
pub struct UnregisterNotificationsQuery {
	service: models::NotificationUnregisterService,
}

pub async fn unregister(
	ctx: Ctx<Auth>,
	query: UnregisterNotificationsQuery,
) -> GlobalResult<models::UnregisterNotificationsResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	op!([ctx] user_notification_auth_unregister {
		user_id: Some(user_ent.user_id.into()),
		service: ApiInto::<user_notification_auth::unregister::request::Service>::api_into(query.service) as i32,
	})
	.await?;

	Ok(models::UnregisterNotificationsResponse {})
}
