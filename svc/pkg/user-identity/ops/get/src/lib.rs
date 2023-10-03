use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct IdentityRow {
	user_id: Uuid,

	email: Option<String>,
}

#[operation(name = "user-identity-get")]
async fn handle(
	ctx: OperationContext<user_identity::get::Request>,
) -> GlobalResult<user_identity::get::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let identity_rows = sqlx::query_as::<_, IdentityRow>(indoc!(
		"
		SELECT user_id, email
		FROM db_user_identity.emails
		WHERE user_id = ANY($1)
	"
	))
	.bind(&user_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let users = user_ids
		.iter()
		.map(|user_id| user_identity::get::response::User {
			user_id: Some((*user_id).into()),
			// Find all matching identities
			identities: identity_rows
				.iter()
				.filter(|x| x.user_id == *user_id)
				.filter_map(|identity| {
					if let Some(email) = &identity.email {
						Some(backend::user_identity::identity::Kind::Email(
							backend::user_identity::identity::Email {
								email: email.clone(),
							},
						))
					} else {
						tracing::warn!(?identity, "unmatched identity");
						None
					}
				})
				.map(|kind| backend::user_identity::Identity { kind: Some(kind) })
				.collect::<Vec<_>>(),
		})
		.collect::<Vec<_>>();

	Ok(user_identity::get::Response { users })
}
