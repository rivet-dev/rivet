use rivet_auth_policy::{OperationKind, ResourceKind};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = AuthTokenResource)]
pub enum Resource {
	Namespace,
	Actor,
	Runner,
	RunnerConfig,
	Datacenter,
	ActorGateway,
	ActorKv,
}

impl From<Resource> for ResourceKind {
	fn from(value: Resource) -> Self {
		match value {
			Resource::Namespace => Self::Namespace,
			Resource::Actor => Self::Actor,
			Resource::Runner => Self::Runner,
			Resource::RunnerConfig => Self::RunnerConfig,
			Resource::Datacenter => Self::Datacenter,
			Resource::ActorGateway => Self::ActorGateway,
			Resource::ActorKv => Self::ActorKv,
		}
	}
}

impl TryFrom<ResourceKind> for Resource {
	type Error = ();

	fn try_from(value: ResourceKind) -> Result<Self, Self::Error> {
		match value {
			ResourceKind::Namespace => Ok(Self::Namespace),
			ResourceKind::Actor => Ok(Self::Actor),
			ResourceKind::Runner => Ok(Self::Runner),
			ResourceKind::RunnerConfig => Ok(Self::RunnerConfig),
			ResourceKind::Datacenter => Ok(Self::Datacenter),
			ResourceKind::ActorGateway => Ok(Self::ActorGateway),
			ResourceKind::ActorKv => Ok(Self::ActorKv),
			_ => Err(()),
		}
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = AuthTokenTargetScope)]
pub enum TargetScope {
	Any,
	Id(rivet_util::Id),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AuthTokenGrant)]
pub struct Grant {
	pub resource: Resource,
	pub target: TargetScope,
	pub operations: Vec<OperationKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AuthTokenCreateRequest)]
pub struct CreateRequest {
	pub namespace: String,
	pub duration: Option<u64>,
	pub subject: Option<String>,
	pub grants: Vec<Grant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AuthTokenCreateResponse)]
pub struct CreateResponse {
	pub token: String,
	pub issued_ts: i64,
	pub expires_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AuthTokenInspectResponse)]
pub struct InspectResponse {
	pub namespace_id: rivet_util::Id,
	pub subject: Option<String>,
	pub grants: Vec<Grant>,
	pub issued_ts: i64,
	pub expires_ts: i64,
}
