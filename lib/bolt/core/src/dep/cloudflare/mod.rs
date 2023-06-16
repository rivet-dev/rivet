pub mod tunnel;

pub use tunnel::{Tunnel, TunnelConfig, TunnelProtocol};

use anyhow::Result;

use crate::context::ProjectContext;

#[derive(Clone, serde::Deserialize)]
pub struct AccessSecret {
	pub client_id: String,
	pub client_secret: String,
}

pub async fn fetch_access_secret(ctx: &ProjectContext, path: &[&str]) -> Result<AccessSecret> {
	Ok(AccessSecret {
		client_id: ctx.read_secret(&[path, &["client_id"]].concat()).await?,
		client_secret: ctx
			.read_secret(&[path, &["client_secret"]].concat())
			.await?,
	})
}
