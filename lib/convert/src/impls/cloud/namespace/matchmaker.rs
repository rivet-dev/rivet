use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiTryFrom;

impl ApiTryFrom<backend::matchmaker::NamespaceConfig> for models::CloudMatchmakerNamespaceConfig {
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::NamespaceConfig) -> GlobalResult<Self> {
		Ok(models::CloudMatchmakerNamespaceConfig {
			lobby_count_max: value.lobby_count_max as i32,
			max_players_per_client: value.max_players_per_client as i32,
			max_players_per_client_vpn: value.max_players_per_client_vpn as i32,
			max_players_per_client_proxy: value.max_players_per_client_proxy as i32,
			max_players_per_client_tor: value.max_players_per_client_tor as i32,
			max_players_per_client_hosting: value.max_players_per_client_hosting as i32,
		})
	}
}
