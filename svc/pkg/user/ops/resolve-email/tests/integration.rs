use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {
		..Default::default()
	})
	.await
	.unwrap();

	let email = util::faker::email();
	op!([ctx] user_identity_create {
		user_id: user_res.user_id,
		identity: Some(backend::user_identity::Identity {
			kind: Some(backend::user_identity::identity::Kind::Email(
				backend::user_identity::identity::Email {
					email: email.clone()
				}
			)),
		}),
	})
	.await
	.unwrap();

	let res = op!([ctx] user_resolve_email {
		emails: vec![email.clone(), util::faker::email()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.users.len());
	let user = res.users.first().unwrap();
	assert_eq!(user_res.user_id, user.user_id);
}
