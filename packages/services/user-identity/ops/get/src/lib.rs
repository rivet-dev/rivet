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
		.fetch_all_proto("user_identity.identity", user_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, user_ids| {
				let ctx = ctx.clone();
				async move {
					let identity_rows = sql_fetch_all!(
						[ctx, IdentityRow]
						"
						SELECT e.user_id AS user_id, e.email
						FROM db_user_identity.emails as e
						WHERE e.user_id = ANY($1)
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
		.map(|user_id| {
			let identities = identities
				.iter()
				.filter(|x| {
					x.user_id
						.as_ref()
						.map_or(false, |x| x.as_uuid() == *user_id)
				})
				.flat_map(|x| {
					IntoIterator::into_iter([x.email.as_ref().map(|email| {
						backend::user_identity::Identity {
							kind: Some(backend::user_identity::identity::Kind::Email(
								backend::user_identity::identity::Email {
									email: email.clone(),
								},
							)),
						}
					})])
					.flatten()
				})
				.collect::<Vec<_>>();

			user_identity::get::response::User {
				user_id: Some((*user_id).into()),
				identities,
			}
		})
		.collect::<Vec<_>>();

	Ok(user_identity::get::Response { users })
}
