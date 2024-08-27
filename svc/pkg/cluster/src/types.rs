use std::{
	convert::{TryFrom, TryInto},
	net::{IpAddr, Ipv4Addr},
};

use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend;
use serde::{Deserialize, Serialize};
use strum::FromRepr;

#[derive(Debug, sqlx::FromRow)]
pub struct Cluster {
	pub cluster_id: Uuid,
	pub name_id: String,
	/// Unset for the default cluster.
	pub owner_team_id: Option<Uuid>,
	pub create_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub name_id: String,
	pub display_name: String,
	pub provider: Provider,
	pub provider_datacenter_id: String,
	pub provider_api_token: Option<String>,
	pub pools: Vec<Pool>,
	pub build_delivery_method: BuildDeliveryMethod,
	pub prebakes_enabled: bool,
	pub create_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Provider {
	Linode,
}

// Backwards compatibility
impl TryFrom<i64> for Provider {
	type Error = GlobalError;

	fn try_from(value: i64) -> GlobalResult<Self> {
		match value {
			0 => Ok(Provider::Linode),
			_ => bail!("unexpected Provider variant"),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Pool {
	pub pool_type: PoolType,
	/// See docs on failover (/docs/packages/cluster/SERVER_PROVISIONING.md#creating-a-new-server)
	pub hardware: Vec<Hardware>,
	pub desired_count: u32,
	pub min_count: u32,
	pub max_count: u32,
	pub drain_timeout: u64,
}

// Backwards compatibility
impl TryFrom<backend::cluster::Pool> for Pool {
	type Error = GlobalError;

	fn try_from(value: backend::cluster::Pool) -> GlobalResult<Self> {
		Ok(Pool {
			pool_type: (value.pool_type as i64).try_into()?,
			hardware: value
				.hardware
				.iter()
				.map(|h| Hardware {
					provider_hardware: h.provider_hardware.clone(),
				})
				.collect(),
			desired_count: value.desired_count,
			min_count: value.min_count,
			max_count: value.max_count,
			drain_timeout: value.drain_timeout,
		})
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum PoolType {
	Job,
	Gg,
	Ats,
}

// Backwards compatibility
impl TryFrom<i64> for PoolType {
	type Error = GlobalError;

	fn try_from(value: i64) -> GlobalResult<Self> {
		match value {
			0 => Ok(PoolType::Job),
			1 => Ok(PoolType::Gg),
			2 => Ok(PoolType::Ats),
			_ => bail!("unexpected PoolType variant"),
		}
	}
}
impl From<PoolType> for i64 {
	fn from(value: PoolType) -> i64 {
		match value {
			PoolType::Job => 0,
			PoolType::Gg => 1,
			PoolType::Ats => 2,
		}
	}
}

impl std::fmt::Display for PoolType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PoolType::Job => write!(f, "job"),
			PoolType::Gg => write!(f, "gg"),
			PoolType::Ats => write!(f, "ats"),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Hardware {
	pub provider_hardware: String,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct PoolUpdate {
	pub pool_type: PoolType,

	// Each can be optionally updated
	pub hardware: Vec<Hardware>,
	pub desired_count: Option<u32>,
	pub min_count: Option<u32>,
	pub max_count: Option<u32>,
	pub drain_timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum BuildDeliveryMethod {
	TrafficServer = 0,
	S3Direct = 1,
}

#[derive(Debug)]
pub struct Server {
	pub server_id: Uuid,
	pub datacenter_id: Uuid,
	pub pool_type: PoolType,
	pub provider_server_id: Option<String>,
	pub vlan_ip: Option<IpAddr>,
	pub public_ip: Option<IpAddr>,
	pub cloud_destroy_ts: Option<i64>,
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
	pub server_ids: Option<Vec<Uuid>>,
	pub datacenter_ids: Option<Vec<Uuid>>,
	pub cluster_ids: Option<Vec<Uuid>>,
	pub pool_types: Option<Vec<PoolType>>,
	pub public_ips: Option<Vec<Ipv4Addr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TlsState {
	Creating,
	Active,
	Renewing,
}

// Backwards compatibility
impl TryFrom<i64> for TlsState {
	type Error = GlobalError;

	fn try_from(value: i64) -> GlobalResult<Self> {
		match value {
			0 => Ok(TlsState::Creating),
			1 => Ok(TlsState::Active),
			2 => Ok(TlsState::Renewing),
			_ => bail!("unexpected TlsState variant"),
		}
	}
}
