//! Downgrades JWT grant sets from version 2 to version 1. Version 1 has no fork operation, so a
//! grant set that holds it is refused as [`Inexpressible`]; every other grant set downgrades
//! unchanged.

use anyhow::Result;

use crate::{
	generated::{auth_grants_v1, auth_grants_v2},
	versioned::Inexpressible,
};

const FORK_OPERATION: &str = "fork operation";

fn convert_target_scope_auth_grants_v2_to_auth_grants_v1(
	x: auth_grants_v2::TargetScope,
) -> Result<auth_grants_v1::TargetScope> {
	Ok(match x {
		auth_grants_v2::TargetScope::Any => auth_grants_v1::TargetScope::Any,
		auth_grants_v2::TargetScope::Id(v) => auth_grants_v1::TargetScope::Id(v),
	})
}

fn convert_resource_kind_auth_grants_v2_to_auth_grants_v1(
	x: auth_grants_v2::ResourceKind,
) -> Result<auth_grants_v1::ResourceKind> {
	Ok(match x {
		auth_grants_v2::ResourceKind::Namespace => auth_grants_v1::ResourceKind::Namespace,
		auth_grants_v2::ResourceKind::Actor => auth_grants_v1::ResourceKind::Actor,
		auth_grants_v2::ResourceKind::Runner => auth_grants_v1::ResourceKind::Runner,
		auth_grants_v2::ResourceKind::RunnerPool => auth_grants_v1::ResourceKind::RunnerPool,
		auth_grants_v2::ResourceKind::Token => auth_grants_v1::ResourceKind::Token,
		auth_grants_v2::ResourceKind::Reserved => auth_grants_v1::ResourceKind::Reserved,
		auth_grants_v2::ResourceKind::Datacenter => auth_grants_v1::ResourceKind::Datacenter,
		auth_grants_v2::ResourceKind::ActorGateway => auth_grants_v1::ResourceKind::ActorGateway,
		auth_grants_v2::ResourceKind::ActorKv => auth_grants_v1::ResourceKind::ActorKv,
	})
}

fn convert_operation_kind_auth_grants_v2_to_auth_grants_v1(
	x: auth_grants_v2::OperationKind,
) -> Result<auth_grants_v1::OperationKind> {
	Ok(match x {
		auth_grants_v2::OperationKind::Read => auth_grants_v1::OperationKind::Read,
		auth_grants_v2::OperationKind::Update => auth_grants_v1::OperationKind::Update,
		auth_grants_v2::OperationKind::List => auth_grants_v1::OperationKind::List,
		auth_grants_v2::OperationKind::Create => auth_grants_v1::OperationKind::Create,
		auth_grants_v2::OperationKind::Delete => auth_grants_v1::OperationKind::Delete,
		auth_grants_v2::OperationKind::Fork => return Err(Inexpressible::lacks(1, FORK_OPERATION)),
	})
}

fn convert_grant_auth_grants_v2_to_auth_grants_v1(
	x: auth_grants_v2::Grant,
) -> Result<auth_grants_v1::Grant> {
	Ok(auth_grants_v1::Grant {
		resource: convert_resource_kind_auth_grants_v2_to_auth_grants_v1(x.resource)?,
		target: convert_target_scope_auth_grants_v2_to_auth_grants_v1(x.target)?,
		operations: x
			.operations
			.into_iter()
			.map(|v| convert_operation_kind_auth_grants_v2_to_auth_grants_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub(super) fn convert_data_auth_grants_v2_to_auth_grants_v1(
	x: auth_grants_v2::Data,
) -> Result<auth_grants_v1::Data> {
	Ok(auth_grants_v1::Data {
		grants: x
			.grants
			.into_iter()
			.map(|v| convert_grant_auth_grants_v2_to_auth_grants_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}
