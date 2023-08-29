use serde::Serialize;

#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rule {
	proto: RuleProtocol,
	port: String,
	group: Option<&'static str>,
	host: Option<&'static str>,
}

impl Rule {
	pub fn group(group: &'static str, proto: RuleProtocol, port: impl ToString) -> Self {
		Self {
			proto,
			port: port.to_string(),
			group: Some(group),
			host: None,
		}
	}

	pub fn host(host: &'static str, proto: RuleProtocol, port: impl ToString) -> Self {
		Self {
			proto,
			port: port.to_string(),
			group: None,
			host: Some(host),
		}
	}
}

#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleProtocol {
	#[serde(rename = "any")]
	Any,
	#[serde(rename = "tcp")]
	TCP,
	#[serde(rename = "udp")]
	UDP,
	#[serde(rename = "icmp")]
	ICMP,
}

pub fn any() -> Vec<Rule> {
	vec![Rule::host("any", RuleProtocol::Any, "any")]
}

pub fn common() -> Vec<Rule> {
	vec![
		// ICMP to allow `ping` to work
		Rule::host("any", RuleProtocol::ICMP, "any"),
		// Nebula Prometheus
		Rule::group("role:prometheus", RuleProtocol::TCP, "4280"),
		// Node exporter
		Rule::group("role:prometheus", RuleProtocol::TCP, "9100"),
	]
}

// https://developer.hashicorp.com/consul/docs/install/ports
pub fn consul() -> Vec<Rule> {
	vec![
		// Serf LAN
		Rule::group("role:consul-server", RuleProtocol::UDP, "8301"),
		Rule::group("role:consul-server", RuleProtocol::TCP, "8301"),
		Rule::group("role:consul-client", RuleProtocol::UDP, "8301"),
		Rule::group("role:consul-client", RuleProtocol::TCP, "8301"),
		// Serf WAN
		Rule::group("role:consul-server", RuleProtocol::UDP, "8302"),
		Rule::group("role:consul-server", RuleProtocol::TCP, "8302"),
		Rule::group("role:consul-client", RuleProtocol::UDP, "8302"),
		Rule::group("role:consul-client", RuleProtocol::TCP, "8302"),
		// HTTP
		Rule::group("pool:svc", RuleProtocol::TCP, "8500"),
		Rule::group("pool:ing-px", RuleProtocol::TCP, "8500"),
		Rule::group("role:prometheus", RuleProtocol::TCP, "8500"),
	]
}

// https://developer.hashicorp.com/nomad/docs/install/production/requirements#ports-used
pub fn nomad() -> Vec<Rule> {
	vec![
		// Serf WAN
		Rule::group("role:nomad-server", RuleProtocol::UDP, "4648"),
		Rule::group("role:nomad-server", RuleProtocol::TCP, "4648"),
		Rule::group("role:nomad-client", RuleProtocol::UDP, "4648"),
		Rule::group("role:nomad-client", RuleProtocol::TCP, "4648"),
		// HTTP
		Rule::group("pool:svc", RuleProtocol::TCP, "4646"),
		Rule::group("role:prometheus", RuleProtocol::TCP, "4646"),
	]
}

pub fn nomad_dynamic() -> Vec<Rule> {
	vec![
		Rule::group("pool:svc", RuleProtocol::UDP, "20000-25999"),
		Rule::group("pool:svc", RuleProtocol::TCP, "20000-25999"),
		Rule::group("pool:ing-px", RuleProtocol::UDP, "20000-25999"),
		Rule::group("pool:ing-px", RuleProtocol::TCP, "20000-25999"),
		Rule::group("role:prometheus", RuleProtocol::UDP, "20000-25999"),
		Rule::group("role:prometheus", RuleProtocol::TCP, "20000-25999"),
	]
}

pub fn nats() -> Vec<Rule> {
	vec![
		// Client
		Rule::group("pool:svc", RuleProtocol::TCP, "4222"),
		// Cluster
		Rule::group("role:nats-server", RuleProtocol::TCP, "6222"),
		// Exporter
		Rule::group("role:prometheus", RuleProtocol::TCP, "7777"),
	]
}

pub fn redis(port: u16) -> Vec<Rule> {
	vec![Rule::group("pool:svc", RuleProtocol::TCP, port)]
}

pub fn traffic_server() -> Vec<Rule> {
	vec![
		Rule::group("pool:svc", RuleProtocol::TCP, "9300"),
		Rule::group("pool:ing-px", RuleProtocol::TCP, "9300"),
		Rule::group("pool:job", RuleProtocol::TCP, "9300"),
	]
}

pub fn cockroach() -> Vec<Rule> {
	vec![
		// SQL
		Rule::group("pool:svc", RuleProtocol::TCP, "26257"),
		// Listen
		Rule::group("role:cockroach", RuleProtocol::TCP, "26258"),
		// HTTP
		Rule::group("role:prometheus", RuleProtocol::TCP, "26300"),
	]
}

// See https://clickhouse.com/docs/en/guides/sre/network-ports/
pub fn clickhouse() -> Vec<Rule> {
	vec![
		// HTTP
		Rule::group("pool:svc", RuleProtocol::TCP, "8123"),
		// Native TCP
		Rule::group("pool:svc", RuleProtocol::TCP, "9000"),
		// Postgres
		Rule::group("pool:svc", RuleProtocol::TCP, "9005"),
		// Prometheus
		Rule::group("role:prometheus", RuleProtocol::TCP, "9363"),
	]
}

pub fn prometheus() -> Vec<Rule> {
	vec![
		Rule::group("pool:svc", RuleProtocol::TCP, "9090"),
		Rule::group("role:prometheus", RuleProtocol::TCP, "9090"),
	]
}

pub fn minio() -> Vec<Rule> {
	vec![
		Rule::group("pool:svc", RuleProtocol::TCP, "9000"),
		Rule::group("pool:job", RuleProtocol::TCP, "9000"),
	]
}

pub fn traefik() -> Vec<Rule> {
	vec![Rule::group("role:prometheus", RuleProtocol::TCP, "9980")]
}
