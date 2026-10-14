use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{auth_grants_v1, auth_grants_v2};

pub enum AuthGrantsData {
	V1(auth_grants_v1::Data),
	V2(auth_grants_v2::Data),
}

impl OwnedVersionedData for AuthGrantsData {
	type Latest = auth_grants_v2::Data;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let Self::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest")
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}
	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}
impl AuthGrantsData {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(x) = self else {
			bail!("unexpected version")
		};
		Ok(Self::V2(auth_grants_v2::Data {
			grants: x
				.grants
				.into_iter()
				.map(|g| {
					Ok(auth_grants_v2::Grant {
						resource: match g.resource {
							auth_grants_v1::ResourceKind::Namespace => {
								auth_grants_v2::ResourceKind::Namespace
							}
							auth_grants_v1::ResourceKind::Actor => {
								auth_grants_v2::ResourceKind::Actor
							}
							auth_grants_v1::ResourceKind::Runner => {
								auth_grants_v2::ResourceKind::Runner
							}
							auth_grants_v1::ResourceKind::RunnerConfig => {
								auth_grants_v2::ResourceKind::RunnerConfig
							}
							auth_grants_v1::ResourceKind::Token => {
								auth_grants_v2::ResourceKind::Token
							}
							auth_grants_v1::ResourceKind::Datacenter => {
								auth_grants_v2::ResourceKind::Datacenter
							}
							auth_grants_v1::ResourceKind::ActorGateway => {
								auth_grants_v2::ResourceKind::ActorGateway
							}
							auth_grants_v1::ResourceKind::ActorKv => {
								auth_grants_v2::ResourceKind::ActorKv
							}
							auth_grants_v1::ResourceKind::Reserved => {
								auth_grants_v2::ResourceKind::Reserved
							}
						},
						target: match g.target {
							auth_grants_v1::TargetScope::Any => auth_grants_v2::TargetScope::Any,
							auth_grants_v1::TargetScope::Id(id) => {
								auth_grants_v2::TargetScope::Id(id)
							}
						},
						operations: g
							.operations
							.into_iter()
							.map(|o| {
								Ok(match o {
									auth_grants_v1::OperationKind::Read => {
										auth_grants_v2::OperationKind::Read
									}
									auth_grants_v1::OperationKind::Update => {
										auth_grants_v2::OperationKind::Update
									}
									auth_grants_v1::OperationKind::List => {
										auth_grants_v2::OperationKind::List
									}
									auth_grants_v1::OperationKind::Create => {
										auth_grants_v2::OperationKind::Create
									}
									auth_grants_v1::OperationKind::Delete => {
										auth_grants_v2::OperationKind::Delete
									}
								})
							})
							.collect::<Result<Vec<_>>>()?,
					})
				})
				.collect::<Result<Vec<_>>>()?,
		}))
	}
	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(x) = self else {
			bail!("unexpected version")
		};
		Ok(Self::V1(auth_grants_v1::Data {
			grants: x
				.grants
				.into_iter()
				.map(|g| {
					Ok(auth_grants_v1::Grant {
						resource: match g.resource {
							auth_grants_v2::ResourceKind::Namespace => {
								auth_grants_v1::ResourceKind::Namespace
							}
							auth_grants_v2::ResourceKind::Actor => {
								auth_grants_v1::ResourceKind::Actor
							}
							auth_grants_v2::ResourceKind::Runner => {
								auth_grants_v1::ResourceKind::Runner
							}
							auth_grants_v2::ResourceKind::RunnerConfig => {
								auth_grants_v1::ResourceKind::RunnerConfig
							}
							auth_grants_v2::ResourceKind::Token => {
								auth_grants_v1::ResourceKind::Token
							}
							auth_grants_v2::ResourceKind::Datacenter => {
								auth_grants_v1::ResourceKind::Datacenter
							}
							auth_grants_v2::ResourceKind::ActorGateway => {
								auth_grants_v1::ResourceKind::ActorGateway
							}
							auth_grants_v2::ResourceKind::ActorKv => {
								auth_grants_v1::ResourceKind::ActorKv
							}
							auth_grants_v2::ResourceKind::Reserved => {
								auth_grants_v1::ResourceKind::Reserved
							}
							_ => bail!("grant requires the newer schema"),
						},
						target: match g.target {
							auth_grants_v2::TargetScope::Any => auth_grants_v1::TargetScope::Any,
							auth_grants_v2::TargetScope::Id(id) => {
								auth_grants_v1::TargetScope::Id(id)
							}
						},
						operations: g
							.operations
							.into_iter()
							.map(|o| {
								Ok(match o {
									auth_grants_v2::OperationKind::Read => {
										auth_grants_v1::OperationKind::Read
									}
									auth_grants_v2::OperationKind::Update => {
										auth_grants_v1::OperationKind::Update
									}
									auth_grants_v2::OperationKind::List => {
										auth_grants_v1::OperationKind::List
									}
									auth_grants_v2::OperationKind::Create => {
										auth_grants_v1::OperationKind::Create
									}
									auth_grants_v2::OperationKind::Delete => {
										auth_grants_v1::OperationKind::Delete
									}
									_ => bail!("grant requires the newer schema"),
								})
							})
							.collect::<Result<Vec<_>>>()?,
					})
				})
				.collect::<Result<Vec<_>>>()?,
		}))
	}
}
