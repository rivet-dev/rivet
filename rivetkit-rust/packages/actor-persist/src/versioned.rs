use anyhow::{Result, bail};
use vbare::{
	Body, Bytes, Converter, DecodeError, DecodeErrorKind, Encode, EncodeError, Message, Sink,
	Versioned,
	wire::{self, Wire},
};

use crate::generated::{v1, v2, v3, v4};

pub enum Actor {
	V1(v1::PersistedActor),
	V2(v2::PersistedActor),
	V3(v3::Actor),
	V4(v4::Actor),
}

impl Versioned for Actor {
	const LATEST: u16 = 4;

	type Latest = v4::Actor;

	/// Latest-version bodies also go through the legacy raw schedule args fallback, so this
	/// replaces the provided method, which decodes them directly.
	fn decode_body(body: Bytes, version: u16) -> std::result::Result<Self::Latest, DecodeError> {
		if version == Self::LATEST {
			return Self::decode_v4_with_legacy_raw_args_fallback(body)
				.map_err(DecodeError::conversion);
		}
		if version == 0 || version > Self::LATEST {
			return Err(unsupported_version(format!(
				"invalid actor persist version: {version}"
			)));
		}
		let mut value = Self::decode_version(version, body)?;
		for upgrade in Self::UPGRADES.iter().skip(usize::from(version) - 1) {
			value = upgrade(value).map_err(DecodeError::conversion)?;
		}
		value.unwrap_latest().map_err(DecodeError::conversion)
	}

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn decode_version(version: u16, payload: Bytes) -> std::result::Result<Self, DecodeError> {
		match version {
			1 => Ok(Self::V1(Message::decode(payload)?)),
			2 => Ok(Self::V2(Message::decode(payload)?)),
			3 => Ok(Self::V3(Message::decode(payload)?)),
			4 => Ok(Self::V4(
				Self::decode_v4_with_legacy_raw_args_fallback(payload)
					.map_err(DecodeError::conversion)?,
			)),
			_ => Err(unsupported_version(format!(
				"invalid actor persist version: {version}"
			))),
		}
	}

	fn encode_version<S: Sink>(
		&self,
		version: u16,
		out: &mut S,
	) -> std::result::Result<(), EncodeError> {
		match (self, version) {
			(Self::V1(data), 1) => {
				data.encode(out);
				Ok(())
			}
			(Self::V2(data), 2) => {
				data.encode(out);
				Ok(())
			}
			(Self::V3(data), 3) => {
				data.encode(out);
				Ok(())
			}
			(Self::V4(data), 4) => {
				data.encode(out);
				Ok(())
			}
			(_, version) => Err(EncodeError::UnsupportedVersion(version)),
		}
	}

	const UPGRADES: &'static [Converter<Self>] = &[Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4];

	const DOWNGRADES: &'static [Converter<Self>] =
		&[Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1];
}

impl Actor {
	fn decode_v4_with_legacy_raw_args_fallback(payload: Bytes) -> Result<v4::Actor> {
		#[derive(serde::Deserialize)]
		struct LegacyRawV4Actor {
			input: Option<Vec<u8>>,
			has_initialized: bool,
			state: Vec<u8>,
			scheduled_events: Vec<LegacyRawV4ScheduleEvent>,
		}

		#[derive(serde::Deserialize)]
		struct LegacyRawV4ScheduleEvent {
			event_id: String,
			timestamp_ms: i64,
			action: String,
			args: Vec<u8>,
		}

		// Legacy raw args can look like current v4 to a lenient decoder. The decoder accepts only
		// canonical bytes, such as bools of exactly 0 or 1, so a legacy payload never decodes here.
		let current_error = match v4::Actor::decode(payload.clone()) {
			Ok(actor) => return Ok(actor),
			Err(error) => error,
		};

		// A short-lived v4 writer stored schedule args as raw `data` while
		// the fixed v4 schema expects `optional<Cbor>`. Decode that reused
		// version so actors persisted by the bad writer can restart.
		match serde_bare::from_slice::<LegacyRawV4Actor>(&payload) {
			Ok(actor) => Ok(v4::Actor {
				input: actor.input.map(Into::into),
				has_initialized: actor.has_initialized,
				state: actor.state.into(),
				scheduled_events: actor
					.scheduled_events
					.into_iter()
					.map(|event| v4::ScheduleEvent {
						event_id: event.event_id.into(),
						timestamp: event.timestamp_ms,
						action: event.action.into(),
						args: (!event.args.is_empty()).then_some(event.args.into()),
					})
					.collect(),
			}),
			Err(_) => Err(current_error.into()),
		}
	}

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

		impl Versioned for $name {
			const LATEST: u16 = 4;

			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V4(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V4(data) => Ok(data),
				}
			}

			fn decode_version(
				version: u16,
				payload: Bytes,
			) -> std::result::Result<Self, DecodeError> {
				match version {
					4 => Ok(Self::V4(Message::decode(payload)?)),
					_ => Err(unsupported_version(format!(
						concat!($label, " only exists in actor persist v4, got {}"),
						version
					))),
				}
			}

			fn encode_version<S: Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), EncodeError> {
				match (self, version) {
					(Self::V4(data), 4) => {
						data.encode(out);
						Ok(())
					}
					(_, version) => Err(EncodeError::UnsupportedVersion(version)),
				}
			}

			const UPGRADES: &'static [Converter<Self>] = &[Ok, Ok, Ok];

			const DOWNGRADES: &'static [Converter<Self>] = &[Ok, Ok, Ok];
		}
	};
}

impl_v4_only!(QueueMetadata, v4::QueueMetadata, "queue metadata");
impl_v4_only!(QueueMessage, v4::QueueMessage, "queue message");

pub enum Conn {
	V3(v3::Conn),
	V4(v4::Conn),
}

impl Versioned for Conn {
	const LATEST: u16 = 4;

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

	fn decode_version(version: u16, payload: Bytes) -> std::result::Result<Self, DecodeError> {
		match version {
			3 => Ok(Self::V3(Message::decode(payload)?)),
			4 => Ok(Self::V4(Message::decode(payload)?)),
			_ => Err(unsupported_version(format!(
				"connection only exists in actor persist v3+, got {version}"
			))),
		}
	}

	fn encode_version<S: Sink>(
		&self,
		version: u16,
		out: &mut S,
	) -> std::result::Result<(), EncodeError> {
		match (self, version) {
			(Self::V3(data), 3) => {
				data.encode(out);
				Ok(())
			}
			(Self::V4(data), 4) => {
				data.encode(out);
				Ok(())
			}
			(_, version) => Err(EncodeError::UnsupportedVersion(version)),
		}
	}

	const UPGRADES: &'static [Converter<Self>] = &[Ok, Ok, Self::v3_to_v4];

	const DOWNGRADES: &'static [Converter<Self>] = &[Self::v4_to_v3, Ok, Ok];
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

/// Implements [`Versioned`] for a single-version envelope `enum $name { V1($latest) }` whose
/// latest value is a [`vbare::Body`] but not a generated message.
macro_rules! impl_v1_body {
	($name:ident, $latest:ty, $label:literal) => {
		impl Versioned for $name {
			const LATEST: u16 = 1;

			type Latest = $latest;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V1(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				let Self::V1(latest) = self;
				Ok(latest)
			}

			fn decode_version(
				version: u16,
				_payload: Bytes,
			) -> std::result::Result<Self, DecodeError> {
				Err(unsupported_version(format!(
					concat!("invalid ", $label, " version: {}"),
					version
				)))
			}

			fn encode_version<S: Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), EncodeError> {
				if version != 1 {
					return Err(EncodeError::UnsupportedVersion(version));
				}
				let Self::V1(latest) = self;
				latest.put_body(out);
				Ok(())
			}
		}
	};
}

/// The physical alarm last pushed to the engine, stored as BARE `optional<i64>`.
pub enum LastPushedAlarm {
	V1(Option<i64>),
}

impl_v1_body!(LastPushedAlarm, Option<i64>, "last pushed alarm");

/// Logical run-handler deadline stored separately from the shared physical
/// alarm. This uses its own versioned type so its persisted meaning cannot be
/// conflated with `LastPushedAlarm` even though both currently encode an
/// optional millisecond timestamp.
pub enum RunWakeAt {
	V1(Option<i64>),
}

impl_v1_body!(RunWakeAt, Option<i64>, "run wake deadline");

/// The span and ray of the last workflow run, which the next run continues. It is stored as
/// three BARE `optional<str>` fields in declaration order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkflowTraceContextV1 {
	pub ray_id: Option<String>,
	pub traceparent: Option<String>,
	pub tracestate: Option<String>,
}

impl WorkflowTraceContextV1 {
	fn fields(&self) -> [&Option<String>; 3] {
		[&self.ray_id, &self.traceparent, &self.tracestate]
	}
}

impl Body for WorkflowTraceContextV1 {
	fn body_len(&self) -> usize {
		self.fields()
			.into_iter()
			.map(|field| match field {
				Some(value) => {
					1 + <wire::Uint as Wire>::encoded_len(&(value.len() as u64)) + value.len()
				}
				None => 1,
			})
			.sum()
	}

	fn put_body<S: Sink>(&self, out: &mut S) {
		for field in self.fields() {
			match field {
				Some(value) => {
					out.put_u8(1);
					vbare::varint::put_uint(out, value.len() as u64);
					out.put_slice(value.as_bytes());
				}
				None => out.put_u8(0),
			}
		}
	}

	fn from_body(bytes: Bytes) -> std::result::Result<Self, DecodeError> {
		type OptionalStr = wire::Optional<wire::Str>;

		vbare::decode_with(&bytes, |v| {
			Ok(WorkflowTraceContextV1 {
				ray_id: OptionalStr::verify_owned(v)?.map(String::from),
				traceparent: OptionalStr::verify_owned(v)?.map(String::from),
				tracestate: OptionalStr::verify_owned(v)?.map(String::from),
			})
		})
	}
}

pub enum WorkflowTraceContext {
	V1(WorkflowTraceContextV1),
}

impl_v1_body!(
	WorkflowTraceContext,
	WorkflowTraceContextV1,
	"workflow trace context"
);

/// The error for a version this envelope cannot decode.
fn unsupported_version(detail: String) -> DecodeError {
	DecodeError::with_detail(DecodeErrorKind::UnsupportedVersion, 0, detail)
}
