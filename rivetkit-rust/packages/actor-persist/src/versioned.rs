use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3, v4};

pub enum Actor {
	V1(v1::PersistedActor),
	V2(v2::PersistedActor),
	V3(v3::Actor),
	V4(v4::Actor),
}

impl OwnedVersionedData for Actor {
	type Latest = v4::Actor;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid actor persist version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected actor persist version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl Actor {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(data) = self else {
			bail!("expected actor persist v1 Actor");
		};

		Ok(Self::V2(v2::PersistedActor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			connections: data
				.connections
				.into_iter()
				.map(|conn| v2::PersistedConnection {
					id: conn.id,
					token: conn.token,
					parameters: conn.parameters,
					state: conn.state,
					subscriptions: conn
						.subscriptions
						.into_iter()
						.map(|sub| v2::PersistedSubscription {
							event_name: sub.event_name,
						})
						.collect(),
					last_seen: conn.last_seen.min(i64::MAX as u64) as i64,
					hibernatable_request_id: None,
				})
				.collect(),
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| v2::PersistedScheduleEvent {
					event_id: event.event_id,
					timestamp: event.timestamp.min(i64::MAX as u64) as i64,
					kind: match event.kind {
						v1::PersistedScheduleEventKind::GenericPersistedScheduleEvent(kind) => {
							v2::PersistedScheduleEventKind::GenericPersistedScheduleEvent(
								v2::GenericPersistedScheduleEvent {
									action: kind.action,
									args: kind.args,
								},
							)
						}
					},
				})
				.collect(),
			hibernatable_web_sockets: Vec::new(),
		}))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected actor persist v2 Actor");
		};

		Ok(Self::V3(v3::Actor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| {
					let v2::PersistedScheduleEventKind::GenericPersistedScheduleEvent(kind) =
						event.kind;
					v3::ScheduleEvent {
						event_id: event.event_id,
						timestamp: event.timestamp,
						action: kind.action,
						args: kind.args,
					}
				})
				.collect(),
		}))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected actor persist v3 Actor");
		};

		Ok(Self::V4(v4::Actor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| v4::ScheduleEvent {
					event_id: event.event_id,
					timestamp: event.timestamp,
					action: event.action,
					args: event.args,
				})
				.collect(),
		}))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected actor persist v4 Actor");
		};

		Ok(Self::V3(v3::Actor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| v3::ScheduleEvent {
					event_id: event.event_id,
					timestamp: event.timestamp,
					action: event.action,
					args: event.args,
				})
				.collect(),
		}))
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected actor persist v3 Actor");
		};

		Ok(Self::V2(v2::PersistedActor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			connections: Vec::new(),
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| v2::PersistedScheduleEvent {
					event_id: event.event_id,
					timestamp: event.timestamp,
					kind: v2::PersistedScheduleEventKind::GenericPersistedScheduleEvent(
						v2::GenericPersistedScheduleEvent {
							action: event.action,
							args: event.args,
						},
					),
				})
				.collect(),
			hibernatable_web_sockets: Vec::new(),
		}))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected actor persist v2 Actor");
		};

		Ok(Self::V1(v1::PersistedActor {
			input: data.input,
			has_initialized: data.has_initialized,
			state: data.state,
			connections: data
				.connections
				.into_iter()
				.map(|conn| v1::PersistedConnection {
					id: conn.id,
					token: conn.token,
					parameters: conn.parameters,
					state: conn.state,
					subscriptions: conn
						.subscriptions
						.into_iter()
						.map(|sub| v1::PersistedSubscription {
							event_name: sub.event_name,
						})
						.collect(),
					last_seen: conn.last_seen.max(0) as u64,
				})
				.collect(),
			scheduled_events: data
				.scheduled_events
				.into_iter()
				.map(|event| v1::PersistedScheduleEvent {
					event_id: event.event_id,
					timestamp: event.timestamp.max(0) as u64,
					kind: match event.kind {
						v2::PersistedScheduleEventKind::GenericPersistedScheduleEvent(kind) => {
							v1::PersistedScheduleEventKind::GenericPersistedScheduleEvent(
								v1::GenericPersistedScheduleEvent {
									action: kind.action,
									args: kind.args,
								},
							)
						}
					},
				})
				.collect(),
		}))
	}
}

macro_rules! impl_v4_only {
	($name:ident, $latest_ty:path, $label:literal) => {
		pub enum $name {
			V4($latest_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V4(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V4(data) => Ok(data),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
					_ => bail!(
						concat!($label, " only exists in actor persist v4, got {}"),
						version
					),
				}
			}

			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				match (self, version) {
					(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
					(_, version) => bail!(
						concat!($label, " only exists in actor persist v4, got {}"),
						version
					),
				}
			}

			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Ok, Ok, Ok]
			}

			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Ok, Ok, Ok]
			}
		}
	};
}

impl_v4_only!(QueueMetadata, v4::QueueMetadata, "queue metadata");
impl_v4_only!(QueueMessage, v4::QueueMessage, "queue message");

pub enum Conn {
	V3(v3::Conn),
	V4(v4::Conn),
}

impl OwnedVersionedData for Conn {
	type Latest = v4::Conn;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("connection only exists in actor persist v3+, got {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("connection only exists in actor persist v3+, got {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Ok, Ok]
	}
}

impl Conn {
	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected actor persist v3 Conn");
		};
		Ok(Self::V4(v4::Conn {
			id: data.id,
			parameters: data.parameters,
			state: data.state,
			subscriptions: data
				.subscriptions
				.into_iter()
				.map(|sub| v4::Subscription {
					event_name: sub.event_name,
				})
				.collect(),
			gateway_id: data.gateway_id,
			request_id: data.request_id,
			server_message_index: data.server_message_index,
			client_message_index: data.client_message_index,
			request_path: data.request_path,
			request_headers: data.request_headers,
		}))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected actor persist v4 Conn");
		};
		Ok(Self::V3(v3::Conn {
			id: data.id,
			parameters: data.parameters,
			state: data.state,
			subscriptions: data
				.subscriptions
				.into_iter()
				.map(|sub| v3::Subscription {
					event_name: sub.event_name,
				})
				.collect(),
			gateway_id: data.gateway_id,
			request_id: data.request_id,
			server_message_index: data.server_message_index,
			client_message_index: data.client_message_index,
			request_path: data.request_path,
			request_headers: data.request_headers,
		}))
	}
}

pub enum LastPushedAlarm {
	V1(Option<i64>),
}

impl OwnedVersionedData for LastPushedAlarm {
	type Latest = Option<i64>;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid last pushed alarm version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected last pushed alarm version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		Vec::<fn(Self) -> Result<Self>>::new()
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		Vec::<fn(Self) -> Result<Self>>::new()
	}
}
