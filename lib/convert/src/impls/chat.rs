use proto::backend;
use rivet_chat_server::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiTryFrom};

impl ApiTryFrom<backend::chat::TypingStatus> for models::ChatTypingStatus {
	type Error = GlobalError;

	fn try_from(value: backend::chat::TypingStatus) -> GlobalResult<Self> {
		let kind = unwrap_ref!(value.kind);

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
		let topic_kind = unwrap_ref!(value.kind);

		match topic_kind {
			backend::chat::topic::Kind::Team(team) => Ok(models::ChatSimpleTopic::Group(
				models::ChatSimpleTopicGroup {
					group_id: unwrap_ref!(team.team_id).as_uuid().to_string(),
				},
			)),
			backend::chat::topic::Kind::Direct(direct) => Ok(models::ChatSimpleTopic::Direct(
				models::ChatSimpleTopicDirect {
					identity_a_id: unwrap_ref!(direct.user_a_id).as_uuid().to_string(),
					identity_b_id: unwrap_ref!(direct.user_b_id).as_uuid().to_string(),
				},
			)),
		}
	}
}
