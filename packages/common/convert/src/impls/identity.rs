use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiTryFrom;

impl ApiTryFrom<backend::user_identity::Identity> for models::IdentityLinkedAccount {
	type Error = GlobalError;

	fn api_try_from(
		value: backend::user_identity::Identity,
	) -> GlobalResult<models::IdentityLinkedAccount> {
		match unwrap_ref!(value.kind) {
			backend::user_identity::identity::Kind::Email(email_ident) => {
				Ok(models::IdentityLinkedAccount {
					email: Some(Box::new(models::IdentityEmailLinkedAccount {
						email: email_ident.email.to_owned(),
					})),
					..Default::default()
				})
			}
			backend::user_identity::identity::Kind::AccessToken(access_token_ident) => {
				Ok(models::IdentityLinkedAccount {
					access_token: Some(Box::new(models::IdentityAccessTokenLinkedAccount {
						name: access_token_ident.name.to_owned(),
					})),
					..Default::default()
				})
			}
		}
	}
}
