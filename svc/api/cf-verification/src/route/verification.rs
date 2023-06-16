use crate::auth::Auth;
use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_operation::prelude::*;

// MARK: GET /.well-known/cf-custom-hostname-challenge/{}
pub async fn verify_custom_hostname(
	ctx: Ctx<Auth>,
	identifier: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<Vec<u8>> {
	let custom_hostnames_res = op!([ctx] cf_custom_hostname_get {
		identifiers: vec![identifier.into()],
	})
	.await?;
	let custom_hostname =
		internal_unwrap_owned!(custom_hostnames_res.custom_hostnames.first(), "not found");
	let challenge = internal_unwrap!(custom_hostname.challenge);

	Ok(format!("{}\n", challenge).into_bytes())
}
