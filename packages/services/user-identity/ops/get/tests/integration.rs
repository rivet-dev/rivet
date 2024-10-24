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

	let res = op!([ctx] user_identity_get {
		user_ids: vec![user_res.user_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(2, res.users.len());
	assert_eq!(
		1,
		res.users
			.iter()
			.find(|u| u.user_id == user_res.user_id)
			.unwrap()
			.identities
			.len()
	);
}
