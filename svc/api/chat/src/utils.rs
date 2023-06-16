use rivet_operation::prelude::*;

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
