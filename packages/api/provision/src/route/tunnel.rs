use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /tunnel/tls
pub async fn tls(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ProvisionTunnelGetTlsResponse> {
	ctx.auth().server()?;

	let tunnel_tls_res = ctx.op(cluster::ops::tunnel::tls_get::Input {}).await?;

	Ok(models::ProvisionTunnelGetTlsResponse {
		cert_pem: tunnel_tls_res.cert_pem,
		root_ca_cert_pem: tunnel_tls_res.root_ca_cert_pem,
		private_key_pem: tunnel_tls_res.private_key_pem,
	})
}
