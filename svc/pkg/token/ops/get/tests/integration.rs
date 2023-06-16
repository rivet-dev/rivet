use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let create_res = op!([ctx] token_create {
		issuer: "test".into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::seconds(30),
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(
				token::create::request::KindNew {
					entitlements: Vec::new(),
				}
				)),
				label: Some("empty".into()),
				..Default::default()
	})
	.await
	.unwrap();
	let jti = create_res.token.as_ref().unwrap().jti.unwrap();

	let res = op!([ctx] token_get {
		jtis: vec![jti],
	})
	.await
	.unwrap();

	assert_eq!(1, res.tokens.len())
}
