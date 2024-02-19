use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CloudflareError {
	errors: Vec<CloudflareErrorEntry>,
}

#[derive(Debug, Deserialize)]
struct CloudflareErrorEntry {
	code: usize,
	// message: String,
}

#[worker(name = "cf-custom-hostname-delete")]
async fn worker(
	ctx: &OperationContext<cf_custom_hostname::msg::delete::Message>,
) -> GlobalResult<()> {
	let game_zone_id = unwrap!(util::env::cloudflare::zone::game::id());

	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	let custom_hostnames_res = op!([ctx] cf_custom_hostname_resolve_hostname {
		hostnames: vec![ctx.hostname.clone()],
	})
	.await?;
	let custom_hostname =
		if let Some(custom_hostname) = custom_hostnames_res.custom_hostnames.first() {
			custom_hostname
		} else {
			tracing::info!(%namespace_id, hostname=%ctx.hostname, "custom hostname does not exist");

			msg!([ctx] cf_custom_hostname::msg::delete_complete(namespace_id, &ctx.hostname) {
				namespace_id: ctx.namespace_id,
				hostname: ctx.hostname.clone(),
			})
			.await?;

			return Ok(());
		};
	let identifier = unwrap!(custom_hostname.identifier).as_uuid();

	let res = reqwest::Client::new()
		.delete(format!(
			"https://api.cloudflare.com/client/v4/zones/{game_zone_id}/custom_hostnames/{identifier}",
			identifier = identifier,
		))
		.header(
			reqwest::header::AUTHORIZATION,
			format!("Bearer {}", util::env::cloudflare::auth_token()),
		)
		.send()
		.await?;

	if !res.status().is_success() {
		let status = res.status();
		let text = res.text().await;

		// Gracefully handle error if possible
		if let Ok(text) = &text {
			match serde_json::from_str::<CloudflareError>(text) {
				Ok(err_body) => {
					if err_body.errors.iter().any(|x| x.code == 1436) {
						tracing::warn!(hostname=?ctx.hostname, "hostname does not exist");
					}
				}
				Err(err) => {
					tracing::warn!(?err, %text, "failed to decode error");
					bail!("failed to delete custom hostname");
				}
			}
		} else {
			tracing::error!(hostname=?ctx.hostname, ?status, "failed to delete custom hostname");
			bail!("failed to delete custom hostname");
		}
	}

	let (_subscription_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"
		SELECT subscription_id
		FROM db_cf_custom_hostname.custom_hostnames
		WHERE identifier = $1
		",
		identifier,
	)
	.await?;

	// TODO: Delete stripe subscription for hostname

	sql_execute!(
		[ctx]
		"
		DELETE FROM db_cf_custom_hostname.custom_hostnames
		WHERE identifier = $1
		",
		identifier,
	)
	.await?;

	msg!([ctx] cf_custom_hostname::msg::delete_complete(namespace_id, &ctx.hostname) {
		namespace_id: ctx.namespace_id,
		hostname: ctx.hostname.clone(),
	})
	.await?;

	Ok(())
}
