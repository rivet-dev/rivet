use anyhow::Result;
use gas::prelude::*;
use std::collections::HashMap;
use universaldb::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Metric {
	/// Seconds (actor name)
	ActorAwake(String),
	/// Count (actor name)
	TotalActors(String),
	/// Bytes (actor name)
	KvStorageUsed(String),
	/// Bytes (actor name)
	KvRead(String),
	/// Bytes (actor name)
	KvWrite(String),
	/// Count (actor name)
	AlarmsSet(String),
	/// Bytes (actor name, type)
	GatewayIngress(String, String),
	/// Bytes (actor name)
	GatewayEgress(String, String),
	/// Count (actor name, type)
	Requests(String, String),
	/// Count (actor name, type)
	ActiveRequests(String, String),
}

impl Metric {
	pub fn variant(&self) -> MetricVariant {
		match self {
			Metric::ActorAwake(_) => MetricVariant::ActorAwake,
			Metric::TotalActors(_) => MetricVariant::TotalActors,
			Metric::KvStorageUsed(_) => MetricVariant::KvStorageUsed,
			Metric::KvRead(_) => MetricVariant::KvRead,
			Metric::KvWrite(_) => MetricVariant::KvWrite,
			Metric::AlarmsSet(_) => MetricVariant::AlarmsSet,
			Metric::GatewayIngress(_, _) => MetricVariant::GatewayIngress,
			Metric::GatewayEgress(_, _) => MetricVariant::GatewayEgress,
			Metric::Requests(_, _) => MetricVariant::Requests,
			Metric::ActiveRequests(_, _) => MetricVariant::ActiveRequests,
		}
	}

	pub fn attributes(&self) -> HashMap<String, String> {
		let entries = match self {
			Metric::ActorAwake(actor_name) => vec![("actor_name".to_string(), actor_name.clone())],
			Metric::TotalActors(actor_name) => vec![("actor_name".to_string(), actor_name.clone())],
			Metric::KvStorageUsed(actor_name) => {
				vec![("actor_name".to_string(), actor_name.clone())]
			}
			Metric::KvRead(actor_name) => vec![("actor_name".to_string(), actor_name.clone())],
			Metric::KvWrite(actor_name) => vec![("actor_name".to_string(), actor_name.clone())],
			Metric::AlarmsSet(actor_name) => vec![("actor_name".to_string(), actor_name.clone())],
			Metric::GatewayIngress(actor_name, req_type) => vec![
				("actor_name".to_string(), actor_name.clone()),
				("type".to_string(), req_type.clone()),
			],
			Metric::GatewayEgress(actor_name, req_type) => vec![
				("actor_name".to_string(), actor_name.clone()),
				("type".to_string(), req_type.clone()),
			],
			Metric::Requests(actor_name, req_type) => vec![
				("actor_name".to_string(), actor_name.clone()),
				("type".to_string(), req_type.clone()),
			],
			Metric::ActiveRequests(actor_name, req_type) => vec![
				("actor_name".to_string(), actor_name.clone()),
				("type".to_string(), req_type.clone()),
			],
		};

		entries.into_iter().collect()
	}
}

#[derive(strum::FromRepr)]
pub enum MetricVariant {
	ActorAwake = 0,
	TotalActors = 1,
	KvStorageUsed = 2,
	KvRead = 3,
	KvWrite = 4,
	AlarmsSet = 5,
	GatewayIngress = 6,
	GatewayEgress = 7,
	Requests = 8,
	ActiveRequests = 9,
}

impl std::fmt::Display for MetricVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MetricVariant::ActorAwake => write!(f, "actor_awake"),
			MetricVariant::TotalActors => write!(f, "total_actors"),
			MetricVariant::KvStorageUsed => write!(f, "kv_storage_used"),
			MetricVariant::KvRead => write!(f, "kv_read"),
			MetricVariant::KvWrite => write!(f, "kv_write"),
			MetricVariant::AlarmsSet => write!(f, "alarms_set"),
			MetricVariant::GatewayIngress => write!(f, "gateway_ingress"),
			MetricVariant::GatewayEgress => write!(f, "gateway_egress"),
			MetricVariant::Requests => write!(f, "requests"),
			MetricVariant::ActiveRequests => write!(f, "active_requests"),
		}
	}
}

#[derive(Debug)]
pub struct MetricKey {
	pub namespace_id: Id,
	pub metric: Metric,
}

impl MetricKey {
	pub fn new(namespace_id: Id, metric: Metric) -> Self {
		MetricKey {
			namespace_id,
			metric,
		}
	}

	pub fn subspace(namespace_id: Id) -> MetricSubspaceKey {
		MetricSubspaceKey::new(namespace_id)
	}
}

impl FormalKey for MetricKey {
	// IMPORTANT: Uses LE bytes, not BE
	/// Count.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for MetricKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (METRIC, self.namespace_id, self.metric.variant() as usize);
		offset += t.pack(w, tuple_depth)?;

		offset += match &self.metric {
			Metric::ActorAwake(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::TotalActors(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::KvStorageUsed(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::KvRead(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::KvWrite(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::AlarmsSet(actor_name) => actor_name.pack(w, tuple_depth)?,
			Metric::GatewayIngress(actor_name, req_type) => {
				(actor_name, req_type).pack(w, tuple_depth)?
			}
			Metric::GatewayEgress(actor_name, req_type) => {
				(actor_name, req_type).pack(w, tuple_depth)?
			}
			Metric::Requests(actor_name, req_type) => {
				(actor_name, req_type).pack(w, tuple_depth)?
			}
			Metric::ActiveRequests(actor_name, req_type) => {
				(actor_name, req_type).pack(w, tuple_depth)?
			}
		};

		std::result::Result::Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for MetricKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, variant)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;
		let variant = MetricVariant::from_repr(variant).ok_or_else(|| {
			PackError::Message(format!("invalid metric variant `{variant}` in key").into())
		})?;

		let (input, v) = match variant {
			MetricVariant::ActorAwake => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::ActorAwake(actor_name),
					},
				)
			}
			MetricVariant::TotalActors => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::TotalActors(actor_name),
					},
				)
			}
			MetricVariant::KvStorageUsed => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::KvStorageUsed(actor_name),
					},
				)
			}
			MetricVariant::KvRead => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::KvRead(actor_name),
					},
				)
			}
			MetricVariant::KvWrite => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::KvWrite(actor_name),
					},
				)
			}
			MetricVariant::AlarmsSet => {
				let (input, actor_name) = String::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::AlarmsSet(actor_name),
					},
				)
			}
			MetricVariant::GatewayIngress => {
				let (input, (actor_name, req_type)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::GatewayIngress(actor_name, req_type),
					},
				)
			}
			MetricVariant::GatewayEgress => {
				let (input, (actor_name, req_type)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::GatewayEgress(actor_name, req_type),
					},
				)
			}
			MetricVariant::Requests => {
				let (input, (actor_name, req_type)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::Requests(actor_name, req_type),
					},
				)
			}
			MetricVariant::ActiveRequests => {
				let (input, (actor_name, req_type)) =
					<(String, String)>::unpack(input, tuple_depth)?;

				(
					input,
					MetricKey {
						namespace_id,
						metric: Metric::ActiveRequests(actor_name, req_type),
					},
				)
			}
		};

		Ok((input, v))
	}
}

pub struct MetricSubspaceKey {
	namespace_id: Id,
}

impl MetricSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		MetricSubspaceKey { namespace_id }
	}
}

impl TuplePack for MetricSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (METRIC, self.namespace_id);
		t.pack(w, tuple_depth)
	}
}

pub fn inc(tx: &universaldb::Transaction, namespace_id: Id, metric: Metric, by: i64) {
	tx.atomic_op(
		&MetricKey::new(namespace_id, metric),
		&by.to_le_bytes(),
		MutationType::Add,
	);
}
