use proto::{backend, common};
use rivet_operation::prelude::*;

pub fn touch_user_presence(ctx: OperationContext<()>, user_id: Uuid) {
	let spawn_res = tokio::task::Builder::new()
		.name("api_party::user_presence_touch")
		.spawn(async move {
			let res = op!([ctx] user_presence_touch {
				user_id: Some(user_id.into()),
			})
			.await;
			match res {
				Ok(_) => {}
				Err(err) => tracing::error!(?err, "failed to touch user presence"),
			}
		});
	if let Err(err) = spawn_res {
		tracing::info!(?err, "failed to spawn user_presence_touch task");
	}
}

pub async fn get_current_party(
	ctx: &OperationContext<()>,
	user_id: Uuid,
) -> GlobalResult<Option<Uuid>> {
	// Fetch the party member if exists
	let party_member_res = op!([ctx] party_member_get {
		user_ids: vec![user_id.into()],
	})
	.await?;
	if let Some(party_member) = party_member_res.party_members.first() {
		let party_id = internal_unwrap!(party_member.party_id).as_uuid();
		Ok(Some(party_id))
	} else {
		Ok(None)
	}
}

pub async fn assert_party_leader(
	ctx: &OperationContext<()>,
	party_id: Uuid,
	user_id: Uuid,
) -> GlobalResult<()> {
	let party_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await?;
	let party = unwrap_with_owned!(party_res.parties.first(), PARTY_IDENTITY_NOT_IN_ANY_PARTY);

	assert_party_leader_with_party(user_id, party)
}

pub fn assert_party_leader_with_party(
	user_id: Uuid,
	party: &backend::party::Party,
) -> GlobalResult<()> {
	let leader_user_id = party.leader_user_id.as_ref().map(common::Uuid::as_uuid);

	assert_eq_with!(
		Some(user_id),
		leader_user_id,
		PARTY_IDENTITY_NOT_PARTY_LEADER
	);

	Ok(())
}
