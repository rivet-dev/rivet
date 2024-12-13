use std::{
	net::{IpAddr, Ipv4Addr},
	ops::Deref,
};

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

	let zone_id = unwrap!(
		ctx.config().server()?.cloudflare()?.zone.job.clone(),
		"dns not configured"
	);
	let domain_job = unwrap!(ctx.config().server()?.rivet.dns()?.domain_job.clone());

	let (primary_dns_record_id, secondary_dns_record_id, actor_wildcard_dns_record_id) = ctx
		.join((
			// Lobbies
			activity(CreateDnsRecordInput {
				record_name: format!("*.lobby.{}.{domain_job}", server_res.datacenter_id),
				public_ip: server_res.public_ip,
				zone_id: zone_id.clone(),
			}),
			// This is solely for compatibility with Discord activities. Their docs say they support parameter
			// mapping but it does not work
			// https://discord.com/developers/docs/activities/development-guides#prefixtarget-formatting-rules
			activity(CreateDnsRecordInput {
				record_name: format!("lobby.{}.{domain_job}", server_res.datacenter_id),
				public_ip: server_res.public_ip,
				zone_id: zone_id.clone(),
			}),
			closure(|ctx| {
				let datacenter_id = server_res.datacenter_id;
				let domain_job = domain_job.clone();
				let public_ip = server_res.public_ip.clone();
				let zone_id = zone_id.clone();

				Box::pin(async move {
					let record_id = match ctx.check_version(2).await? {
						1 => None,
						_latest => Some(
							ctx.activity(CreateDnsRecordInput {
								record_name: format!("*.actor.{datacenter_id}.{domain_job}"),
								public_ip,
								zone_id,
							})
							.await?,
						),
					};

					Ok(record_id)
				})
			}),
		))
		.await?;

	{
		let v1 = InsertDbInputV1 {
			server_id: input.server_id,
			primary_dns_record_id,
			secondary_dns_record_id,
		};

		match ctx.check_version(2).await? {
			1 => ctx.activity(v1).await?,
			_latest => {
				ctx.activity(InsertDbInputV2 {
					v1,
					actor_wildcard_dns_record_id,
				})
				.await?
			}
		}
	}

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
	let cf_token = ctx.config().server()?.cloudflare()?.auth_token.read();
	let client = cf_client(ctx.config(), Some(cf_token)).await?;

	let record_id = create_dns_record(
		&client,
		cf_token,
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

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
struct InsertDbInputV1 {
	server_id: Uuid,
	primary_dns_record_id: String,
	secondary_dns_record_id: String,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertDbInputV2 {
	v1: InsertDbInputV1,
	actor_wildcard_dns_record_id: Option<String>,
}

impl Deref for InsertDbInputV2 {
	type Target = InsertDbInputV1;

	fn deref(&self) -> &Self::Target {
		&self.v1
	}
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInputV1) -> GlobalResult<()> {
	insert_db_inner(
		ctx,
		&InsertDbInputV2 {
			v1: input.clone(),
			actor_wildcard_dns_record_id: None,
		},
	)
	.await
}

#[activity(InsertDbV2)]
async fn insert_db_v2(ctx: &ActivityCtx, input: &InsertDbInputV2) -> GlobalResult<()> {
	insert_db_inner(ctx, input).await
}

async fn insert_db_inner(ctx: &ActivityCtx, input: &InsertDbInputV2) -> GlobalResult<()> {
	// Upsert since this needs to be idempotent for wf upgrades
	//
	// Can't use `USERT` since we use a non-standard PK
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers_cloudflare (
			server_id, dns_record_id, secondary_dns_record_id, actor_wildcard_dns_record_id
		)
		VALUES ($1, $2, $3, $4)
		",
		input.server_id,
		&input.primary_dns_record_id,
		&input.secondary_dns_record_id,
		&input.actor_wildcard_dns_record_id,
	)
	.await?;

	Ok(())
}
