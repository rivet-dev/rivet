use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use proto::claims;
use redis::AsyncCommands;
use redis_util::escape_search_query;

async fn fail(
	client: &chirp_client::Client,
	party_id: Uuid,
	invite_id: Uuid,
	error_code: party::msg::invite_create_fail::ErrorCode,
) -> GlobalResult<()> {
	msg!([client] party::msg::invite_create_fail(party_id, invite_id) {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		error_code: error_code as i32,
	})
	.await?;
	Ok(())
}

#[worker(name = "party-invite-create")]
async fn worker(ctx: OperationContext<party::msg::invite_create::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	// TODO: Do this in Lua script to prevent race conditions

	let mut redis = ctx.redis_party().await?;

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let invite_id = internal_unwrap!(ctx.invite_id).as_uuid();

	let alias = if let Some(alias) = &ctx.alias {
		Some(util_party::key::party_invite_config::Alias {
			namespace_id: internal_unwrap!(alias.namespace_id).as_uuid(),
			alias: alias.alias.clone(),
		})
	} else {
		None
	};

	// Check party exists
	if !ctx.preemptive_party
		&& !redis
			.exists(util_party::key::party_config(party_id))
			.await?
	{
		return fail(
			ctx.chirp(),
			party_id,
			invite_id,
			party::msg::invite_create_fail::ErrorCode::PartyDoesNotExist,
		)
		.await;
	}

	// Check if invite limit was reached
	let party_invites_res = op!([ctx] party_invite_list {
		party_id: Some(party_id.into()),
	})
	.await?;
	assert_with!(
		party_invites_res.invite_ids.len() < 16,
		PARTY_TOO_MANY_INVITES
	);

	// Check invite alias not already used
	if let Some(alias) = &alias {
		let search_res = redis::cmd("FT.SEARCH")
			.arg("party-invite-idx")
			.arg(format!(
				"(@alias_namespace_id:{{{ns_id}}}) (@alias:{{{alias}}})",
				ns_id = escape_search_query(alias.namespace_id),
				alias = escape_search_query(&alias.alias),
			))
			.arg("NOCONTENT")
			.query_async::<_, redis_util::SearchResultNoContent>(&mut redis)
			.await?;
		if search_res.count > 0 {
			return fail(
				ctx.chirp(),
				party_id,
				invite_id,
				party::msg::invite_create_fail::ErrorCode::AliasNotUnique,
			)
			.await;
		}
	}

	// Create invite token
	let token_res = op!([ctx] token_create {
		issuer: "party-invite-create".into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::days(7),
		}),
		refresh_token_config: None,
		client: ctx.client.clone(),
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				claims::Entitlement {
					kind: Some(
					  claims::entitlement::Kind::PartyInvite(claims::entitlement::PartyInvite {
						  invite_id: Some(invite_id.into()),
					  })
				  )
				}
			],
		})),
		label: Some("invite".into()),
		..Default::default()
	})
	.await?;
	let token = internal_unwrap!(token_res.token);

	redis::cmd("JSON.SET")
		.arg(util_party::key::party_invite_config(invite_id))
		.arg("$")
		.arg(serde_json::to_string(
			&util_party::key::party_invite_config::Config {
				invite_id,
				party_id,
				create_ts: ctx.ts(),
				token: token.token.clone(),
				alias,
			},
		)?)
		.query_async(&mut redis)
		.await?;

	msg!([ctx] party::msg::invite_create_complete(party_id, invite_id) {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		token: token.token.to_owned(),
	})
	.await?;

	if !ctx.preemptive_party {
		msg!([ctx] party::msg::update(party_id) {
			party_id: Some(party_id.into()),
		})
		.await?;
	}

	Ok(())
}
