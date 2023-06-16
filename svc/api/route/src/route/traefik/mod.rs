use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::auth::Auth;

mod cdn;
mod job_run;

// MARK: GET /traefik/config
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraefikConfigQuery {
	token: String,
	pool: String,
	region: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikConfigResponse {
	pub http: TraefikHttp,
	pub tcp: TraefikHttp,
	pub udp: TraefikHttp,
}

/// Traefik will throw an error if we don't list any services, so this lets us exclude empty maps.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikConfigResponseNullified {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub http: Option<TraefikHttpNullified>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tcp: Option<TraefikHttpNullified>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub udp: Option<TraefikHttpNullified>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikHttp {
	pub services: HashMap<String, TraefikService>,
	pub routers: HashMap<String, TraefikRouter>,
	pub middlewares: HashMap<String, TraefikMiddleware>,
}

/// See above.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikHttpNullified {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub services: Option<HashMap<String, TraefikService>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub routers: Option<HashMap<String, TraefikRouter>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub middlewares: Option<HashMap<String, TraefikMiddleware>>,
}

impl TraefikHttp {
	pub fn nullified(self) -> Option<TraefikHttpNullified> {
		if self.services.is_empty() && self.routers.is_empty() && self.middlewares.is_empty() {
			None
		} else {
			Some(TraefikHttpNullified {
				services: if self.services.is_empty() {
					None
				} else {
					Some(self.services)
				},
				routers: if self.routers.is_empty() {
					None
				} else {
					Some(self.routers)
				},
				middlewares: if self.middlewares.is_empty() {
					None
				} else {
					Some(self.middlewares)
				},
			})
		}
	}
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikService {
	pub load_balancer: TraefikLoadBalancer,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikLoadBalancer {
	#[serde(default)]
	pub servers: Vec<TraefikServer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sticky: Option<TraefikLoadBalancerSticky>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraefikLoadBalancerSticky {
	#[serde(rename = "cookie", rename_all = "camelCase")]
	Cookie {},
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikServer {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikRouter {
	pub entry_points: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rule: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub priority: Option<usize>,
	pub service: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub middlewares: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tls: Option<TraefikTls>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikTls {
	#[serde(skip_serializing_if = "Option::is_none")]
	cert_resolver: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	domains: Option<Vec<TraefikTlsDomain>>,
}

impl TraefikTls {
	/// Builds a `TraefikTls` object relevant to the environment.
	///
	/// We don't associate a cert resolver if in local development because we generate certificates
	/// with mkcert.
	fn build(domains: Vec<TraefikTlsDomain>) -> TraefikTls {
		TraefikTls {
			cert_resolver: None,
			domains: Some(domains),
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikTlsDomain {
	main: String,
	sans: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraefikMiddleware {
	#[serde(rename = "chain", rename_all = "camelCase")]
	Chain { middlewares: Vec<String> },
	#[serde(rename = "ipWhiteList", rename_all = "camelCase")]
	IpWhiteList {
		source_range: Vec<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		ip_strategy: Option<IpStrategy>,
	},
	#[serde(rename = "replacePathRegex", rename_all = "camelCase")]
	ReplacePathRegex { regex: String, replacement: String },
	#[serde(rename = "stripPrefix", rename_all = "camelCase")]
	StripPrefix {
		prefixes: Vec<String>,
		force_slash: bool,
	},
	#[serde(rename = "addPrefix", rename_all = "camelCase")]
	AddPrefix { prefix: String },
	#[serde(rename = "rateLimit", rename_all = "camelCase")]
	RateLimit {
		average: usize,
		period: String,
		burst: usize,
		source_criterion: InFlightReqSourceCriterion,
	},
	#[serde(rename = "inFlightReq", rename_all = "camelCase")]
	InFlightReq {
		amount: usize,
		source_criterion: InFlightReqSourceCriterion,
	},
	#[serde(rename = "retry", rename_all = "camelCase")]
	Retry {
		attempts: usize,
		initial_interval: String,
	},
	#[serde(rename = "compress", rename_all = "camelCase")]
	Compress {},
	#[serde(rename = "headers", rename_all = "camelCase")]
	Headers(TraefikMiddlewareHeaders),
	#[serde(rename = "redirectRegex", rename_all = "camelCase")]
	RedirectRegex {
		permanent: bool,
		regex: String,
		replacement: String,
	},
	#[serde(rename = "basicAuth", rename_all = "camelCase")]
	BasicAuth {
		users: Vec<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		realm: Option<String>,
		#[serde(default)]
		remove_header: bool,
	},
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraefikMiddlewareHeaders {
	#[serde(skip_serializing_if = "Option::is_none")]
	access_control_allow_methods: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	access_control_allow_origin_list: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	access_control_max_age: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	custom_request_headers: Option<HashMap<String, String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	custom_response_headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IpStrategy {
	depth: usize,

	#[serde(rename = "excludedIPs", skip_serializing_if = "Option::is_none")]
	exclude_ips: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum InFlightReqSourceCriterion {
	#[serde(rename = "ipStrategy")]
	IpStrategy(IpStrategy),
	#[serde(rename = "requestHeaderName", rename_all = "camelCase")]
	RequestHeaderName { request_header_name: String },
	#[serde(rename = "requestHost", rename_all = "camelCase")]
	RequestHost {},
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	TraefikConfigQuery {
		token,
		region,
		pool,
	}: TraefikConfigQuery,
) -> GlobalResult<TraefikConfigResponseNullified> {
	assert_eq_with!(
		token,
		util::env::read_secret(&["rivet", "api_route", "token"]).await?,
		API_FORBIDDEN,
		reason = "Invalid token"
	);

	// Fetch configs and catch any errors
	let (cdn_config, job_run_config) = tokio::join!(
		async {
			match cdn::build(&ctx, &pool).await {
				Ok(x) => x,
				Err(err) => {
					tracing::error!(?err, "error building cdn config");
					Default::default()
				}
			}
		},
		async {
			match job_run::build(&ctx, &pool, &region).await {
				Ok(x) => x,
				Err(err) => {
					tracing::error!(?err, "error building job run config");
					Default::default()
				}
			}
		},
	);

	// Merge configs
	let merged_config = TraefikConfigResponse {
		http: TraefikHttp {
			services: cdn_config
				.http
				.services
				.into_iter()
				.chain(job_run_config.http.services.into_iter())
				.collect(),
			routers: cdn_config
				.http
				.routers
				.into_iter()
				.chain(job_run_config.http.routers.into_iter())
				.collect(),
			middlewares: cdn_config
				.http
				.middlewares
				.into_iter()
				.chain(job_run_config.http.middlewares.into_iter())
				.collect(),
		},
		tcp: TraefikHttp {
			services: cdn_config
				.tcp
				.services
				.into_iter()
				.chain(job_run_config.tcp.services.into_iter())
				.collect(),
			routers: cdn_config
				.tcp
				.routers
				.into_iter()
				.chain(job_run_config.tcp.routers.into_iter())
				.collect(),
			middlewares: cdn_config
				.tcp
				.middlewares
				.into_iter()
				.chain(job_run_config.tcp.middlewares.into_iter())
				.collect(),
		},
		udp: TraefikHttp {
			services: cdn_config
				.udp
				.services
				.into_iter()
				.chain(job_run_config.udp.services.into_iter())
				.collect(),
			routers: cdn_config
				.udp
				.routers
				.into_iter()
				.chain(job_run_config.udp.routers.into_iter())
				.collect(),
			middlewares: cdn_config
				.udp
				.middlewares
				.into_iter()
				.chain(job_run_config.udp.middlewares.into_iter())
				.collect(),
		},
	};
	tracing::info!(
		http_services = ?merged_config.http.services.len(),
		http_routers = merged_config.http.routers.len(),
		http_middlewares = ?merged_config.http.middlewares.len(),
		tcp_services = ?merged_config.tcp.services.len(),
		tcp_routers = merged_config.tcp.routers.len(),
		tcp_middlewares = ?merged_config.tcp.middlewares.len(),
		udp_services = ?merged_config.udp.services.len(),
		udp_routers = merged_config.udp.routers.len(),
		udp_middlewares = ?merged_config.udp.middlewares.len(),
		"merged traefik config"
	);

	Ok(TraefikConfigResponseNullified {
		http: merged_config.http.nullified(),
		tcp: merged_config.tcp.nullified(),
		udp: merged_config.udp.nullified(),
	})
}
