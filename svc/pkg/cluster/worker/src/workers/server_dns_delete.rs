use std::sync::Arc;

use chirp_worker::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework, framework::async_api::ApiClient};
use futures_util::FutureExt;
use proto::backend::pkg::*;

use crate::util::CloudflareError;

#[derive(sqlx::FromRow)]
struct DnsRecordRow {
	dns_record_id: Option<String>,
	secondary_dns_record_id: Option<String>,
}

#[worker(name = "cluster-server-dns-delete")]
async fn worker(
	ctx: &OperationContext<cluster::msg::server_dns_delete::Message>,
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
	ctx: OperationContext<cluster::msg::server_dns_delete::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	client: Arc<cf_framework::async_api::Client>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let row = sql_fetch_optional!(
		[ctx, DnsRecordRow, @tx tx]
		"
		SELECT dns_record_id, secondary_dns_record_id
		FROM db_cluster.servers_cloudflare
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		FOR UPDATE
		",
		&server_id,
		util::timestamp::now(),
	)
	.await?;

	let Some(DnsRecordRow {
		dns_record_id,
		secondary_dns_record_id,
	}) = row
	else {
		// NOTE: It is safe to do nothing in this case because both this worker and
		// `cluster-server-dns-create` use transactions
		tracing::warn!("server has no dns records");
		return Ok(());
	};

	let zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");

	// Delete main record
	if let Some(record_id) = dns_record_id {
		let res = client
			.request(&cf::dns::DeleteDnsRecord {
				zone_identifier: zone_id,
				identifier: &record_id,
			})
			.await;

		if let Err(cf_framework::response::ApiFailure::Error(
			http::status::StatusCode::NOT_FOUND,
			_,
		)) = res
		{
			tracing::warn!(%zone_id, %record_id, "dns record not found");
		} else {
			res?;
			tracing::info!(%record_id, "deleted dns record");
		}
	} else {
		tracing::warn!("server has no primary dns record");
	}

	// Delete secondary record
	if let Some(record_id) = secondary_dns_record_id {
		client
			.request(&cf::dns::DeleteDnsRecord {
				zone_identifier: zone_id,
				identifier: &record_id,
			})
			.await?;

		tracing::info!(%record_id, "deleted secondary dns record");
	} else {
		tracing::warn!("server has no secondary dns record");
	}

	// Update db record
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers_cloudflare
		SET destroy_ts = $2
		WHERE
			server_id = $1 AND
			destroy_ts IS NULL
		",
		server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}
