use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rivet_auth_jwt::{
	Id, MAX_OPERATIONS_PER_GRANT, TokenError, ValidatedGrantSet, decode_grants,
	encode_grants as encode_validated_grants,
};
use rivet_auth_policy::{OperationKind, OwnedGrant, ResourceKind, Scope};

fn validate(
	namespace_id: Id,
	grants: &[OwnedGrant<Id, Id>],
) -> Result<ValidatedGrantSet, TokenError> {
	ValidatedGrantSet::new(namespace_id, grants.iter().cloned())
}

fn encode(namespace_id: Id, grants: &[OwnedGrant<Id, Id>]) -> Result<String, TokenError> {
	encode_validated_grants(&validate(namespace_id, grants)?)
}

const RESOURCES: [ResourceKind; 7] = [
	ResourceKind::Namespace,
	ResourceKind::Actor,
	ResourceKind::Runner,
	ResourceKind::RunnerConfig,
	ResourceKind::Datacenter,
	ResourceKind::ActorGateway,
	ResourceKind::ActorKv,
];

const OPERATIONS: [OperationKind; 5] = [
	OperationKind::Read,
	OperationKind::Update,
	OperationKind::List,
	OperationKind::Create,
	OperationKind::Delete,
];

#[test]
fn round_trips_every_shared_authorization_variant() {
	let namespace_id = Id::nil();
	let grants = RESOURCES.map(|resource| OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource,
		target: Scope::Any,
		operations: OPERATIONS.to_vec(),
	});

	let validated = validate(namespace_id, &grants).unwrap();
	let encoded = encode_validated_grants(&validated).unwrap();
	let decoded = decode_grants(namespace_id, &encoded).unwrap();

	assert_eq!(decoded, validated.into_grants());
	assert_eq!(OPERATIONS.len(), MAX_OPERATIONS_PER_GRANT);
}

#[test]
fn rejects_control_plane_credential_grants() {
	let namespace_id = Id::nil();
	for resource in [ResourceKind::Jwt, ResourceKind::Token] {
		let grants = [OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource,
			target: Scope::Any,
			operations: vec![OperationKind::Create],
		}];
		assert_eq!(
			validate(namespace_id, &grants),
			Err(TokenError::InvalidGrants)
		);
	}
}

#[test]
fn rejects_encoded_control_plane_credential_grants() {
	use rivet_data::{AUTH_GRANTS_VERSION, generated::auth_grants_v1, versioned::AuthGrantsData};
	use vbare::OwnedVersionedData;

	let namespace_id = Id::nil();
	for resource in [
		auth_grants_v1::ResourceKind::Token,
		auth_grants_v1::ResourceKind::Reserved,
	] {
		let data = auth_grants_v1::Data {
			grants: vec![auth_grants_v1::Grant {
				resource,
				target: auth_grants_v1::TargetScope::Any,
				operations: vec![auth_grants_v1::OperationKind::Create],
			}],
		};
		let bytes = AuthGrantsData::wrap_latest(data)
			.serialize_with_embedded_version(AUTH_GRANTS_VERSION)
			.unwrap();
		assert_eq!(
			decode_grants(namespace_id, &URL_SAFE_NO_PAD.encode(bytes)),
			Err(TokenError::InvalidGrants)
		);
	}
}

#[test]
fn round_trips_id_targets_and_normalizes_duplicate_operations() {
	let namespace_id = Id::new_v1(1);
	let target_id = Id::new_v1(2);
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Id(target_id),
		operations: vec![OperationKind::Read, OperationKind::Read],
	}];

	let validated = validate(namespace_id, &grants).unwrap();
	let encoded = encode_validated_grants(&validated).unwrap();
	let decoded = decode_grants(namespace_id, &encoded).unwrap();
	assert_eq!(decoded, validated.into_grants());
}

#[test]
fn canonicalizes_selector_order_and_merges_operations() {
	let namespace_id = Id::new_v1(1);
	let grants = [
		OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: ResourceKind::Runner,
			target: Scope::Any,
			operations: vec![OperationKind::Read],
		},
		OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: ResourceKind::Actor,
			target: Scope::Any,
			operations: vec![OperationKind::Update],
		},
		OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: ResourceKind::Actor,
			target: Scope::Any,
			operations: vec![OperationKind::Read],
		},
	];

	let validated = validate(namespace_id, &grants).unwrap();
	assert_eq!(validated.grants().len(), 2);
	assert_eq!(validated.grants()[0].resource, ResourceKind::Actor);
	assert_eq!(
		validated.grants()[0].operations,
		[OperationKind::Read, OperationKind::Update]
	);
}

#[test]
fn rejects_operation_limit_before_normalization() {
	let namespace_id = Id::nil();
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Any,
		operations: vec![OperationKind::Read; MAX_OPERATIONS_PER_GRANT + 1],
	}];

	assert_eq!(
		validate(namespace_id, &grants),
		Err(TokenError::InvalidGrants)
	);
}

#[test]
fn rejects_empty_and_cross_namespace_grants() {
	let namespace_id = Id::nil();
	assert_eq!(encode(namespace_id, &[]), Err(TokenError::InvalidGrants));

	let other_namespace_id = Id::new_v1(1);
	let grants = [OwnedGrant {
		namespace: Scope::Id(other_namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Any,
		operations: vec![OperationKind::Read],
	}];
	assert_eq!(
		encode(namespace_id, &grants),
		Err(TokenError::InvalidGrants)
	);
}

#[test]
fn rejects_unknown_versions_and_padded_base64() {
	let namespace_id = Id::nil();
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Any,
		operations: vec![OperationKind::Read],
	}];
	let encoded = encode(namespace_id, &grants).unwrap();

	let mut bytes = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
	bytes[..2].copy_from_slice(&2_u16.to_le_bytes());
	assert_eq!(
		decode_grants(namespace_id, &URL_SAFE_NO_PAD.encode(bytes)),
		Err(TokenError::InvalidGrants)
	);
	assert_eq!(
		decode_grants(namespace_id, &format!("{encoded}=")),
		Err(TokenError::InvalidGrants)
	);

	let mut trailing = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
	trailing.push(0);
	assert_eq!(
		decode_grants(namespace_id, &URL_SAFE_NO_PAD.encode(trailing)),
		Err(TokenError::InvalidGrants)
	);
}

#[test]
fn rejects_malformed_target_ids() {
	let namespace_id = Id::nil();
	let mut target_bytes = [2; 19];
	target_bytes[0] = 1;
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Id(Id::from_slice(&target_bytes).unwrap()),
		operations: vec![OperationKind::Read],
	}];
	let encoded = encode(namespace_id, &grants).unwrap();
	let bytes = URL_SAFE_NO_PAD.decode(encoded).unwrap();
	let target_start = bytes
		.windows(target_bytes.len())
		.position(|window| window == target_bytes)
		.unwrap();

	let mut invalid_version = bytes.clone();
	invalid_version[target_start] = 2;
	assert_eq!(
		decode_grants(namespace_id, &URL_SAFE_NO_PAD.encode(invalid_version)),
		Err(TokenError::InvalidGrants)
	);

	let mut invalid_length = bytes;
	invalid_length[target_start - 1] = 18;
	assert_eq!(
		decode_grants(namespace_id, &URL_SAFE_NO_PAD.encode(invalid_length)),
		Err(TokenError::InvalidGrants)
	);
}

#[test]
fn v1_wire_encoding_is_frozen() {
	let namespace_id = Id::nil();
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Any,
		operations: vec![OperationKind::Read, OperationKind::Update],
	}];

	assert_eq!(encode(namespace_id, &grants).unwrap(), "AQABAQACAAE");
}

#[test]
fn v1_id_target_wire_encoding_is_frozen() {
	let namespace_id = Id::nil();
	let mut target_bytes = [2; 19];
	target_bytes[0] = 1;
	let grants = [OwnedGrant {
		namespace: Scope::Id(namespace_id),
		resource: ResourceKind::Actor,
		target: Scope::Id(Id::from_slice(&target_bytes).unwrap()),
		operations: vec![OperationKind::Read],
	}];

	assert_eq!(
		encode(namespace_id, &grants).unwrap(),
		"AQABAQETAQICAgICAgICAgICAgICAgICAgEA"
	);
}
