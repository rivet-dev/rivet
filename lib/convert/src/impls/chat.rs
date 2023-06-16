use proto::backend;
use rivet_chat_server::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiTryFrom};

impl ApiTryFrom<backend::chat::TypingStatus> for models::ChatTypingStatus {
	type Error = GlobalError;

	fn try_from(value: backend::chat::TypingStatus) -> GlobalResult<Self> {
		let kind = internal_unwrap!(value.kind);

		match kind {
			backend::chat::typing_status::Kind::Idle(_) => {
				Ok(models::ChatTypingStatus::Idle(models::Unit {}))
			}
			backend::chat::typing_status::Kind::Typing(_) => {
				Ok(models::ChatTypingStatus::Typing(models::Unit {}))
			}
		}
	}
}

impl ApiTryFrom<backend::chat::Topic> for models::ChatSimpleTopic {
	type Error = GlobalError;

	fn try_from(value: backend::chat::Topic) -> GlobalResult<Self> {
		let topic_kind = internal_unwrap!(value.kind);

		match topic_kind {
			backend::chat::topic::Kind::Team(team) => Ok(models::ChatSimpleTopic::Group(
				models::ChatSimpleTopicGroup {
					group_id: internal_unwrap!(team.team_id).as_uuid().to_string(),
				},
			)),
			backend::chat::topic::Kind::Party(party) => Ok(models::ChatSimpleTopic::Party(
				models::ChatSimpleTopicParty {
					party_id: internal_unwrap!(party.party_id).as_uuid().to_string(),
				},
			)),
			backend::chat::topic::Kind::Direct(direct) => Ok(models::ChatSimpleTopic::Direct(
				models::ChatSimpleTopicDirect {
					identity_a_id: internal_unwrap!(direct.user_a_id).as_uuid().to_string(),
					identity_b_id: internal_unwrap!(direct.user_b_id).as_uuid().to_string(),
				},
			)),
		}
	}
}

impl ApiFrom<backend::chat::message_body::party_activity_change::State>
	for backend::party::party::State
{
	fn api_from(value: backend::chat::message_body::party_activity_change::State) -> Self {
		match value {
			backend::chat::message_body::party_activity_change::State::MatchmakerFindingLobby(
				inner,
			) => backend::party::party::State::MatchmakerFindingLobby(inner),
			backend::chat::message_body::party_activity_change::State::MatchmakerLobby(inner) => {
				backend::party::party::State::MatchmakerLobby(inner)
			}
		}
	}
}
