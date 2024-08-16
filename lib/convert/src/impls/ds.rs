use std::collections::HashMap;

use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;
use util::timestamp;

use crate::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};
use serde_json::{json, to_value};

impl ApiTryFrom<backend::ds::Server> for models::ServersServer {
	type Error = GlobalError;
	fn api_try_from(value: backend::ds::Server) -> GlobalResult<models::ServersServer> {
		Ok(models::ServersServer {
			id: unwrap!(value.server_id).as_uuid(),
			environment: unwrap!(value.env_id).as_uuid(),
			datacenter: unwrap!(value.datacenter_id).as_uuid(),
			cluster: unwrap!(value.cluster_id).as_uuid(),
			created_at: value.create_ts,
			started_at: value.start_ts,
			destroyed_at: value.destroy_ts,
			tags: Some(to_value(value.tags).unwrap()),
			runtime: Box::new(models::ServersRuntime {
				image: unwrap!(value.image_id).as_uuid(),
				arguments: Some(value.args),
				environment: Some(value.environment),
			}),
			network: Box::new(models::ServersNetwork {
				mode: Some(
					unwrap!(backend::ds::NetworkMode::from_i32(value.network_mode)).api_into(),
				),
				ports: value
					.network_ports
					.into_iter()
					.map(|(s, p)| Ok((s, p.api_try_into()?)))
					.collect::<GlobalResult<HashMap<_, _>>>()?,
			}),
			lifecycle: Box::new(models::ServersLifecycle {
				kill_timeout: Some(value.kill_timeout_ms),
			}),
			resources: Box::new(unwrap!(value.resources).api_into()),
		})
	}
}

impl ApiFrom<models::ServersResources> for backend::ds::ServerResources {
	fn api_from(value: models::ServersResources) -> backend::ds::ServerResources {
		backend::ds::ServerResources {
			cpu_millicores: value.cpu,
			memory_mib: value.memory,
		}
	}
}

impl ApiFrom<backend::ds::ServerResources> for models::ServersResources {
	fn api_from(value: backend::ds::ServerResources) -> models::ServersResources {
		models::ServersResources {
			cpu: value.cpu_millicores,
			memory: value.memory_mib,
		}
	}
}

impl ApiFrom<models::ServersNetworkMode> for backend::ds::NetworkMode {
	fn api_from(value: models::ServersNetworkMode) -> backend::ds::NetworkMode {
		match value {
			models::ServersNetworkMode::Bridge => backend::ds::NetworkMode::Bridge,
			models::ServersNetworkMode::Host => backend::ds::NetworkMode::Host,
		}
	}
}

impl ApiFrom<backend::ds::NetworkMode> for models::ServersNetworkMode {
	fn api_from(value: backend::ds::NetworkMode) -> models::ServersNetworkMode {
		match value {
			backend::ds::NetworkMode::Bridge => models::ServersNetworkMode::Bridge,
			backend::ds::NetworkMode::Host => models::ServersNetworkMode::Host,
		}
	}
}

impl ApiTryFrom<backend::ds::Port> for models::ServersPort {
	type Error = GlobalError;

	fn api_try_from(value: backend::ds::Port) -> GlobalResult<models::ServersPort> {
		let protocol = match unwrap!(&value.routing) {
			backend::ds::port::Routing::GameGuard(x) => {
				unwrap!(backend::ds::GameGuardProtocol::from_i32(x.protocol)).api_into()
			}
			backend::ds::port::Routing::Host(x) => {
				unwrap!(backend::ds::HostProtocol::from_i32(x.protocol)).api_into()
			}
		};

		let routing = models::ServersPortRouting {
			game_guard: if let Some(backend::ds::port::Routing::GameGuard(_)) = &value.routing {
				Some(json!({}))
			} else {
				None
			},
			host: if let Some(backend::ds::port::Routing::Host(_)) = &value.routing {
				Some(json!({}))
			} else {
				None
			},
		};

		Ok(models::ServersPort {
			protocol,
			internal_port: value.internal_port,
			public_hostname: value.public_hostname,
			public_port: value.public_port,
			routing: Box::new(routing),
		})
	}
}

impl ApiFrom<models::ServersPortProtocol> for backend::ds::GameGuardProtocol {
	fn api_from(value: models::ServersPortProtocol) -> backend::ds::GameGuardProtocol {
		match value {
			models::ServersPortProtocol::Udp => backend::ds::GameGuardProtocol::Udp,
			models::ServersPortProtocol::Tcp => backend::ds::GameGuardProtocol::Tcp,
			models::ServersPortProtocol::Http => backend::ds::GameGuardProtocol::Http,
			models::ServersPortProtocol::Https => backend::ds::GameGuardProtocol::Https,
			models::ServersPortProtocol::TcpTls => backend::ds::GameGuardProtocol::TcpTls,
		}
	}
}

impl ApiFrom<backend::ds::GameGuardProtocol> for models::ServersPortProtocol {
	fn api_from(value: backend::ds::GameGuardProtocol) -> models::ServersPortProtocol {
		match value {
			backend::ds::GameGuardProtocol::Udp => models::ServersPortProtocol::Udp,
			backend::ds::GameGuardProtocol::Tcp => models::ServersPortProtocol::Tcp,
			backend::ds::GameGuardProtocol::Http => models::ServersPortProtocol::Http,
			backend::ds::GameGuardProtocol::Https => models::ServersPortProtocol::Https,
			backend::ds::GameGuardProtocol::TcpTls => models::ServersPortProtocol::TcpTls,
		}
	}
}

impl ApiTryFrom<models::ServersPortProtocol> for backend::ds::HostProtocol {
	type Error = GlobalError;
	fn api_try_from(value: models::ServersPortProtocol) -> GlobalResult<backend::ds::HostProtocol> {
		Ok(match value {
			models::ServersPortProtocol::Udp => backend::ds::HostProtocol::HostUdp,
			models::ServersPortProtocol::Tcp => backend::ds::HostProtocol::HostTcp,
			_ => bail_with!(SERVERS_UNSUPPORTED_HOST_PROTOCOL),
		})
	}
}

impl ApiFrom<backend::ds::HostProtocol> for models::ServersPortProtocol {
	fn api_from(value: backend::ds::HostProtocol) -> models::ServersPortProtocol {
		match value {
			backend::ds::HostProtocol::HostUdp => models::ServersPortProtocol::Udp,
			backend::ds::HostProtocol::HostTcp => models::ServersPortProtocol::Tcp,
		}
	}
}
