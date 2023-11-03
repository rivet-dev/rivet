use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct IdentityRow {
	user_id: Uuid,
	email: Option<String>,
}

impl From<IdentityRow> for user_identity::get::CacheUserIdentity {
	fn from(val: IdentityRow) -> Self {
		user_identity::get::CacheUserIdentity {
			user_id: Some(val.user_id.into()),
			email: val.email,
		}
	}
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

	let identities = ctx
		.cache()
		.fetch_all_proto("user_identity.emails", user_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, user_ids| {
				let ctx = ctx.clone();
				async move {
					let identity_rows = sql_fetch_all!(
						[ctx, IdentityRow]
						"
						SELECT user_id, email
						FROM db_user_identity.emails
						WHERE user_id = ANY($1)
						",
						&user_ids,
					)
					.await?;

					for row in identity_rows {
						cache.resolve(
							&row.user_id.clone(),
							user_identity::get::CacheUserIdentity::from(row),
						);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	let users = user_ids
		.iter()
		.map(|user_id| user_identity::get::response::User {
			user_id: Some((*user_id).into()),
			// Find all matching identities
			identities: identities
				.iter()
				.filter(|x| {
					x.user_id
						.as_ref()
						.map_or(false, |x| x.as_uuid() == *user_id)
				})
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
