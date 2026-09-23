use rivet_auth_policy::{
	AccessRequest, Grant, OperationKind, OwnedGrant, ResourceKind, Scope, can_delegate,
	grant_allows, is_authorized,
};

const READ: &[OperationKind] = &[OperationKind::Read];

fn request(namespace: Scope<u64>, target: Scope<u64>) -> AccessRequest<u64, u64> {
	AccessRequest {
		namespace,
		resource: ResourceKind::Actor,
		target,
		operation: OperationKind::Read,
	}
}

fn grant(namespace: Scope<u64>, target: Scope<u64>) -> Grant<'static, u64, u64> {
	Grant {
		namespace,
		resource: ResourceKind::Actor,
		target,
		operations: READ,
	}
}

#[test]
fn denies_without_grants() {
	assert!(!is_authorized(&request(Scope::Any, Scope::Any), [],));
}

#[test]
fn matches_exact_and_wildcard_scopes() {
	let namespace_id = 1;
	let target_id = 2;

	assert!(grant_allows(
		&grant(Scope::Id(namespace_id), Scope::Id(target_id)),
		&request(Scope::Id(namespace_id), Scope::Id(target_id)),
	));
	assert!(grant_allows(
		&grant(Scope::Any, Scope::Any),
		&request(Scope::Id(namespace_id), Scope::Id(target_id)),
	));
}

#[test]
fn rejects_scope_mismatches() {
	let namespace_id = 1;
	let other_namespace_id = 2;
	let target_id = 3;
	let other_target_id = 4;

	assert!(!grant_allows(
		&grant(Scope::Id(namespace_id), Scope::Any),
		&request(Scope::Any, Scope::Any),
	));
	assert!(!grant_allows(
		&grant(Scope::Id(namespace_id), Scope::Any),
		&request(Scope::Id(other_namespace_id), Scope::Any),
	));
	assert!(!grant_allows(
		&grant(Scope::Any, Scope::Id(target_id)),
		&request(Scope::Any, Scope::Any),
	));
	assert!(!grant_allows(
		&grant(Scope::Any, Scope::Id(target_id)),
		&request(Scope::Any, Scope::Id(other_target_id)),
	));
}

#[test]
fn requires_resource_and_operation_match() {
	let request = request(Scope::Any, Scope::Any);

	assert!(!grant_allows(
		&Grant {
			namespace: Scope::Any,
			resource: ResourceKind::Runner,
			target: Scope::Any,
			operations: READ,
		},
		&request,
	));
	assert!(!grant_allows(
		&Grant {
			namespace: Scope::Any,
			resource: ResourceKind::Actor,
			target: Scope::Any,
			operations: &[OperationKind::Create],
		},
		&request,
	));
}

#[test]
fn permits_when_any_grant_matches() {
	let request = request(Scope::Any, Scope::Any);
	let grants = [
		Grant {
			resource: ResourceKind::Runner,
			..grant(Scope::Any, Scope::Any)
		},
		grant(Scope::Any, Scope::Any),
	];

	assert!(is_authorized(&request, grants));
}

fn owned_grant(
	namespace: Scope<u64>,
	resource: ResourceKind,
	target: Scope<u64>,
	operations: &[OperationKind],
) -> OwnedGrant<u64, u64> {
	OwnedGrant {
		namespace,
		resource,
		target,
		operations: operations.to_vec(),
	}
}

#[test]
fn delegates_only_equal_or_narrower_scopes() {
	let parent = [owned_grant(
		Scope::Any,
		ResourceKind::Actor,
		Scope::Any,
		&[OperationKind::Read],
	)];
	let requested = [owned_grant(
		Scope::Id(1),
		ResourceKind::Actor,
		Scope::Id(2),
		&[OperationKind::Read],
	)];

	assert!(can_delegate(&parent, &requested));
	assert!(!can_delegate(&requested, &parent));
}

#[test]
fn delegation_supports_operations_split_across_parent_grants() {
	let parent = [
		owned_grant(
			Scope::Id(1),
			ResourceKind::Actor,
			Scope::Id(2),
			&[OperationKind::Read],
		),
		owned_grant(
			Scope::Id(1),
			ResourceKind::Actor,
			Scope::Id(2),
			&[OperationKind::Update],
		),
	];
	let requested = [owned_grant(
		Scope::Id(1),
		ResourceKind::Actor,
		Scope::Id(2),
		&[OperationKind::Read, OperationKind::Update],
	)];

	assert!(can_delegate(&parent, &requested));
}

#[test]
fn delegation_rejects_resource_target_and_operation_expansion() {
	let parent = [owned_grant(
		Scope::Id(1),
		ResourceKind::Actor,
		Scope::Id(2),
		&[OperationKind::Read],
	)];

	assert!(!can_delegate(
		&parent,
		&[owned_grant(
			Scope::Id(1),
			ResourceKind::ActorKv,
			Scope::Id(2),
			&[OperationKind::Read],
		)]
	));
	assert!(!can_delegate(
		&parent,
		&[owned_grant(
			Scope::Id(1),
			ResourceKind::Actor,
			Scope::Id(3),
			&[OperationKind::Read],
		)]
	));
	assert!(!can_delegate(
		&parent,
		&[owned_grant(
			Scope::Id(1),
			ResourceKind::Actor,
			Scope::Id(2),
			&[OperationKind::Delete],
		)]
	));
}
