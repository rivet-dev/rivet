use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let mut redis = ctx.redis_party().await.unwrap();

	let party_id = create_party(&ctx).await;

	let invite_id = Uuid::new_v4();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> party::msg::invite_create_complete {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: Some(party::msg::invite_create::Alias {
			namespace_id: Some(Uuid::new_v4().into()),
			alias: Uuid::new_v4().to_string(),
		}),
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] party::msg::invite_destroy(invite_id) -> party::msg::update(party_id) {
		invite_id: Some(invite_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	loop {
		if !redis
			.exists::<_, bool>(util_party::key::party_invite_config(invite_id))
			.await
			.unwrap()
		{
			break;
		} else {
			tracing::info!("still exists");
		}
	}
}

async fn create_party(ctx: &TestCtx) -> Uuid {
	let party_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(Uuid::new_v4().into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	party_id
}
