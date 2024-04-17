use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct DatacenterTls {
	datacenter_id: Uuid,
	gg_cert_pem: Option<String>,
	gg_private_key_pem: Option<String>,
	job_cert_pem: Option<String>,
	job_private_key_pem: Option<String>,
	state: i64,
	expire_ts: i64,
}

impl From<DatacenterTls> for cluster::datacenter_tls_get::response::Datacenter {
	fn from(value: DatacenterTls) -> Self {
		cluster::datacenter_tls_get::response::Datacenter {
			datacenter_id: Some(value.datacenter_id.into()),
			gg_cert_pem: value.gg_cert_pem,
			gg_private_key_pem: value.gg_private_key_pem,
			job_cert_pem: value.job_cert_pem,
			job_private_key_pem: value.job_private_key_pem,
			state: value.state as i32,
			expire_ts: value.expire_ts,
		}
	}
}

#[operation(name = "cluster-datacenter-tls-get")]
pub async fn handle(
	ctx: OperationContext<cluster::datacenter_tls_get::Request>,
) -> GlobalResult<cluster::datacenter_tls_get::Response> {
	let datacenter_ids = ctx
		.datacenter_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let rows = sql_fetch_all!(
		[ctx, DatacenterTls]
		"
		SELECT
			datacenter_id,
			gg_cert_pem,
			gg_private_key_pem,
			job_cert_pem,
			job_private_key_pem,
			state,
			expire_ts
		FROM db_cluster.datacenter_tls
		WHERE datacenter_id = ANY($1)
		",
		datacenter_ids,
	)
	.await?;

	Ok(cluster::datacenter_tls_get::Response {
		datacenters: rows.into_iter().map(Into::into).collect::<Vec<_>>(),
	})
}
