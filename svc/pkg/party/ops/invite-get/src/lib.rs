use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "party-invite-get")]
async fn handle(
	ctx: OperationContext<party::invite_get::Request>,
) -> GlobalResult<party::invite_get::Response> {
	// TODO:
	return Ok(party::invite_get::Response {
		invites: Vec::new(),
	});

	let mut redis = ctx.redis_party().await?;
	let invite_ids = ctx
		.invite_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Query all party invites
	let mut pipe = redis::pipe();
	for &invite_id in &invite_ids {
		pipe.cmd("JSON.GET")
			.arg(util_party::key::party_invite_config(invite_id));
	}

	// Fetch & convert party invites
	let invites = pipe
		.query_async::<_, Vec<Option<String>>>(&mut redis)
		.await?
		.into_iter()
		.zip(invite_ids.iter())
		.filter_map(|(json, invite_id)| json.map(|json| (json, invite_id)))
		.filter_map(|(json, &invite_id)| {
			match serde_json::from_str::<util_party::key::party_invite_config::Config>(&json) {
				Ok(x) => Some((invite_id, x)),
				Err(err) => {
					tracing::error!(
						?invite_id,
						?err,
						?json,
						"failed to parse party invite config"
					);
					None
				}
			}
		})
		.map(|(invite_id, data)| backend::party::Invite {
			invite_id: Some(invite_id.into()),
			party_id: Some(data.party_id.into()),
			create_ts: data.create_ts,
			token: data.token,
			alias: data.alias.map(|alias| backend::party::InviteAlias {
				namespace_id: Some(alias.namespace_id.into()),
				alias: alias.alias,
			}),
		})
		.collect::<Vec<_>>();

	Ok(party::invite_get::Response { invites })
}
