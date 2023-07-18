use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-mm-player")]
async fn handle(
	ctx: OperationContext<faker::mm_player::Request>,
) -> GlobalResult<faker::mm_player::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	let fake_ip = util::faker::ip_addr_v4();
	let client = backend::net::ClientInfo {
		user_agent: Some("Test".into()),
		remote_address: Some(fake_ip.to_string()),
	};

	// Generate token
	let player_id = Uuid::new_v4();
	let token_res = op!([ctx] token_create {
		issuer: Self::NAME.into(),
		token_config: Some(token::create::request::TokenConfig {
			// Has to be greater than the player register time since this
			// token is used in the player disconnect too.
			ttl: util::duration::days(90),
		}),
		refresh_token_config: None,
		client: Some(client.clone()),
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
					  proto::claims::entitlement::Kind::MatchmakerPlayer(proto::claims::entitlement::MatchmakerPlayer {
						  player_id: Some(player_id.into()),
					  })
				  )
				}
			],
		})),
		label: Some("player".into()),
		..Default::default()
	})
	.await?;
	let token = internal_unwrap!(token_res.token);
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

	// Find lobby
	let query_id = Uuid::new_v4();
	let find_res = msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id)
		-> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail>
	{
		namespace_id: ctx.namespace_id,
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(token_session_id.into()),
			client_info: Some(client.clone()),
		}],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: ctx.lobby_id,
		})),
		..Default::default()
	})
	.await?;

	// Check error code
	match find_res.map_err(|msg| mm::msg::lobby_find_fail::ErrorCode::from_i32(msg.error_code)) {
		Ok(_) => {}
		Err(Some(code)) => {
			use mm::msg::lobby_find_fail::ErrorCode::*;

			match code {
				Unknown => internal_panic!("unknown find error code"),
				StaleMessage => panic_with!(CHIRP_STALE_MESSAGE),
				TooManyPlayersFromSource => panic_with!(MATCHMAKER_TOO_MANY_PLAYERS_FROM_SOURCE),

				LobbyStopped | LobbyStoppedPrematurely => panic_with!(MATCHMAKER_LOBBY_STOPPED),
				LobbyClosed => panic_with!(MATCHMAKER_LOBBY_CLOSED),
				LobbyNotFound => panic_with!(MATCHMAKER_LOBBY_NOT_FOUND),
				NoAvailableLobbies => panic_with!(MATCHMAKER_NO_AVAILABLE_LOBBIES),
				LobbyFull => panic_with!(MATCHMAKER_LOBBY_FULL),
				LobbyCountOverMax => panic_with!(MATCHMAKER_TOO_MANY_LOBBIES),
				RegionNotEnabled => panic_with!(MATCHMAKER_REGION_NOT_ENABLED_FOR_GAME_MODE),

				DevTeamInvalidStatus => panic_with!(GROUP_INVALID_DEVELOPER_STATUS),

				FindDisabled => panic_with!(MATCHMAKER_FIND_DISABLED),
				JoinDisabled => panic_with!(MATCHMAKER_JOIN_DISABLED),
				VerificationFailed => panic_with!(MATCHMAKER_VERIFICATION_FAILED),
				VerificationRequestFailed => panic_with!(MATCHMAKER_VERIFICATION_REQUEST_FAILED),
				IdentityRequired => panic_with!(MATCHMAKER_IDENTITY_REQUIRED),
				RegistrationRequired => panic_with!(MATCHMAKER_REGISTRATION_REQUIRED),
			};
		}
		Err(None) => internal_panic!("failed to parse find error code"),
	}

	Ok(faker::mm_player::Response {
		player_id: Some(player_id.into()),
		player_token: token.token.clone(),
		token_session_id: Some(token_session_id.into()),
	})
}
