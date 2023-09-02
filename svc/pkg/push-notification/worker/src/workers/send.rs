use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use fcm::{Client, MessageBuilder, NotificationBuilder};
use serde::Serialize;

const FCM_SERVER_KEY: &str = "AAAAYGT1Pys:APA91bFlpAPLVFqsABrYy-38UZAH6tbbxiaM52FOFQDty3N6Ofwjhy5vv_9GgWer7yytg0OxTzKgxvkrgNKYEWL1CK4kiPCSrAq7eEDVU5mSwPvUB7qraqxjuAPSx-h5eMVeO4KTqIRB";

#[derive(Serialize)]
struct NotificationData {
	url: String,
}

#[worker(name = "push-notification-send")]
async fn worker(
	ctx: &OperationContext<push_notification::msg::create::Message>,
) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let thread_id = internal_unwrap!(ctx.thread_id).as_uuid();
	let service = internal_unwrap_owned!(backend::notification::NotificationService::from_i32(
		ctx.service
	));

	match service {
		// Send push notification through Firebase
		backend::notification::NotificationService::Firebase => {
			let row = sqlx::query_as::<_, (Option<String>,)>(indoc!(
				"
				SELECT
				firebase_access_key
				FROM users
				WHERE user_id = $1
				"
			))
			.bind(user_id)
			.fetch_optional(&ctx.crdb("db-user-notification-auth").await?)
			.await?;

			// Only send notification if registered for Firebase
			if let Some((Some(firebase_access_key),)) = row {
				let client = Client::new();

				let msg = internal_unwrap!(ctx.message);
				let body = internal_unwrap!(msg.body);
				let kind = internal_unwrap!(body.kind);

				match kind {
					backend::chat::message_body::Kind::Text(text) => {
						let mut message_builder =
							MessageBuilder::new(FCM_SERVER_KEY, firebase_access_key.as_str());

						let sender_user_id = internal_unwrap!(text.sender_user_id);
						let user_res = op!([ctx] user_get {
							user_ids: vec![*sender_user_id],
						})
						.await?;
						let user = internal_unwrap_owned!(user_res.users.first());

						let thread_id = thread_id.to_string();

						let title = user.display_name.to_owned();
						let body = util::format::truncate_at_code_point(
							&text.body.chars().collect::<Vec<_>>(),
							1024,
						)?;
						let icon = util::route::user_avatar(&user);
						let click_url = format!("/threads/{}", thread_id);

						let mut notif_builder = NotificationBuilder::new();
						notif_builder.title(&title);
						notif_builder.body(&body);
						notif_builder.icon(&icon);
						if let Some(tag) = ctx.tag.as_ref() {
							notif_builder.tag(tag);
						}
						notif_builder.color("#151515");
						notif_builder.click_action(&click_url);
						// notif_builder.sound(); // TODO
						let notification = notif_builder.finalize();

						message_builder.notification(notification);
						message_builder.collapse_key(&thread_id);

						client.send(message_builder.finalize()).await?;
					}
					_ => {}
				}
			}
		}
	}

	Ok(())
}
