use std::fmt::Display;

use crate::{config, context::ProjectContext, dep::cloudflare};

pub mod api;
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
}

impl NomadCtx {
	pub async fn remote(ctx: &ProjectContext) -> Self {
		let (region_id, access_secret) = match ctx.ns().deploy.kind {
			config::ns::DeployKind::Local { .. } => ("local".to_string(), None),
			config::ns::DeployKind::Cluster { .. } => {
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

		NomadCtx {
			project_ctx: ctx.clone(),
			client: reqwest::Client::new(),
			region: region_id.clone(),
			access_secret,
		}
	}

	pub fn build_nomad_request(
		&self,
		method: reqwest::Method,
		path: impl Display,
	) -> reqwest::RequestBuilder {
		match self.project_ctx.ns().deploy.kind {
			config::ns::DeployKind::Local { .. } => self
				.client
				.request(method, format!("http://nomad.service.consul:4646{path}")),

			config::ns::DeployKind::Cluster { .. } => {
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

	pub fn build_consul_request(
		&self,
		method: reqwest::Method,
		path: impl Display,
	) -> reqwest::RequestBuilder {
		match self.project_ctx.ns().deploy.kind {
			config::ns::DeployKind::Local { .. } => self
				.client
				.request(method, format!("http://consul.service.consul:8500{path}")),

			config::ns::DeployKind::Cluster { .. } => {
				let access_secret = self.access_secret.as_ref().unwrap();
				self.client
					.request(
						method,
						format!("https://consul.{}{path}", self.project_ctx.domain_main()),
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
