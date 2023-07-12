use chirp_worker::prelude::*;
use http::StatusCode;
use proto::backend::{
	self,
	pkg::{mm::msg::lobby_find::message::Query, *},
};
use serde_json::json;

use super::LobbyGroupConfig;

pub async fn verify(
	ctx: &OperationContext<mm::msg::lobby_find::Message>,
	query: &Query,
	lobby_group_config: &LobbyGroupConfig,
	lobby_state: Option<backend::matchmaker::Lobby>,
) -> GlobalResult<()> {
	// Get required verifications
	let (identity_requirement, external_request_config) = match (
		query,
		lobby_group_config.lobby_group.find_config.as_ref(),
		lobby_group_config.lobby_group.join_config.as_ref(),
	) {
		(Query::LobbyGroup(_), Some(find_config), _) => (
			internal_unwrap_owned!(
				backend::matchmaker::IdentityRequirement::from_i32(
					find_config.identity_requirement
				),
				"invalid identity requirement variant"
			),
			find_config.verification_config.as_ref().map(|config| {
				backend::net::ExternalRequestConfig {
					url: config.url.clone(),
					method: backend::net::HttpMethod::Post as i32,
					headers: config.headers.clone(),
					timeout: util::duration::seconds(10) as u64,
				}
			}),
		),
		(Query::Direct(_), _, Some(join_config)) => (
			internal_unwrap_owned!(
				backend::matchmaker::IdentityRequirement::from_i32(
					join_config.identity_requirement
				),
				"invalid identity requirement variant"
			),
			join_config.verification_config.as_ref().map(|config| {
				backend::net::ExternalRequestConfig {
					url: config.url.clone(),
					method: backend::net::HttpMethod::Post as i32,
					headers: config.headers.clone(),
					timeout: util::duration::seconds(10) as u64,
				}
			}),
		),
		_ => (backend::matchmaker::IdentityRequirement::None, None),
	};

	// TODO: Verify identity registration

	// Verify user data
	if external_request_config.is_some() {
		let request_id = Uuid::new_v4();

		// Build body
		let lobby_state = if let Some(l) = lobby_state {
			// TODO: Add readable region name and lobby group name
			json!({
				"lobby_id": internal_unwrap!(l.lobby_id).as_uuid(),
				"lobby_group_id": internal_unwrap!(l.lobby_group_id).as_uuid(),
				"region_id": internal_unwrap!(l.region_id).as_uuid(),
				"create_ts": util::timestamp::to_string(l.create_ts)?,
				"is_closed": l.is_closed,
				"namespace_id": internal_unwrap!(l.namespace_id).as_uuid(),
			})
		} else {
			serde_json::Value::Null
		};
		let body = json!({
			"verification_data": ctx
				.verification_data_json
				.as_ref()
				.map(serde_json::to_value)
				.transpose()?,
			"lobby": lobby_state,
			"type": match query {
				Query::LobbyGroup(_) => "find",
				Query::Direct(_) => "join",
			},
		});

		let external_res = msg!([ctx] external::msg::request_call(request_id) -> external::msg::request_call_complete {
			request_id: Some(request_id.into()),
			config: external_request_config,
			body: Some(serde_json::to_vec(&body)?),
		}).await?;
		let status = StatusCode::from_u16(external_res.status_code as u16)?;

		tracing::info!(?status, "user verification response");

		if !status.is_success() {
			// TODO:
		}
	}

	Ok(())
}
