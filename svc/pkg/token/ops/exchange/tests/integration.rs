use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
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
	let jti = create_res
		.token
		.as_ref()
		.unwrap()
		.jti
		.as_ref()
		.unwrap()
		.as_uuid();

	op!([ctx] token_exchange {
		jti: Some(jti.into()),
	})
	.await
	.unwrap();

	let (revoke_ts,) =
		sqlx::query_as::<_, (Option<i64>,)>("SELECT revoke_ts FROM tokens WHERE jti = $1")
			.bind(jti)
			.fetch_one(&ctx.crdb("db-token").await.unwrap())
			.await
			.unwrap();
	assert!(revoke_ts.is_some());

	op!([ctx] token_exchange {
		jti: Some(jti.into()),
	})
	.await
	.unwrap_err();
}
