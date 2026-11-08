use crate::generated::{topology_worker_heartbeat_v1, topology_worker_heartbeat_v2};

pub enum TopologyWorkerHeartbeatData {
	V1(topology_worker_heartbeat_v1::Data),
	V2(topology_worker_heartbeat_v2::Heartbeat),
}

impl_versioned_family!(TopologyWorkerHeartbeatData, TOPOLOGY_WORKER_HEARTBEAT);

impl ::vbare::Versioned for TopologyWorkerHeartbeatData {
	type Latest = topology_worker_heartbeat_v2::Heartbeat;

	const LATEST: u16 = 2;

	const UPGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
		Self::V1(data) => Ok(Self::V2(heartbeat_v1_to_v2(data))),
		Self::V2(_) => ::anyhow::bail!("expected version 1"),
	}];

	const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
		Self::V2(data) => Ok(Self::V1(heartbeat_v2_to_v1(data))),
		Self::V1(_) => ::anyhow::bail!("expected version 2"),
	}];

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
		match self {
			Self::V2(latest) => Ok(latest),
			Self::V1(_) => ::anyhow::bail!("expected latest version"),
		}
	}

	fn decode_version(
		version: u16,
		body: ::vbare::Bytes,
	) -> ::std::result::Result<Self, ::vbare::DecodeError> {
		match version {
			1 => Ok(Self::V1(
				<topology_worker_heartbeat_v1::Data as ::vbare::Body>::from_body(body)?,
			)),
			_ => Err(crate::versioned::unsupported_version(version, 2)),
		}
	}

	fn encode_version<S: ::vbare::Sink>(
		&self,
		version: u16,
		out: &mut S,
	) -> ::std::result::Result<(), ::vbare::EncodeError> {
		match (self, version) {
			(Self::V1(data), 1) => ::vbare::Body::put_body(data, out),
			(Self::V2(data), 2) => ::vbare::Body::put_body(data, out),
			_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
		}
		Ok(())
	}
}

/// A version 1 writer predates zones, so its worker reports none.
fn heartbeat_v1_to_v2(
	data: topology_worker_heartbeat_v1::Data,
) -> topology_worker_heartbeat_v2::Heartbeat {
	topology_worker_heartbeat_v2::Heartbeat {
		instance_id: data.instance_id,
		api_peer_address: data.api_peer_address,
		build_version: data.build_version,
		build_git_sha: data.build_git_sha,
		started_at_unix_ms: data.started_at_unix_ms,
		write_sequence: data.write_sequence,
		state: data.state,
		storage_capacity_millicores: data.storage_capacity_millicores,
		versions: data.versions,
		directory_lock: data.directory_lock,
		zone: None,
	}
}

/// Version 1 has no zone, so a downgrade drops it.
fn heartbeat_v2_to_v1(
	data: topology_worker_heartbeat_v2::Heartbeat,
) -> topology_worker_heartbeat_v1::Data {
	topology_worker_heartbeat_v1::Data {
		instance_id: data.instance_id,
		api_peer_address: data.api_peer_address,
		build_version: data.build_version,
		build_git_sha: data.build_git_sha,
		started_at_unix_ms: data.started_at_unix_ms,
		write_sequence: data.write_sequence,
		state: data.state,
		storage_capacity_millicores: data.storage_capacity_millicores,
		versions: data.versions,
		directory_lock: data.directory_lock,
	}
}
