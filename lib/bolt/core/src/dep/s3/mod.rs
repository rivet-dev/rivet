use anyhow::Result;

use crate::context::ProjectContext;

#[derive(Clone, serde::Deserialize)]
pub struct ServiceKey {
	pub key_id: String,
	pub key: String,
}

pub async fn fetch_service_key(ctx: &ProjectContext, path: &[&str]) -> Result<ServiceKey> {
	Ok(ServiceKey {
		key_id: ctx.read_secret(&[path, &["key_id"]].concat()).await?,
		key: ctx.read_secret(&[path, &["key"]].concat()).await?,
	})
}
