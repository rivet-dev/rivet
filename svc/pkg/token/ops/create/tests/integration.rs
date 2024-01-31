use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use rivet_claims::ClaimsDecode;

#[worker_test]
async fn new_and_reload(ctx: TestCtx) {
	let user_id = Uuid::new_v4();

	// Create the token
	let new_res = op!([ctx] token_create {
		issuer: "gg.rivet.test".to_owned(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(123),
		}),
		refresh_token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(456),
		}),
		client: Some(backend::net::ClientInfo {
			user_agent: Some("Hello World".to_owned()),
			remote_address: Some("1.2.3.4".to_owned()),
		}),
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
							user_id: Some(user_id.into())
						})),
					}
				]
			},
		)),
		label: Some("usr".into()),
		..Default::default()
	})
	.await
	.expect("create new token");

	let _ = rivet_claims::decode(&new_res.token.as_ref().unwrap().token).expect("decode new token");
	let _ = rivet_claims::decode(&new_res.refresh_token.as_ref().unwrap().token)
		.expect("decode refresh token");
	assert_eq!(
		"usr",
		new_res
			.token
			.as_ref()
			.unwrap()
			.token
			.split_once('.')
			.unwrap()
			.0
	);
	assert_eq!(
		"usr_rf",
		new_res
			.refresh_token
			.as_ref()
			.unwrap()
			.token
			.split_once('.')
			.unwrap()
			.0
	);

	// Refresh the token
	op!([ctx] token_create {
		issuer: "gg.rivet.test".to_owned(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(456),
		}),
		refresh_token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(789),
		}),
		client: Some(backend::net::ClientInfo {
			user_agent: Some("Howdy Cowboy".to_owned()),
			remote_address: Some("5.6.7.8".to_owned()),
		}),
		kind: Some(token::create::request::Kind::Refresh(
			token::create::request::KindRefresh {
				refresh_token: new_res.refresh_token.as_ref().unwrap().token.clone(),
			},
		)),
		label: Some("rfr".into()),
		..Default::default()
	})
	.await
	.expect("create new token");

	// Decode and validate the tokens
	{
		let user_token = rivet_claims::decode(&new_res.token.as_ref().unwrap().token)
			.expect("decode user token")
			.expect("validate user token");
		let user_ent = user_token.as_user().unwrap();
		assert_eq!(user_id, user_ent.user_id, "mismatching user id");

		let refresh_token = rivet_claims::decode(&new_res.refresh_token.as_ref().unwrap().token)
			.expect("decode user token")
			.expect("validate refresh token");
		let _refresh_ent = refresh_token.as_refresh().unwrap();
	}
}

#[worker_test]
async fn new_and_reload_combined(ctx: TestCtx) {
	let user_id = Uuid::new_v4();

	// Create the token
	let new_res = op!([ctx] token_create {
		issuer: "gg.rivet.test".to_owned(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(123),
		}),
		refresh_token_config: None,
		client: Some(backend::net::ClientInfo {
			user_agent: Some("Hello World".to_owned()),
			remote_address: Some("1.2.3.4".to_owned()),
		}),
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
							user_id: Some(user_id.into())
						})),
					}
				]
			},
		)),
		label: Some("usr".into()),
		combine_refresh_token: true,
		..Default::default()
	})
	.await
	.expect("create new token");

	let _ = rivet_claims::decode(&new_res.token.as_ref().unwrap().token).expect("decode new token");
	assert_eq!(
		"usr",
		new_res
			.token
			.as_ref()
			.unwrap()
			.token
			.split_once('.')
			.unwrap()
			.0
	);
	assert!(new_res.refresh_token.is_none());

	// Refresh the token
	op!([ctx] token_create {
		issuer: "gg.rivet.test".to_owned(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(456),
		}),
		refresh_token_config: None,
		client: Some(backend::net::ClientInfo {
			user_agent: Some("Howdy Cowboy".to_owned()),
			remote_address: Some("5.6.7.8".to_owned()),
		}),
		kind: Some(token::create::request::Kind::Refresh(
			token::create::request::KindRefresh {
				refresh_token: new_res.token.as_ref().unwrap().token.clone(),
			},
		)),
		label: Some("usr".into()),
		combine_refresh_token: true,
		..Default::default()
	})
	.await
	.expect("create new token");

	// Decode and validate the tokens
	{
		let user_token = rivet_claims::decode(&new_res.token.as_ref().unwrap().token)
			.expect("decode user token")
			.expect("validate user token");
		let user_ent = user_token.as_user().unwrap();
		assert_eq!(user_id, user_ent.user_id, "mismatching user id");
		assert!(new_res.refresh_token.is_none());
	}
}
