use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

use crate::ApiFrom;

impl ApiFrom<user_presence::msg::update::message::Kind>
	for backend::user::event::presence_update::Kind
{
	fn api_from(kind: user_presence::msg::update::message::Kind) -> Self {
		match kind {
			user_presence::msg::update::message::Kind::Status(status) => {
				backend::user::event::presence_update::Kind::Status(status)
			}
			user_presence::msg::update::message::Kind::GameActivity(game_activity) => {
				backend::user::event::presence_update::Kind::GameActivity(game_activity)
			}
		}
	}
}

impl ApiFrom<user_presence::msg::update::message::Kind>
	for backend::user::update::presence_update::Kind
{
	fn api_from(kind: user_presence::msg::update::message::Kind) -> Self {
		match kind {
			user_presence::msg::update::message::Kind::Status(status) => {
				backend::user::update::presence_update::Kind::Status(status)
			}
			user_presence::msg::update::message::Kind::GameActivity(game_activity) => {
				backend::user::update::presence_update::Kind::GameActivity(game_activity)
			}
		}
	}
}
