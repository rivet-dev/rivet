use std::net::Ipv4Addr;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /datacenters/{}/tls
pub async fn tls(
	ctx: Ctx<Auth>,
	datacenter_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ProvisionDatacentersGetTlsResponse> {
	ctx.auth().server()?;

	let datacenter_res = op!([ctx] cluster_datacenter_tls_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());

	let (Some(job_cert_pem), Some(job_private_key_pem)) =
		(&datacenter.job_cert_pem, &datacenter.job_private_key_pem)
	else {
		bail_with!(API_NOT_FOUND);
	};

	Ok(models::ProvisionDatacentersGetTlsResponse {
		job_cert_pem: job_cert_pem.clone(),
		job_private_key_pem: job_private_key_pem.clone(),
	})
}
