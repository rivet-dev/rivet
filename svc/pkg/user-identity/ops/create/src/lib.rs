use email_address_parser::EmailAddress;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "user-identity-create")]
async fn handle(
	ctx: OperationContext<user_identity::create::Request>,
) -> GlobalResult<user_identity::create::Response> {
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();
	let identity = unwrap_ref!(ctx.identity);
	let identity_kind = unwrap_ref!(identity.kind);

	match &identity_kind {
		backend::user_identity::identity::Kind::Email(email) => {
			ensure!(EmailAddress::is_valid(&email.email, None), "invalid email");

			sql_execute!(
				[ctx]
				"
				INSERT INTO db_user_identity.emails (email, user_id, create_ts)
				VALUES ($1, $2, $3)
				",
				&email.email,
				user_id,
				ctx.ts(),
			)
			.await?;

			msg!([ctx] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						name: "user_identity.create".into(),
						user_id: Some(user_id.into()),
						namespace_id: None,
						properties_json: Some(serde_json::to_string(&json!({
							"identity_email": email.email,
						}))?),
						..Default::default()
					}
				],
			})
			.await?;
		}
		backend::user_identity::identity::Kind::AccessToken(access_token) => {
			sql_execute!(
				[ctx]
				"
				INSERT INTO db_user_identity.access_tokens (name, user_id, create_ts)
				VALUES ($1, $2, $3)
				",
				&access_token.name,
				user_id,
				ctx.ts(),
			)
			.await?;

			msg!([ctx] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						name: "user_identity.create".into(),
						user_id: Some(user_id.into()),
						namespace_id: None,
						properties_json: Some(serde_json::to_string(&json!({
							"identity_access_token": access_token.name,
						}))?),
						..Default::default()
					}
				],
			})
			.await?;
		}
	}

	msg!([ctx] user::msg::update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	Ok(user_identity::create::Response {})
}
