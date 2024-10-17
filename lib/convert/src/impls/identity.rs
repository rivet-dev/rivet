use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiTryFrom};

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

impl ApiFrom<game_user::link_get::response::GameUserLinkStatus> for models::IdentityGameLinkStatus {
	fn api_from(value: game_user::link_get::response::GameUserLinkStatus) -> Self {
		match value {
			game_user::link_get::response::GameUserLinkStatus::Complete => {
				models::IdentityGameLinkStatus::Complete
			}
			game_user::link_get::response::GameUserLinkStatus::Incomplete => {
				models::IdentityGameLinkStatus::Incomplete
			}
			game_user::link_get::response::GameUserLinkStatus::Cancelled => {
				models::IdentityGameLinkStatus::Cancelled
			}
		}
	}
}
