use std::collections::HashMap;

use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};
use serde_json::{json, to_value};

impl ApiTryFrom<backend::build::Build> for models::GamesServersBuild {
	type Error = GlobalError;
	fn api_try_from(value: backend::build::Build) -> GlobalResult<models::GamesServersBuild> {
		Ok(models::GamesServersBuild {
			id: unwrap!(value.build_id).as_uuid(),
			upload: unwrap!(value.upload_id).as_uuid(),
			created_at: timestamp::to_string(value.create_ts)?,
			content_length: value.content_length,
			completed_at: value.completed_at.map(timestamp::to_string).transpose()?,
			tags: value.tags,
		})
	}
}

impl ApiTryFrom<backend::ds::Server> for models::GamesServersServer {
	type Error = GlobalError;
	fn api_try_from(value: backend::ds::Server) -> GlobalResult<models::GamesServersServer> {
		Ok(models::GamesServersServer {
			id: unwrap!(value.server_id).as_uuid(),
			cluster: unwrap!(value.cluster_id).as_uuid(),
			created_at: value.create_ts,
			started_at: value.start_ts,
			datacenter: unwrap!(value.datacenter_id).as_uuid(),
			destroyed_at: value.destroy_ts,
			game: unwrap!(value.game_id).as_uuid(),
			kill_timeout: Some(value.kill_timeout_ms),
			tags: Some(to_value(value.tags).unwrap()),
			resources: Box::new(unwrap!(value.resources).api_into()),
			arguments: Some(value.args),
			environment: Some(value.environment),
			image: unwrap!(value.image_id).as_uuid(),
			network: Box::new(models::GamesServersNetwork {
				mode: Some(
					unwrap!(backend::ds::NetworkMode::from_i32(value.network_mode)).api_into(),
				),
				ports: value
					.network_ports
					.into_iter()
					.map(|(s, p)| Ok((s, p.api_try_into()?)))
					.collect::<GlobalResult<HashMap<_, _>>>()?,
			}),
		})
	}
}

impl ApiFrom<models::GamesServersResources> for backend::ds::ServerResources {
	fn api_from(value: models::GamesServersResources) -> backend::ds::ServerResources {
		backend::ds::ServerResources {
			cpu_millicores: value.cpu,
			memory_mib: value.memory,
		}
	}
}

impl ApiFrom<backend::ds::ServerResources> for models::GamesServersResources {
	fn api_from(value: backend::ds::ServerResources) -> models::GamesServersResources {
		models::GamesServersResources {
			cpu: value.cpu_millicores,
			memory: value.memory_mib,
		}
	}
}

impl ApiFrom<models::GamesServersNetworkMode> for backend::ds::NetworkMode {
	fn api_from(value: models::GamesServersNetworkMode) -> backend::ds::NetworkMode {
		match value {
			models::GamesServersNetworkMode::Bridge => backend::ds::NetworkMode::Bridge,
			models::GamesServersNetworkMode::Host => backend::ds::NetworkMode::Host,
		}
	}
}

impl ApiFrom<backend::ds::NetworkMode> for models::GamesServersNetworkMode {
	fn api_from(value: backend::ds::NetworkMode) -> models::GamesServersNetworkMode {
		match value {
			backend::ds::NetworkMode::Bridge => models::GamesServersNetworkMode::Bridge,
			backend::ds::NetworkMode::Host => models::GamesServersNetworkMode::Host,
		}
	}
}

impl ApiTryFrom<backend::ds::Port> for models::GamesServersPort {
	type Error = GlobalError;

	fn api_try_from(value: backend::ds::Port) -> GlobalResult<models::GamesServersPort> {
		let protocol = match unwrap!(&value.routing) {
			backend::ds::port::Routing::GameGuard(x) => {
				unwrap!(backend::ds::GameGuardProtocol::from_i32(x.protocol)).api_into()
			}
			backend::ds::port::Routing::Host(x) => {
				unwrap!(backend::ds::HostProtocol::from_i32(x.protocol)).api_into()
			}
		};

		let routing = models::GamesServersPortRouting {
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

		Ok(models::GamesServersPort {
			protocol,
			internal_port: value.internal_port,
			public_hostname: value.public_hostname,
			public_port: value.public_port,
			routing: Box::new(routing),
		})
	}
}

impl ApiFrom<models::GamesServersPortProtocol> for backend::ds::GameGuardProtocol {
	fn api_from(value: models::GamesServersPortProtocol) -> backend::ds::GameGuardProtocol {
		match value {
			models::GamesServersPortProtocol::Udp => backend::ds::GameGuardProtocol::Udp,
			models::GamesServersPortProtocol::Tcp => backend::ds::GameGuardProtocol::Tcp,
			models::GamesServersPortProtocol::Http => backend::ds::GameGuardProtocol::Http,
			models::GamesServersPortProtocol::Https => backend::ds::GameGuardProtocol::Https,
			models::GamesServersPortProtocol::TcpTls => backend::ds::GameGuardProtocol::TcpTls,
		}
	}
}

impl ApiFrom<backend::ds::GameGuardProtocol> for models::GamesServersPortProtocol {
	fn api_from(value: backend::ds::GameGuardProtocol) -> models::GamesServersPortProtocol {
		match value {
			backend::ds::GameGuardProtocol::Udp => models::GamesServersPortProtocol::Udp,
			backend::ds::GameGuardProtocol::Tcp => models::GamesServersPortProtocol::Tcp,
			backend::ds::GameGuardProtocol::Http => models::GamesServersPortProtocol::Http,
			backend::ds::GameGuardProtocol::Https => models::GamesServersPortProtocol::Https,
			backend::ds::GameGuardProtocol::TcpTls => models::GamesServersPortProtocol::TcpTls,
		}
	}
}

impl ApiTryFrom<models::GamesServersPortProtocol> for backend::ds::HostProtocol {
	type Error = GlobalError;
	fn api_try_from(
		value: models::GamesServersPortProtocol,
	) -> GlobalResult<backend::ds::HostProtocol> {
		Ok(match value {
			models::GamesServersPortProtocol::Udp => backend::ds::HostProtocol::HostUdp,
			models::GamesServersPortProtocol::Tcp => backend::ds::HostProtocol::HostTcp,
			_ => bail_with!(SERVERS_UNSUPPORTED_HOST_PROTOCOL),
		})
	}
}

impl ApiFrom<backend::ds::HostProtocol> for models::GamesServersPortProtocol {
	fn api_from(value: backend::ds::HostProtocol) -> models::GamesServersPortProtocol {
		match value {
			backend::ds::HostProtocol::HostUdp => models::GamesServersPortProtocol::Udp,
			backend::ds::HostProtocol::HostTcp => models::GamesServersPortProtocol::Tcp,
		}
	}
}
