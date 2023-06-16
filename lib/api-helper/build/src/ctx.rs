use std::net::IpAddr;

use rivet_operation::OperationContext;
use types::rivet::backend;
use url::Url;

use crate::auth;

pub struct Ctx<A: auth::ApiAuth> {
	pub(crate) auth: A,
	pub(crate) op_ctx: OperationContext<()>,
	pub(crate) user_agent: Option<String>,
	pub(crate) origin: Option<Url>,
	pub(crate) remote_address: Option<IpAddr>,
	pub(crate) coords: Option<(f64, f64)>,
	pub(crate) asn: Option<u32>,
}

impl<A: auth::ApiAuth> Ctx<A> {
	pub fn auth(&self) -> &A {
		&self.auth
	}

	pub fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	pub fn cache(&self) -> rivet_cache::RequestConfig {
		self.op_ctx.cache()
	}

	pub fn cache_handle(&self) -> rivet_cache::Cache {
		self.op_ctx.cache_handle()
	}

	pub fn client_info(&self) -> backend::net::ClientInfo {
		backend::net::ClientInfo {
			user_agent: self.user_agent.clone(),
			remote_address: self.remote_address.map(|ra| ra.to_string()),
		}
	}

	pub fn user_agent(&self) -> Option<&str> {
		self.user_agent.as_ref().map(String::as_str)
	}

	pub fn origin(&self) -> Option<&Url> {
		self.origin.as_ref()
	}

	pub fn remote_address(&self) -> Option<IpAddr> {
		self.remote_address
	}

	/// A Cloudflare worker includes the request meta automatically so instead of looking up an IP in a Geo IP
	/// database like we used to, we use whatever Cloudflare tells us is the geo IP is since that’s way faster
	/// and cheaper.
	pub fn coords(&self) -> Option<(f64, f64)> {
		self.coords
	}

	pub fn asn(&self) -> Option<u32> {
		self.asn
	}
}
