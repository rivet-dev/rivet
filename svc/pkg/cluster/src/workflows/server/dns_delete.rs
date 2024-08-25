use chirp_workflow::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework};

use crate::util::cf_client;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
}

#[workflow]
pub async fn cluster_server_dns_delete(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let records_res = ctx
		.activity(GetDnsRecordsInput {
			server_id: input.server_id,
		})
		.await?;

	let zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");

	if let Some(dns_record_id) = records_res.dns_record_id {
		ctx.activity(DeleteDnsRecordInput {
			dns_record_id,
			zone_id: zone_id.to_string(),
		})
		.await?;
	} else {
		tracing::warn!("server has no primary dns record");
	}

	if let Some(dns_record_id) = records_res.secondary_dns_record_id {
		ctx.activity(DeleteDnsRecordInput {
			dns_record_id,
			zone_id: zone_id.to_string(),
		})
		.await?;
	} else {
		tracing::warn!("server has no secondary dns record");
	}

	ctx.activity(UpdateDbInput {
		server_id: input.server_id,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetDnsRecordsInput {
	server_id: Uuid,
}

#[derive(Debug, Default, sqlx::FromRow, Serialize, Deserialize, Hash)]
struct GetDnsRecordsOutput {
	dns_record_id: Option<String>,
	secondary_dns_record_id: Option<String>,
}

#[activity(GetDnsRecords)]
async fn get_dns_records(
	ctx: &ActivityCtx,
	input: &GetDnsRecordsInput,
) -> GlobalResult<GetDnsRecordsOutput> {
	let row = sql_fetch_optional!(
		[ctx, GetDnsRecordsOutput]
		"
		SELECT dns_record_id, secondary_dns_record_id
		FROM db_cluster.servers_cloudflare
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		",
		&input.server_id,
	)
	.await?;

	if row.is_none() {
		tracing::warn!("server has no DNS record row");
	}

	Ok(row.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteDnsRecordInput {
	dns_record_id: String,
	zone_id: String,
}

#[activity(DeleteDnsRecord)]
async fn delete_dns_record(ctx: &ActivityCtx, input: &DeleteDnsRecordInput) -> GlobalResult<()> {
	let client = cf_client(None).await?;

	let res = client
		.request(&cf::dns::DeleteDnsRecord {
			zone_identifier: &input.zone_id,
			identifier: &input.dns_record_id,
		})
		.await;

	// Gracefully fail if not found
	if let Err(cf_framework::response::ApiFailure::Error(http::status::StatusCode::NOT_FOUND, _)) =
		res
	{
		tracing::warn!(zone_id=%input.zone_id, record_id=%input.dns_record_id, "dns record not found");
	} else {
		res?;
		tracing::info!(record_id=%input.dns_record_id, "deleted dns record");
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers_cloudflare
		SET destroy_ts = $2
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}
