use std::net::IpAddr;

use chirp_worker::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework, framework::async_api::ApiClient};
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
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let server = sql_fetch_one!(
		[ctx, Server]
		"
		SELECT
			datacenter_id, public_ip, cloud_destroy_ts
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		server_id,
	)
	.await?;

	if server.cloud_destroy_ts.is_some() {
		tracing::info!("server marked for deletion, not creating dns record");
		return Ok(());
	}

	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	let zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
	let public_ip = match server.public_ip {
		IpAddr::V4(ip) => ip,
		IpAddr::V6(_) => bail!("unexpected ipv6 public ip"),
	};

	// Create cloudflare HTTP client
	let client = cf_framework::async_api::Client::new(
		cf_framework::auth::Credentials::UserAuthToken { token: cf_token },
		Default::default(),
		cf_framework::Environment::Production,
	)
	.map_err(CloudflareError::from)?;

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
	let record_id = create_record_res.result.id;

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
		.await;

	// Optionally get secondary record id
	let secondary_dns_record_id = create_secondary_record_res
		.as_ref()
		.ok()
		.map(|res| res.result.id.clone());

	// Save record ids for deletion
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers_cloudflare (
			server_id,
			dns_record_id,
			secondary_dns_record_id
		)
		VALUES ($1, $2, $3)
		",
		server_id,
		record_id,
		secondary_dns_record_id,
	)
	.await?;

	create_secondary_record_res?;

	Ok(())
}
