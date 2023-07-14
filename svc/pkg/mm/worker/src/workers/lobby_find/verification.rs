use chirp_worker::prelude::*;
use http::StatusCode;
use proto::backend::{
	self,
	pkg::{
		mm::{msg::lobby_find::message::Query, msg::lobby_find_fail::ErrorCode},
		*,
	},
};
use serde_json::json;

use super::{fail, LobbyGroupConfig};

pub async fn verify(
	ctx: &OperationContext<mm::msg::lobby_find::Message>,
	namespace_id: Uuid,
	query_id: Uuid,
	query: &Query,
	lobby_group_config: &LobbyGroupConfig,
) -> GlobalResult<bool> {
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
				}
			}),
		),
		_ => (backend::matchmaker::IdentityRequirement::None, None),
	};

	// Verify identity registration
	match (identity_requirement, ctx.user_id) {
		(backend::matchmaker::IdentityRequirement::Registered, Some(user_id)) => {
			let user_identities_res = op!([ctx] user_identity_get {
				user_ids: vec![user_id],
			})
			.await?;
			let user = internal_unwrap_owned!(
				user_identities_res.users.first(),
				"could not find user identities"
			);
			let is_registered = !user.identities.is_empty();

			if !is_registered {
				fail(
					ctx,
					namespace_id,
					query_id,
					ErrorCode::RegistrationRequired,
					true,
				)
				.await?;
			}
		}
		(
			backend::matchmaker::IdentityRequirement::Guest
			| backend::matchmaker::IdentityRequirement::Registered,
			None,
		) => {
			fail(
				ctx,
				namespace_id,
				query_id,
				ErrorCode::IdentityRequired,
				true,
			)
			.await?;
		}
		_ => {}
	}

	// Verify user data externally
	if external_request_config.is_some() {
		let request_id = Uuid::new_v4();

		// Build body
		let lobby_state = if let Some(l) = &lobby_group_config.lobby_state {
			// Fetch region data for readable name
			let region_id = internal_unwrap_owned!(l.region_id);
			let regions_res = op!([ctx] region_get {
				region_ids: vec![region_id],
			})
			.await?;
			let region = internal_unwrap_owned!(regions_res.regions.first());

			json!({
				"lobby_id": internal_unwrap!(l.lobby_id).as_uuid(),
				"lobby_group_id": internal_unwrap!(l.lobby_group_id).as_uuid(),
				"lobby_group_name_id": lobby_group_config.lobby_group.name_id,
				"region_id": region_id.as_uuid(),
				"region_name_id": region.name_id,
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

		// Send request
		let external_res = msg!([ctx] external::msg::request_call(request_id)
		-> Result<external::msg::request_call_complete, external::msg::request_call_fail>
		{
			request_id: Some(request_id.into()),
			config: external_request_config,
			timeout: util::duration::seconds(10) as u64,
			body: Some(serde_json::to_vec(&body)?),
			..Default::default()
		})
		.await?;

		// Handle status code
		let success = if let Ok(res) = external_res {
			let status = StatusCode::from_u16(res.status_code as u16)?;

			tracing::info!(?status, "user verification response");

			if status.is_success() {
				true
			} else {
				fail(
					ctx,
					namespace_id,
					query_id,
					ErrorCode::VerificationFailed,
					true,
				)
				.await?;

				false
			}
		} else {
			fail(
				ctx,
				namespace_id,
				query_id,
				ErrorCode::VerificationRequestFailed,
				true,
			)
			.await?;

			false
		};

		return Ok(success);
	}

	Ok(true)
}
