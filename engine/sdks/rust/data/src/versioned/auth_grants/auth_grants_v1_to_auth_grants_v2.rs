//! Upgrades JWT grant sets from version 1 to version 2. Version 2 only adds the fork operation,
//! so every version 1 grant set upgrades unchanged.

use anyhow::Result;

use crate::generated::{auth_grants_v1, auth_grants_v2};

fn convert_target_scope_auth_grants_v1_to_auth_grants_v2(
	x: auth_grants_v1::TargetScope,
) -> Result<auth_grants_v2::TargetScope> {
	Ok(match x {
		auth_grants_v1::TargetScope::Any => auth_grants_v2::TargetScope::Any,
		auth_grants_v1::TargetScope::Id(v) => auth_grants_v2::TargetScope::Id(v),
	})
}

fn convert_resource_kind_auth_grants_v1_to_auth_grants_v2(
	x: auth_grants_v1::ResourceKind,
) -> Result<auth_grants_v2::ResourceKind> {
	Ok(match x {
		auth_grants_v1::ResourceKind::Namespace => auth_grants_v2::ResourceKind::Namespace,
		auth_grants_v1::ResourceKind::Actor => auth_grants_v2::ResourceKind::Actor,
		auth_grants_v1::ResourceKind::Runner => auth_grants_v2::ResourceKind::Runner,
		auth_grants_v1::ResourceKind::RunnerPool => auth_grants_v2::ResourceKind::RunnerPool,
		auth_grants_v1::ResourceKind::Token => auth_grants_v2::ResourceKind::Token,
		auth_grants_v1::ResourceKind::Reserved => auth_grants_v2::ResourceKind::Reserved,
		auth_grants_v1::ResourceKind::Datacenter => auth_grants_v2::ResourceKind::Datacenter,
		auth_grants_v1::ResourceKind::ActorGateway => auth_grants_v2::ResourceKind::ActorGateway,
		auth_grants_v1::ResourceKind::ActorKv => auth_grants_v2::ResourceKind::ActorKv,
	})
}

fn convert_operation_kind_auth_grants_v1_to_auth_grants_v2(
	x: auth_grants_v1::OperationKind,
) -> Result<auth_grants_v2::OperationKind> {
	Ok(match x {
		auth_grants_v1::OperationKind::Read => auth_grants_v2::OperationKind::Read,
		auth_grants_v1::OperationKind::Update => auth_grants_v2::OperationKind::Update,
		auth_grants_v1::OperationKind::List => auth_grants_v2::OperationKind::List,
		auth_grants_v1::OperationKind::Create => auth_grants_v2::OperationKind::Create,
		auth_grants_v1::OperationKind::Delete => auth_grants_v2::OperationKind::Delete,
	})
}

fn convert_grant_auth_grants_v1_to_auth_grants_v2(
	x: auth_grants_v1::Grant,
) -> Result<auth_grants_v2::Grant> {
	Ok(auth_grants_v2::Grant {
		resource: convert_resource_kind_auth_grants_v1_to_auth_grants_v2(x.resource)?,
		target: convert_target_scope_auth_grants_v1_to_auth_grants_v2(x.target)?,
		operations: x
			.operations
			.into_iter()
			.map(|v| convert_operation_kind_auth_grants_v1_to_auth_grants_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub(super) fn convert_data_auth_grants_v1_to_auth_grants_v2(
	x: auth_grants_v1::Data,
) -> Result<auth_grants_v2::Data> {
	Ok(auth_grants_v2::Data {
		grants: x
			.grants
			.into_iter()
			.map(|v| convert_grant_auth_grants_v1_to_auth_grants_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}
