use std::cmp::Ordering;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rivet_auth_policy::{OperationKind, OwnedGrant, ResourceKind, Scope};
use rivet_data::{AUTH_GRANTS_VERSION, generated::auth_grants_v2, versioned::AuthGrantsData};
use vbare::OwnedVersionedData;

use crate::{Id, TokenError};

pub const MAX_GRANTS: usize = 32;
pub const MAX_OPERATIONS_PER_GRANT: usize = 7;
pub const MAX_GRANTS_BYTES: usize = 2_048;
const MAX_ENCODED_GRANTS_BYTES: usize = 2_731;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedGrantSet {
	grants: Vec<OwnedGrant<Id, Id>>,
}

impl ValidatedGrantSet {
	pub fn new(
		namespace_id: Id,
		grants: impl IntoIterator<Item = OwnedGrant<Id, Id>>,
	) -> Result<Self, TokenError> {
		let grants = grants.into_iter().collect::<Vec<_>>();
		if grants.is_empty() || grants.len() > MAX_GRANTS {
			return Err(TokenError::InvalidGrants);
		}
		if grants.iter().any(|grant| {
			!matches!(grant.namespace, Scope::Id(id) if id == namespace_id)
				|| matches!(grant.resource, ResourceKind::Jwt | ResourceKind::Token)
				|| grant.operations.is_empty()
				|| grant.operations.len() > MAX_OPERATIONS_PER_GRANT
		}) {
			return Err(TokenError::InvalidGrants);
		}

		let grants = canonicalize_grants(grants);
		if grants.is_empty()
			|| grants.len() > MAX_GRANTS
			|| grants
				.iter()
				.any(|grant| grant.operations.len() > MAX_OPERATIONS_PER_GRANT)
		{
			return Err(TokenError::InvalidGrants);
		}

		Ok(Self { grants })
	}

	pub fn grants(&self) -> &[OwnedGrant<Id, Id>] {
		&self.grants
	}

	pub fn into_grants(self) -> Vec<OwnedGrant<Id, Id>> {
		self.grants
	}
}

pub fn encode_grants(grants: &ValidatedGrantSet) -> Result<String, TokenError> {
	let version = if grants.grants.iter().any(|g| {
		matches!(
			g.resource,
			ResourceKind::ActorStream
				| ResourceKind::DurableStream
				| ResourceKind::ApplicationToken
				| ResourceKind::WebhookSigning
		) || g
			.operations
			.iter()
			.any(|o| matches!(o, OperationKind::Append | OperationKind::Close))
	}) {
		AUTH_GRANTS_VERSION
	} else {
		1
	};
	encode_grants_version(grants, version)
}

fn encode_grants_version(grants: &ValidatedGrantSet, version: u16) -> Result<String, TokenError> {
	let data = auth_grants_v2::Data {
		grants: grants
			.grants
			.iter()
			.cloned()
			.map(|grant| auth_grants_v2::Grant {
				resource: encode_resource(grant.resource),
				target: match grant.target {
					Scope::Any => auth_grants_v2::TargetScope::Any,
					Scope::Id(id) => auth_grants_v2::TargetScope::Id(id.as_bytes()),
				},
				operations: grant.operations.into_iter().map(encode_operation).collect(),
			})
			.collect(),
	};

	let encoded = AuthGrantsData::wrap_latest(data)
		.serialize_with_embedded_version(version)
		.map_err(|_| TokenError::InvalidGrants)?;
	if encoded.len() > MAX_GRANTS_BYTES {
		return Err(TokenError::InvalidGrants);
	}

	Ok(URL_SAFE_NO_PAD.encode(encoded))
}

fn canonicalize_grants(
	grants: impl IntoIterator<Item = OwnedGrant<Id, Id>>,
) -> Vec<OwnedGrant<Id, Id>> {
	let mut grants = grants
		.into_iter()
		.map(|mut grant| {
			grant.operations.sort_unstable();
			grant.operations.dedup();
			grant
		})
		.collect::<Vec<_>>();

	grants.sort_unstable_by(compare_grant_selector);

	let mut canonical = Vec::<OwnedGrant<Id, Id>>::with_capacity(grants.len());
	for grant in grants {
		if let Some(previous) = canonical.last_mut()
			&& compare_grant_selector(previous, &grant) == Ordering::Equal
		{
			previous.operations.extend(grant.operations);
			previous.operations.sort_unstable();
			previous.operations.dedup();
		} else {
			canonical.push(grant);
		}
	}

	canonical
}

fn compare_grant_selector(a: &OwnedGrant<Id, Id>, b: &OwnedGrant<Id, Id>) -> Ordering {
	// This order feeds the signed JWT grant encoding and is therefore protocol-significant.
	// Reordering `Scope` or `ResourceKind` variants requires a new grants schema version.
	a.namespace
		.cmp(&b.namespace)
		.then_with(|| a.resource.cmp(&b.resource))
		.then_with(|| a.target.cmp(&b.target))
}

pub fn decode_grants(
	namespace_id: Id,
	encoded: &str,
) -> Result<Vec<OwnedGrant<Id, Id>>, TokenError> {
	if encoded.is_empty() || encoded.len() > MAX_ENCODED_GRANTS_BYTES || encoded.contains('=') {
		return Err(TokenError::InvalidGrants);
	}

	let decoded = URL_SAFE_NO_PAD
		.decode(encoded)
		.map_err(|_| TokenError::InvalidGrants)?;
	if decoded.len() > MAX_GRANTS_BYTES || URL_SAFE_NO_PAD.encode(&decoded) != encoded {
		return Err(TokenError::InvalidGrants);
	}

	let data = AuthGrantsData::deserialize_with_embedded_version(&decoded)
		.map_err(|_| TokenError::InvalidGrants)?;
	if data.grants.is_empty() || data.grants.len() > MAX_GRANTS {
		return Err(TokenError::InvalidGrants);
	}

	let mut grants = Vec::with_capacity(data.grants.len());
	for grant in data.grants {
		if grant.operations.is_empty() || grant.operations.len() > MAX_OPERATIONS_PER_GRANT {
			return Err(TokenError::InvalidGrants);
		}

		grants.push(OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: decode_resource(grant.resource)?,
			target: match grant.target {
				auth_grants_v2::TargetScope::Any => Scope::Any,
				auth_grants_v2::TargetScope::Id(id) => {
					Scope::Id(Id::from_slice(&id).map_err(|_| TokenError::InvalidGrants)?)
				}
			},
			operations: grant.operations.into_iter().map(decode_operation).collect(),
		});
	}

	let validated = ValidatedGrantSet::new(namespace_id, grants)?;
	let version = u16::from_le_bytes([decoded[0], decoded[1]]);
	if encode_grants_version(&validated, version).as_deref() != Ok(encoded) {
		return Err(TokenError::InvalidGrants);
	}

	Ok(validated.into_grants())
}

fn encode_resource(resource: ResourceKind) -> auth_grants_v2::ResourceKind {
	match resource {
		ResourceKind::Namespace => auth_grants_v2::ResourceKind::Namespace,
		ResourceKind::Actor => auth_grants_v2::ResourceKind::Actor,
		ResourceKind::Runner => auth_grants_v2::ResourceKind::Runner,
		ResourceKind::RunnerConfig => auth_grants_v2::ResourceKind::RunnerConfig,
		ResourceKind::Token => auth_grants_v2::ResourceKind::Token,
		ResourceKind::Datacenter => auth_grants_v2::ResourceKind::Datacenter,
		ResourceKind::ActorGateway => auth_grants_v2::ResourceKind::ActorGateway,
		ResourceKind::ActorKv => auth_grants_v2::ResourceKind::ActorKv,
		ResourceKind::ActorStream => auth_grants_v2::ResourceKind::ActorStream,
		ResourceKind::DurableStream => auth_grants_v2::ResourceKind::DurableStream,
		ResourceKind::ApplicationToken => auth_grants_v2::ResourceKind::ApplicationToken,
		ResourceKind::WebhookSigning => auth_grants_v2::ResourceKind::WebhookSigning,
		ResourceKind::Jwt => unreachable!("control-plane grants are rejected during validation"),
	}
}

fn decode_resource(resource: auth_grants_v2::ResourceKind) -> Result<ResourceKind, TokenError> {
	Ok(match resource {
		auth_grants_v2::ResourceKind::Namespace => ResourceKind::Namespace,
		auth_grants_v2::ResourceKind::Actor => ResourceKind::Actor,
		auth_grants_v2::ResourceKind::Runner => ResourceKind::Runner,
		auth_grants_v2::ResourceKind::RunnerConfig => ResourceKind::RunnerConfig,
		auth_grants_v2::ResourceKind::Token => ResourceKind::Token,
		auth_grants_v2::ResourceKind::Reserved => return Err(TokenError::InvalidGrants),
		auth_grants_v2::ResourceKind::Datacenter => ResourceKind::Datacenter,
		auth_grants_v2::ResourceKind::ActorGateway => ResourceKind::ActorGateway,
		auth_grants_v2::ResourceKind::ActorKv => ResourceKind::ActorKv,
		auth_grants_v2::ResourceKind::ActorStream => ResourceKind::ActorStream,
		auth_grants_v2::ResourceKind::DurableStream => ResourceKind::DurableStream,
		auth_grants_v2::ResourceKind::ApplicationToken => ResourceKind::ApplicationToken,
		auth_grants_v2::ResourceKind::WebhookSigning => ResourceKind::WebhookSigning,
	})
}

fn encode_operation(operation: OperationKind) -> auth_grants_v2::OperationKind {
	match operation {
		OperationKind::Read => auth_grants_v2::OperationKind::Read,
		OperationKind::Update => auth_grants_v2::OperationKind::Update,
		OperationKind::List => auth_grants_v2::OperationKind::List,
		OperationKind::Create => auth_grants_v2::OperationKind::Create,
		OperationKind::Delete => auth_grants_v2::OperationKind::Delete,
		OperationKind::Append => auth_grants_v2::OperationKind::Append,
		OperationKind::Close => auth_grants_v2::OperationKind::Close,
	}
}

fn decode_operation(operation: auth_grants_v2::OperationKind) -> OperationKind {
	match operation {
		auth_grants_v2::OperationKind::Read => OperationKind::Read,
		auth_grants_v2::OperationKind::Update => OperationKind::Update,
		auth_grants_v2::OperationKind::List => OperationKind::List,
		auth_grants_v2::OperationKind::Create => OperationKind::Create,
		auth_grants_v2::OperationKind::Delete => OperationKind::Delete,
		auth_grants_v2::OperationKind::Append => OperationKind::Append,
		auth_grants_v2::OperationKind::Close => OperationKind::Close,
	}
}
