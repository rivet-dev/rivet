#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikHttpNullified {
	#[allow(missing_docs)] // documentation missing in model
	pub services: std::option::Option<
		std::collections::HashMap<std::string::String, TraefikService>,
	>,
	#[allow(missing_docs)] // documentation missing in model
	pub routers: std::option::Option<
		std::collections::HashMap<std::string::String, TraefikRouter>,
	>,
	#[allow(missing_docs)] // documentation missing in model
	pub middlewares: std::option::Option<
		std::collections::HashMap<std::string::String, TraefikMiddleware>,
	>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraefikMiddleware {
	#[allow(missing_docs)] // documentation missing in model
	AddPrefix(TraefikMiddlewareAddPrefix),
	#[allow(missing_docs)] // documentation missing in model
	BasicAuth(TraefikMiddlewareBasicAuth),
	#[allow(missing_docs)] // documentation missing in model
	Chain(TraefikMiddlewareChain),
	#[allow(missing_docs)] // documentation missing in model
	Compress(Unit),
	#[allow(missing_docs)] // documentation missing in model
	Headers(TraefikMiddlewareHeaders),
	#[allow(missing_docs)] // documentation missing in model
	InFlightReq(TraefikMiddlewareInFlightReq),
	#[allow(missing_docs)] // documentation missing in model
	IpWhiteList(TraefikMiddlewareIpWhiteList),
	#[allow(missing_docs)] // documentation missing in model
	RateLimit(TraefikMiddlewareRateLimit),
	#[allow(missing_docs)] // documentation missing in model
	RedirectRegex(TraefikMiddlewareRedirectRegex),
	#[allow(missing_docs)] // documentation missing in model
	ReplacePathRegex(TraefikMiddlewareReplacePathRegex),
	#[allow(missing_docs)] // documentation missing in model
	Retry(TraefikMiddlewareRetry),
	#[allow(missing_docs)] // documentation missing in model
	StripPrefix(TraefikMiddlewareStripPrefix),
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareBasicAuth {
	#[allow(missing_docs)] // documentation missing in model
	pub users: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub remove_header: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareRedirectRegex {
	#[allow(missing_docs)] // documentation missing in model
	pub permanent: bool,
	#[allow(missing_docs)] // documentation missing in model
	pub regex: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub replacement: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareHeaders {
	#[allow(missing_docs)] // documentation missing in model
	pub access_control_allow_methods: std::option::Option<std::vec::Vec<std::string::String>>,
	#[allow(missing_docs)] // documentation missing in model
	pub access_control_allow_origin_list: std::option::Option<std::vec::Vec<std::string::String>>,
	/// Unsigned 32 bit integer.
	pub access_control_max_age: std::option::Option<i32>,
	#[allow(missing_docs)] // documentation missing in model
	pub custom_response_headers:
		std::option::Option<std::collections::HashMap<std::string::String, std::string::String>>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unit {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareRetry {
	/// Unsigned 32 bit integer.
	pub attempts: i32,
	#[allow(missing_docs)] // documentation missing in model
	pub initial_interval: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareInFlightReq {
	/// Unsigned 32 bit integer.
	pub amount: i32,
	#[allow(missing_docs)] // documentation missing in model
	pub source_criterion: std::option::Option<TraefikInFlightReqSourceCriterion>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraefikInFlightReqSourceCriterion {
	#[allow(missing_docs)] // documentation missing in model
	IpStrategy(TraefikIpStrategy),
	#[allow(missing_docs)] // documentation missing in model
	RequestHeaderName(TraefikInFlightReqSourceCriterionRequestHeaderName),
	#[allow(missing_docs)] // documentation missing in model
	RequestHost(Unit),
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikInFlightReqSourceCriterionRequestHeaderName {
	#[allow(missing_docs)] // documentation missing in model
	pub request_header_name: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikIpStrategy {
	/// Unsigned 32 bit integer.
	pub depth: i32,
	#[allow(missing_docs)] // documentation missing in model
	pub exclude_i_ps: std::option::Option<std::vec::Vec<std::string::String>>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareRateLimit {
	/// Unsigned 32 bit integer.
	pub average: i32,
	#[allow(missing_docs)] // documentation missing in model
	pub period: std::string::String,
	/// Unsigned 32 bit integer.
	pub burst: i32,
	#[allow(missing_docs)] // documentation missing in model
	pub source_criterion: TraefikInFlightReqSourceCriterion,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareAddPrefix {
	#[allow(missing_docs)] // documentation missing in model
	pub prefix: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareStripPrefix {
	#[allow(missing_docs)] // documentation missing in model
	pub prefixes: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub force_slash: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareReplacePathRegex {
	#[allow(missing_docs)] // documentation missing in model
	pub regex: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub replacement: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareIpWhiteList {
	#[allow(missing_docs)] // documentation missing in model
	pub source_range: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub ip_strategy: std::option::Option<TraefikIpStrategy>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikMiddlewareChain {
	#[allow(missing_docs)] // documentation missing in model
	pub middlewares: std::vec::Vec<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikRouter {
	#[allow(missing_docs)] // documentation missing in model
	pub entry_points: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub rule: std::string::String,
	/// Unsigned 32 bit integer.
	pub priority: std::option::Option<i32>,
	#[allow(missing_docs)] // documentation missing in model
	pub service: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub middlewares: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub tls: std::option::Option<TraefikTls>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikTls {
	#[allow(missing_docs)] // documentation missing in model
	pub cert_resolver: std::option::Option<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub domains: std::option::Option<std::vec::Vec<TraefikTlsDomain>>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikTlsDomain {
	#[allow(missing_docs)] // documentation missing in model
	pub main: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub sans: std::vec::Vec<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikService {
	#[allow(missing_docs)] // documentation missing in model
	pub load_balancer: TraefikLoadBalancer,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikLoadBalancer {
	#[allow(missing_docs)] // documentation missing in model
	pub servers: std::vec::Vec<TraefikServer>,
	#[allow(missing_docs)] // documentation missing in model
	pub sticky: std::option::Option<TraefikLoadBalancerSticky>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraefikLoadBalancerSticky {
	#[allow(missing_docs)] // documentation missing in model
	Cookie(Unit),
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikServer {
	#[allow(missing_docs)] // documentation missing in model
	pub url: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikConfigRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraefikConfigResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub http: TraefikHttpNullified,
}

