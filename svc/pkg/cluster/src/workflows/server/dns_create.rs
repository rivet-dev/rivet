use std::net::{IpAddr, Ipv4Addr};

use chirp_workflow::prelude::*;
use cloudflare::endpoints as cf;

use crate::util::{cf_client, create_dns_record};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
}

#[workflow]
pub async fn cluster_server_dns_create(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let server_res = ctx
		.activity(GetServerInfoInput {
			server_id: input.server_id,
		})
		.await?;

	let zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
	let domain_job = unwrap!(util::env::domain_job());

	let (primary_dns_record_id, secondary_dns_record_id) = ctx
		.join((
			activity(CreateDnsRecordInput {
				record_name: format!("*.lobby.{}.{domain_job}", server_res.datacenter_id),
				public_ip: server_res.public_ip,
				zone_id: zone_id.to_string(),
			}),
			// This is solely for compatibility with Discord activities. Their docs say they support parameter
			// mapping but it does not work
			// https://discord.com/developers/docs/activities/development-guides#prefixtarget-formatting-rules
			activity(CreateDnsRecordInput {
				record_name: format!("lobby.{}.{domain_job}", server_res.datacenter_id),
				public_ip: server_res.public_ip,
				zone_id: zone_id.to_string(),
			}),
		))
		.await?;

	ctx.activity(InsertDbInput {
		server_id: input.server_id,
		primary_dns_record_id,
		secondary_dns_record_id,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetServerInfoInput {
	server_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetServerInfoOutput {
	datacenter_id: Uuid,
	public_ip: Ipv4Addr,
}

#[activity(GetServerInfo)]
async fn get_server_info(
	ctx: &ActivityCtx,
	input: &GetServerInfoInput,
) -> GlobalResult<GetServerInfoOutput> {
	let (datacenter_id, public_ip) = sql_fetch_one!(
		[ctx, (Uuid, IpAddr)]
		"
		SELECT datacenter_id, public_ip
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	let public_ip = match public_ip {
		IpAddr::V4(ip) => ip,
		IpAddr::V6(_) => bail!("unexpected ipv6 public ip"),
	};

	Ok(GetServerInfoOutput {
		datacenter_id,
		public_ip,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateDnsRecordInput {
	record_name: String,
	public_ip: Ipv4Addr,
	zone_id: String,
}

#[activity(CreateDnsRecord)]
async fn create_dns_record(
	ctx: &ActivityCtx,
	input: &CreateDnsRecordInput,
) -> GlobalResult<String> {
	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	let client = cf_client(Some(&cf_token)).await?;

	let record_id = create_dns_record(
		&client,
		&cf_token,
		&input.zone_id,
		&input.record_name,
		cf::dns::DnsContent::A {
			content: input.public_ip,
		},
	)
	.await?;

	tracing::info!(%record_id, "created dns record");

	Ok(record_id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	server_id: Uuid,
	primary_dns_record_id: String,
	secondary_dns_record_id: String,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers_cloudflare (
			server_id, dns_record_id, secondary_dns_record_id
		)
		VALUES ($1, $2, $3)
		",
		input.server_id,
		&input.primary_dns_record_id,
		&input.secondary_dns_record_id,
	)
	.await?;

	Ok(())
}
