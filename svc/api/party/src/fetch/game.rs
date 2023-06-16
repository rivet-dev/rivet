use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;

use crate::auth::Auth;

pub struct NamespaceData {
	pub namespace_id: Uuid,
	pub version_id: Uuid,
}

pub async fn fetch_ns(ctx: &Ctx<Auth>, namespace_id: Uuid) -> GlobalResult<NamespaceData> {
	// Get the namespace data
	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns_data = internal_unwrap_owned!(ns_res.namespaces.first());
	let version_id = internal_unwrap!(ns_data.version_id).as_uuid();

	Ok(NamespaceData {
		namespace_id,
		version_id,
	})
}
