//! Types shared by authentication mechanisms and authorization callers.

use std::fmt;

use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::ToSchema;

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	ToSchema,
	EnumIter,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = AuthResourceKind)]
pub enum ResourceKind {
	Namespace,
	Actor,
	Runner,
	RunnerConfig,
	Token,
	Datacenter,
	ActorGateway,
	ActorKv,
	Jwt,
}

impl fmt::Display for ResourceKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			ResourceKind::Actor => "actors",
			ResourceKind::ActorGateway => "actor gateways",
			ResourceKind::ActorKv => "actor KVs",
			ResourceKind::Jwt => "JWTs",
			ResourceKind::Runner => "runners",
			ResourceKind::RunnerConfig => "runner configs",
			ResourceKind::Token => "tokens",
			ResourceKind::Namespace => "namespaces",
			ResourceKind::Datacenter => "datacenters",
		})
	}
}

impl ResourceKind {
	pub fn to_kebab_case(&self) -> &'static str {
		match self {
			ResourceKind::Actor => "actor",
			ResourceKind::ActorGateway => "actor-gateway",
			ResourceKind::ActorKv => "actor-kv",
			ResourceKind::Jwt => "jwt",
			ResourceKind::Runner => "runner",
			ResourceKind::RunnerConfig => "runner-config",
			ResourceKind::Token => "token",
			ResourceKind::Namespace => "namespace",
			ResourceKind::Datacenter => "datacenter",
		}
	}
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	ToSchema,
	EnumIter,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = AuthOperationKind)]
pub enum OperationKind {
	Read,
	Update,
	List,
	Create,
	Delete,
}

impl fmt::Display for OperationKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			OperationKind::Create => "create",
			OperationKind::Read => "read",
			OperationKind::Update => "update",
			OperationKind::Delete => "delete",
			OperationKind::List => "list",
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Scope<T> {
	Any,
	Id(T),
}

impl<T> Scope<T> {
	pub fn as_ref(&self) -> Scope<&T> {
		match self {
			Scope::Any => Scope::Any,
			Scope::Id(id) => Scope::Id(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessRequest<NamespaceId, TargetId> {
	pub namespace: Scope<NamespaceId>,
	pub resource: ResourceKind,
	pub target: Scope<TargetId>,
	pub operation: OperationKind,
}

#[derive(Debug, Clone, Copy)]
pub struct Grant<'a, NamespaceId, TargetId> {
	pub namespace: Scope<NamespaceId>,
	pub resource: ResourceKind,
	pub target: Scope<TargetId>,
	pub operations: &'a [OperationKind],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedGrant<NamespaceId, TargetId> {
	pub namespace: Scope<NamespaceId>,
	pub resource: ResourceKind,
	pub target: Scope<TargetId>,
	pub operations: Vec<OperationKind>,
}

/// The complete normalized grant set carried by one authenticated credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveAuthority<NamespaceId, TargetId> {
	grants: Vec<OwnedGrant<NamespaceId, TargetId>>,
}

impl<NamespaceId, TargetId> EffectiveAuthority<NamespaceId, TargetId> {
	pub fn new(grants: Vec<OwnedGrant<NamespaceId, TargetId>>) -> Self {
		Self { grants }
	}

	pub fn grants(&self) -> &[OwnedGrant<NamespaceId, TargetId>] {
		&self.grants
	}

	pub fn into_grants(self) -> Vec<OwnedGrant<NamespaceId, TargetId>> {
		self.grants
	}
}

impl<NamespaceId, TargetId> OwnedGrant<NamespaceId, TargetId> {
	pub fn as_grant(&self) -> Grant<'_, &NamespaceId, &TargetId> {
		Grant {
			namespace: self.namespace.as_ref(),
			resource: self.resource,
			target: self.target.as_ref(),
			operations: &self.operations,
		}
	}
}
