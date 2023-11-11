use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct CloudflareResponse {
	result: CloudflareResult,
}

#[derive(Debug, Deserialize)]
struct CloudflareResult {
	id: Uuid,
	hostname: String,
	ownership_verification_http: CloudflareOwnershipVerificationHttp,
}

#[derive(Debug, Deserialize)]
struct CloudflareOwnershipVerificationHttp {
	http_body: Uuid,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
	errors: Vec<CloudflareErrorEntry>,
}

#[derive(Debug, Deserialize)]
struct CloudflareErrorEntry {
	code: usize,
	// message: String,
}

/// Send a lobby create fail message and cleanup the lobby if needed.
#[tracing::instrument]
async fn fail(
	client: &chirp_client::Client,
	namespace_id: Uuid,
	hostname: String,
	error_code: cf_custom_hostname::msg::create_fail::ErrorCode,
) -> GlobalResult<()> {
	msg!([client] cf_custom_hostname::msg::create_fail(namespace_id, &hostname) {
		namespace_id: Some(namespace_id.into()),
		hostname: hostname,
		error_code: error_code as i32,
	})
	.await?;

	Ok(())
}

#[worker(name = "cf-custom-hostname-create")]
async fn worker(
	ctx: &OperationContext<cf_custom_hostname::msg::create::Message>,
) -> GlobalResult<()> {
	let game_zone_id = unwrap!(util::env::cloudflare::zone::game::id());

	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	let games_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let game = unwrap!(games_res.games.first());
	let game_id = unwrap!(game.game_id);

	let games_res = op!([ctx] game_get {
		game_ids: vec![game_id],
	})
	.await?;
	let game = unwrap!(games_res.games.first());
	let developer_team_id = unwrap!(game.developer_team_id);

	// Count current pending hostnames in team
	if !ctx.bypass_pending_cap {
		let teams_res = op!([ctx] team_dev_game_list {
			team_ids: vec![developer_team_id],
		})
		.await?;
		let team = unwrap!(teams_res.teams.first());

		let namespaces_res = op!([ctx] game_namespace_list {
			game_ids: team.game_ids.clone(),
		})
		.await?;

		let namespaces_res = op!([ctx] cf_custom_hostname_list_for_namespace_id {
			namespace_ids: namespaces_res.games
				.iter()
				.flat_map(|game| &game.namespace_ids)
				.cloned()
				.collect::<Vec<_>>(),
			pending_only: true,
		})
		.await?;

		let identifier_count = namespaces_res
			.namespaces
			.iter()
			.fold(0, |acc, namespace| acc + namespace.identifiers.len());

		if identifier_count >= 10 {
			return fail(
				ctx.chirp(),
				namespace_id,
				ctx.hostname.clone(),
				cf_custom_hostname::msg::create_fail::ErrorCode::TooManyPendingHostnames,
			)
			.await;
		}
	}

	let payload = json!({
		"hostname": ctx.hostname,
		"ssl": {
			"method": "http",
			"type": "dv",
			"settings": {
				"http2": "on",
				"min_tls_version": "1.2",
				"tls_1_3": "on",
				"ciphers": [
					"ECDHE-RSA-AES128-GCM-SHA256",
					"AES128-SHA"
				],
				"early_hints": "on"
			},
			"bundle_method": "ubiquitous",
			"wildcard": false
		},
	});
	let res = reqwest::Client::new()
		.post(format!(
			"https://api.cloudflare.com/client/v4/zones/{game_zone_id}/custom_hostnames",
		))
		.header(
			reqwest::header::AUTHORIZATION,
			format!("Bearer {}", util::env::cloudflare::auth_token()),
		)
		.json(&payload)
		.send()
		.await?;

	if !res.status().is_success() {
		let status = res.status();
		let text = res.text().await;

		// Gracefully handle error if possible
		if let Ok(text) = &text {
			match serde_json::from_str::<CloudflareError>(text) {
				Ok(err_body) => {
					if err_body.errors.iter().any(|x| x.code == 1406) {
						tracing::warn!(hostname=?ctx.hostname, "hostname already exists");

						return fail(
							ctx.chirp(),
							namespace_id,
							ctx.hostname.clone(),
							cf_custom_hostname::msg::create_fail::ErrorCode::AlreadyExists,
						)
						.await;
					}
				}
				Err(err) => {
					tracing::warn!(?err, %text, "failed to decode error");
				}
			}
		}

		tracing::error!(hostname=?ctx.hostname, ?status, "failed to create custom hostname");
		bail!("failed to create custom hostname");
	}

	let res = res.json::<CloudflareResponse>().await?;
	let identifier = res.result.id;
	let hostname = res.result.hostname;
	let challenge = res.result.ownership_verification_http.http_body;
	let subscription_id = Uuid::new_v4();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cf_custom_hostname.custom_hostnames (
			identifier, namespace_id, hostname, challenge, create_ts, status, subscription_id
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		",
		identifier,
		namespace_id,
		&hostname,
		challenge,
		ctx.ts(),
		backend::cf::custom_hostname::Status::Pending as i32,
		subscription_id,
	)
	.await?;

	// TODO: Add stripe subscription for hostname

	msg!([ctx] cf_custom_hostname::msg::create_complete(namespace_id, &ctx.hostname) {
		namespace_id: ctx.namespace_id,
		hostname: ctx.hostname.clone(),
		identifier: Some(identifier.into()),
	})
	.await?;

	Ok(())
}
