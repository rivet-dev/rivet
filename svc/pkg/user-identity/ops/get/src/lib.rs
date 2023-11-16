use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct EmailRow {
	user_id: Uuid,
	email: String,
}

impl From<EmailRow> for user_identity::get::CacheUserEmailIdentity {
	fn from(val: EmailRow) -> Self {
		user_identity::get::CacheUserEmailIdentity {
			user_id: Some(val.user_id.into()),
			email: val.email,
		}
	}
}

#[derive(Debug, sqlx::FromRow)]
struct AccessTokenRow {
	user_id: Uuid,
	name: String,
}

impl From<AccessTokenRow> for user_identity::get::CacheUserAccessTokenIdentity {
	fn from(val: AccessTokenRow) -> Self {
		user_identity::get::CacheUserAccessTokenIdentity {
			user_id: Some(val.user_id.into()),
			name: val.name,
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

	let emails = ctx
		.cache()
		.fetch_all_proto("user_identity.emails", user_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, user_ids| {
				let ctx = ctx.clone();
				async move {
					let identity_rows = sql_fetch_all!(
						[ctx, EmailRow]
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
							user_identity::get::CacheUserEmailIdentity::from(row),
						);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	let access_tokens = ctx
		.cache()
		.fetch_all_proto("user_identity.access_tokens", user_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, user_ids| {
				let ctx = ctx.clone();
				async move {
					let identity_rows = sql_fetch_all!(
						[ctx, AccessTokenRow]
						"
							SELECT user_id, name
							FROM db_user_identity.access_tokens
							WHERE user_id = ANY($1)
							",
						&user_ids,
					)
					.await?;

					for row in identity_rows {
						cache.resolve(
							&row.user_id.clone(),
							user_identity::get::CacheUserAccessTokenIdentity::from(row),
						);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	let users = user_ids
		.iter()
		.map(|user_id| {
			let emails = emails
				.iter()
				.filter(|x| {
					x.user_id
						.as_ref()
						.map_or(false, |x| x.as_uuid() == *user_id)
				})
				.map(|x| backend::user_identity::Identity {
					kind: Some(backend::user_identity::identity::Kind::Email(
						backend::user_identity::identity::Email {
							email: x.email.clone(),
						},
					)),
				});

			let access_tokens = access_tokens
				.iter()
				.filter(|x| {
					x.user_id
						.as_ref()
						.map_or(false, |x| x.as_uuid() == *user_id)
				})
				.map(|x| backend::user_identity::Identity {
					kind: Some(backend::user_identity::identity::Kind::AccessToken(
						backend::user_identity::identity::AccessToken {
							name: x.name.clone(),
						},
					)),
				});

			user_identity::get::response::User {
				user_id: Some((*user_id).into()),
				// Find all matching identities
				identities: emails.chain(access_tokens).collect::<Vec<_>>(),
			}
		})
		.collect::<Vec<_>>();

	Ok(user_identity::get::Response { users })
}
