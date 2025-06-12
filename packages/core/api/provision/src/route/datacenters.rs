use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::auth::Auth;

// MARK: GET /datacenters/{}/tls
pub async fn tls(
	ctx: Ctx<Auth>,
	datacenter_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ProvisionDatacentersGetTlsResponse> {
	ctx.auth().server()?;

	let datacenter_res = ctx
		.op(cluster::ops::datacenter::tls_get::Input {
			datacenter_ids: vec![datacenter_id],
		})
		.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());

	let (
		Some(job_cert_pem),
		Some(job_private_key_pem),
		Some(api_cert_pem),
		Some(api_private_key_pem),
	) = (
		&datacenter.job_cert_pem,
		&datacenter.job_private_key_pem,
		&datacenter.api_cert_pem,
		&datacenter.api_private_key_pem,
	)
	else {
		bail_with!(API_NOT_FOUND);
	};

	Ok(models::ProvisionDatacentersGetTlsResponse {
		job_cert_pem: job_cert_pem.clone(),
		job_private_key_pem: job_private_key_pem.clone(),
		api_cert_pem: api_cert_pem.clone(),
		api_private_key_pem: api_private_key_pem.clone(),
	})
}

#[derive(Deserialize)]
pub struct ServerFilterQuery {
	pools: Vec<models::ProvisionPoolType>,
}

// MARK: GET /datacenters/{}/servers
pub async fn servers(
	ctx: Ctx<Auth>,
	datacenter_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ServerFilterQuery,
) -> GlobalResult<models::ProvisionDatacentersGetServersResponse> {
	// Find server based on public ip
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: cluster::types::Filter {
				datacenter_ids: Some(vec![datacenter_id]),
				pool_types: (!query.pools.is_empty())
					.then(|| query.pools.into_iter().map(ApiInto::api_into).collect()),
				..Default::default()
			},
			include_destroyed: false,
			exclude_draining: true,
			exclude_no_vlan: true,
		})
		.await?;

	Ok(models::ProvisionDatacentersGetServersResponse {
		servers: servers_res
			.servers
			.into_iter()
			// Filter out installing servers
			.filter(|server| server.install_complete_ts.is_some())
			.map(ApiInto::api_into)
			.collect(),
	})
}
