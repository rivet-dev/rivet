use chirp_workflow::prelude::*;

use crate::types::TlsState;

#[derive(Debug)]
pub struct Input {}

#[derive(Debug, sqlx::FromRow)]
pub struct Output {
	pub cert_pem: String,
	pub private_key_pem: String,
	pub root_ca_cert_pem: String,
}

#[operation]
pub async fn cluster_datacenter_tls_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let row = sql_fetch_optional!(
		[ctx, Output]
		"
		SELECT cert_pem, private_key_pem, root_ca_cert_pem
		FROM db_cluster.tunnel_tls
		WHERE state != $1
		",
		TlsState::Creating as i64,
	)
	.await?;

	Ok(unwrap!(row, "tunnel tls not created yet"))
}
