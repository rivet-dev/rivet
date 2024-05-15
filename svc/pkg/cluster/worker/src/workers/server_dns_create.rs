use std::{net::IpAddr, sync::Arc};

use chirp_worker::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework, framework::async_api::ApiClient};
use futures_util::FutureExt;
use proto::backend::pkg::*;

use crate::util::CloudflareError;

#[derive(sqlx::FromRow)]
struct Server {
	datacenter_id: Uuid,
	public_ip: IpAddr,
	cloud_destroy_ts: Option<i64>,
}

#[worker(name = "cluster-server-dns-create")]
async fn worker(
	ctx: &OperationContext<cluster::msg::server_dns_create::Message>,
) -> GlobalResult<()> {
	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	// Create cloudflare HTTP client
	let client = Arc::new(
		cf_framework::async_api::Client::new(
			cf_framework::auth::Credentials::UserAuthToken { token: cf_token },
			Default::default(),
			cf_framework::Environment::Production,
		)
		.map_err(CloudflareError::from)?,
	);

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		inner(ctx.clone(), tx, client.clone()).boxed()
	})
	.await?;

	Ok(())
}

async fn inner(
	ctx: OperationContext<cluster::msg::server_dns_create::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	client: Arc<cf_framework::async_api::Client>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let server = sql_fetch_one!(
		[ctx, Server, @tx tx]
		"
		SELECT
			datacenter_id, public_ip, cloud_destroy_ts
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		server_id,
	)
	.await?;

	// Lock row
	sql_execute!(
		[ctx, @tx tx]
		"
		SELECT 1 FROM db_cluster.servers_cloudflare
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		FOR UPDATE
		",
		server_id,
	)
	.await?;

	if server.cloud_destroy_ts.is_some() {
		tracing::info!("server marked for deletion, not creating dns record");
		return Ok(());
	}

	let zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
	let public_ip = match server.public_ip {
		IpAddr::V4(ip) => ip,
		IpAddr::V6(_) => bail!("unexpected ipv6 public ip"),
	};

	let record_name = format!(
		"*.lobby.{}.{}",
		server.datacenter_id,
		unwrap!(util::env::domain_job()),
	);
	let create_record_res = client
		.request(&cf::dns::CreateDnsRecord {
			zone_identifier: zone_id,
			params: cf::dns::CreateDnsRecordParams {
				name: &record_name,
				content: cf::dns::DnsContent::A { content: public_ip },
				proxied: Some(false),
				ttl: Some(60),
				priority: None,
			},
		})
		.await?;

	tracing::info!(record_id=%create_record_res.result.id, "created dns record");

	// Save record id for deletion
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers_cloudflare
		SET dns_record_id = $2
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		",
		server_id,
		create_record_res.result.id,
	)
	.await?;

	// This is solely for compatibility with Discord activities. Their docs say they support parameter
	// mapping but it does not work
	// https://discord.com/developers/docs/activities/development-guides#prefixtarget-formatting-rules
	let secondary_record_name = format!(
		"lobby.{}.{}",
		server.datacenter_id,
		unwrap!(util::env::domain_job()),
	);
	let create_secondary_record_res = client
		.request(&cf::dns::CreateDnsRecord {
			zone_identifier: zone_id,
			params: cf::dns::CreateDnsRecordParams {
				name: &secondary_record_name,
				content: cf::dns::DnsContent::A { content: public_ip },
				proxied: Some(false),
				ttl: Some(60),
				priority: None,
			},
		})
		.await?;

	tracing::info!(record_id=%create_secondary_record_res.result.id, "created secondary dns record");

	// Save record id for deletion
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers_cloudflare
		SET secondary_dns_record_id = $2
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		",
		server_id,
		create_secondary_record_res.result.id,
	)
	.await?;

	Ok(())
}
