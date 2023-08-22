use std::{fmt::Display, sync::Arc};

use anyhow::*;

use crate::{
	config,
	context::ProjectContext,
	dep::cloudflare,
	utils::{self, DroppablePort},
};

pub mod api;
pub mod cli;
pub mod gen;
pub mod job_schema;

#[derive(Clone)]
pub struct NomadCtx {
	project_ctx: ProjectContext,
	client: reqwest::Client,

	/// Nomad & Consul datacenter to deploy to.
	pub region: String,

	/// Secret used to connect to the Cloudflare tunnel.
	access_secret: Option<cloudflare::AccessSecret>,

	/// Port forward to Nomad.
	_handle: Arc<DroppablePort>,
}

impl NomadCtx {
	pub async fn remote(ctx: &ProjectContext) -> Result<Self> {
		let (region_id, access_secret) = match ctx.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => ("local".to_string(), None),
			config::ns::ClusterKind::Distributed { .. } => {
				assert!(
					matches!(
						ctx.ns().dns.provider,
						config::ns::DnsProvider::Cloudflare {
							access: Some(_),
							..
						}
					),
					"cloudflare access not enabled"
				);

				let access_secret = cloudflare::fetch_access_secret(
					ctx,
					&["cloudflare", "access", "terraform_nomad"],
				)
				.await
				.unwrap();
				(ctx.primary_region_or_local(), Some(access_secret))
			}
		};

		let handle = utils::kubectl_port_forward("nomad-server", "nomad", (4646, 4646))?;

		// Wait for port forward to open and check if successful
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
		handle.check()?;

		Ok(NomadCtx {
			project_ctx: ctx.clone(),
			client: reqwest::Client::new(),
			region: region_id.clone(),
			access_secret,
			_handle: Arc::new(handle),
		})
	}

	pub fn build_nomad_request(
		&self,
		method: reqwest::Method,
		path: impl Display,
	) -> reqwest::RequestBuilder {
		match self.project_ctx.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => self
				.client
				.request(method, format!("http://127.0.0.1:4646{path}")),

			config::ns::ClusterKind::Distributed { .. } => {
				let access_secret = self.access_secret.as_ref().unwrap();
				self.client
					.request(
						method,
						format!("https://nomad.{}{path}", self.project_ctx.domain_main()),
					)
					.header("CF-Access-Client-Id", access_secret.client_id.as_str())
					.header(
						"CF-Access-Client-Secret",
						access_secret.client_secret.as_str(),
					)
			}
		}
	}
}
