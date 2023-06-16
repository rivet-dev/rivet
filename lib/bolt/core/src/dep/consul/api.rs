use reqwest::Method;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::dep::nomad::NomadCtx;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Service {
	service_address: String,
	service_port: u16,
}

impl Service {
	pub fn address(&self) -> &str {
		&self.service_address
	}

	pub fn port(&self) -> u16 {
		self.service_port
	}

	pub fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(self.service_address.parse().unwrap(), self.service_port)
	}

	pub fn host(&self) -> String {
		format!("{}:{}", self.service_address, self.service_port)
	}
}

// TODO: Cache this
pub async fn service(nomad_ctx: &NomadCtx, name: &str, tag: &str, dc: Option<&str>) -> Service {
	let path = if let Some(dc) = dc {
		format!("/v1/catalog/service/{name}?dc={dc}",)
	} else {
		format!("/v1/catalog/service/{name}?filter=\"{tag}\" in ServiceTags",)
	};
	let res = nomad_ctx
		.build_consul_request(Method::GET, path)
		.send()
		.await
		.expect("consul request");
	assert!(res.status().is_success());

	let services = res.json::<Vec<Service>>().await.expect("parse body");

	let service = services
		.into_iter()
		.next()
		.expect(&format!("services {} does not exist", name));

	rivet_term::status::info("Consul", format!("{} -> {}", name, service.host()));

	service
}
