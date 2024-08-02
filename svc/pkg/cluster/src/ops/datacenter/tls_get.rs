use std::convert::{TryFrom, TryInto};

use chirp_workflow::prelude::*;

use crate::types::TlsState;

#[derive(sqlx::FromRow)]
struct DatacenterTlsRow {
	datacenter_id: Uuid,
	gg_cert_pem: Option<String>,
	gg_private_key_pem: Option<String>,
	job_cert_pem: Option<String>,
	job_private_key_pem: Option<String>,
	state: i64,
	state2: Option<sqlx::types::Json<TlsState>>,
	expire_ts: i64,
}

impl TryFrom<DatacenterTlsRow> for DatacenterTls {
	type Error = GlobalError;

	fn try_from(value: DatacenterTlsRow) -> GlobalResult<Self> {
		Ok(DatacenterTls {
			datacenter_id: value.datacenter_id,
			gg_cert_pem: value.gg_cert_pem,
			gg_private_key_pem: value.gg_private_key_pem,
			job_cert_pem: value.job_cert_pem,
			job_private_key_pem: value.job_private_key_pem,
			// Handle backwards compatibility
			state: if let Some(state) = value.state2 {
				state.0
			} else {
				value.state.try_into()?
			},
			expire_ts: value.expire_ts,
		})
	}
}

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<DatacenterTls>,
}

#[derive(Debug)]
pub struct DatacenterTls {
	pub datacenter_id: Uuid,
	pub gg_cert_pem: Option<String>,
	pub gg_private_key_pem: Option<String>,
	pub job_cert_pem: Option<String>,
	pub job_private_key_pem: Option<String>,
	pub state: TlsState,
	pub expire_ts: i64,
}

#[operation]
pub async fn cluster_datacenter_tls_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let rows = sql_fetch_all!(
		[ctx, DatacenterTlsRow]
		"
		SELECT
			datacenter_id,
			gg_cert_pem,
			gg_private_key_pem,
			job_cert_pem,
			job_private_key_pem,
			state,
			state2,
			expire_ts
		FROM db_cluster.datacenter_tls
		WHERE datacenter_id = ANY($1)
		",
		&input.datacenter_ids,
	)
	.await?;

	Ok(Output {
		datacenters: rows
			.into_iter()
			.map(TryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
