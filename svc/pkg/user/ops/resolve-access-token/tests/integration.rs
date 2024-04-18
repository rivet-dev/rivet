use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {
		..Default::default()
	})
	.await
	.unwrap();

	let name = util::faker::ident();
	op!([ctx] user_identity_create {
		user_id: user_res.user_id,
		identity: Some(backend::user_identity::Identity {
			kind: Some(backend::user_identity::identity::Kind::AccessToken(
				backend::user_identity::identity::AccessToken {
					name: name.clone(),
				}
			)),
		}),
	})
	.await
	.unwrap();

	let res = op!([ctx] user_resolve_access_token {
		names: vec![name, "bar".to_string()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.users.len());
	let user = res.users.first().unwrap();
	assert_eq!(user_res.user_id, user.user_id);
}
