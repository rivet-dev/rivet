//! Namespace JWTs issued by a persistent credential such as the admin token. Claims are stateless; the coordinator
//! owns the signing ring.
use crate::ApiError;
use axum::{
	Json,
	extract::State,
	http::HeaderMap,
	response::{IntoResponse, Response},
};
use rivet_auth_jwt::{Id, ValidatedGrantSet};
use rivet_auth_policy::{OwnedGrant, Scope};
use rivet_engine_context::EngineContext;
use rivet_guard_core::auth::{self, Credential, OperationKind, ResourceKind};
use rivet_topology::versions::{VersionFamily, WriteVersions};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
	Any,
	Id(Id),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
	pub resource: ResourceKind,
	pub target: Target,
	pub operations: Vec<OperationKind>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
	pub namespace: String,
	pub duration: Option<u64>,
	pub subject: Option<String>,
	pub grants: Vec<Grant>,
}

pub async fn create(
	State(ctx): State<EngineContext>,
	headers: HeaderMap,
	Json(body): Json<CreateRequest>,
) -> Result<Response, ApiError> {
	let jwt = &ctx.config().auth_required()?.jwt;
	if !jwt.issuance_enabled() {
		return Err(rivet_auth_policy::errors::Auth::IssuanceDisabled
			.build()
			.into());
	}
	// Issuance requires a presented persistent credential, including in insecure deployments.
	let credential = auth::authenticate(&ctx, auth::transport_token(&headers)).await?;
	credential.require_management()?;
	pegboard::catalog::ensure_namespace(&ctx, &body.namespace).await?;
	validate_request(&body, jwt.max_duration().as_secs())?;
	let grants = ValidatedGrantSet::new(
		body.namespace.clone(),
		body.grants.into_iter().map(|g| OwnedGrant {
			namespace: Scope::Id(body.namespace.clone()),
			resource: g.resource,
			target: match g.target {
				Target::Any => Scope::Any,
				Target::Id(id) => Scope::Id(id),
			},
			operations: g.operations,
		}),
	)
	.map_err(|_| {
		InvalidInput {
			message: "invalid JWT grant set".into(),
		}
		.build()
	})?;
	let duration = body
		.duration
		.unwrap_or_else(|| jwt.default_duration().as_secs());
	admit_grants(ctx.topology().as_ref(), &grants)?;
	let (token, issued_ts, expires_ts) = ctx
		.jwt()
		.issue(body.namespace, grants, body.subject, duration)
		.await
		.map_err(|error| {
			tracing::warn!(?error, "JWT signing ring is unavailable");
			rivet_auth_policy::errors::Auth::IssuanceUnavailable.build()
		})?;
	Ok(no_store(
		Json(json!({ "token": token, "issued_ts": issued_ts, "expires_ts": expires_ts }))
			.into_response(),
	))
}

/// Refuses a grant set that grants fork, with `auth.operation_unavailable` before the token is
/// signed, while a listed engine worker cannot decode the `auth.grants` version that carries it.
/// A set without fork is encoded at version 1, which every engine with JWTs decodes, so it never
/// waits.
///
/// Only this region's storage workers are listed, and stateless workers are never listed. A
/// token that grants fork is therefore accepted everywhere only once every region has upgraded
/// and, within a region, stateless workers roll before storage workers.
fn admit_grants(fleet: &dyn WriteVersions, grants: &ValidatedGrantSet) -> anyhow::Result<()> {
	if !grants.grants_fork() {
		return Ok(());
	}
	let fleet_reads = fleet.write_version(VersionFamily::AUTH_GRANTS);
	if matches!(fleet_reads, Ok(version) if version >= rivet_auth_jwt::FORK_GRANTS_VERSION) {
		return Ok(());
	}
	tracing::debug!(
		?fleet_reads,
		"refused a JWT that grants fork while an engine worker may not read it"
	);
	Err(rivet_auth_policy::errors::Auth::OperationUnavailable {
		operation: OperationKind::Fork.to_string(),
		retry_after_ms: rivet_auth_policy::errors::OPERATION_UNAVAILABLE_RETRY_AFTER_MS,
	}
	.build())
}

pub async fn inspect(
	State(ctx): State<EngineContext>,
	headers: HeaderMap,
) -> Result<Response, ApiError> {
	let credential = auth::authenticate(&ctx, auth::transport_token(&headers)).await?;
	let Credential::Jwt(jwt) = credential else {
		return Err(auth::InvalidToken.build().into());
	};
	let grants: Vec<_> = jwt
		.token
		.grants
		.iter()
		.map(|g| Grant {
			resource: g.resource,
			target: match g.target {
				Scope::Any => Target::Any,
				Scope::Id(id) => Target::Id(id),
			},
			operations: g.operations.clone(),
		})
		.collect();
	Ok(no_store(Json(json!({ "namespace": jwt.token.namespace, "subject": jwt.token.claims.sub, "grants": grants, "issued_ts": jwt.token.claims.iat * 1000, "expires_ts": jwt.token.claims.exp * 1000 })).into_response()))
}
fn no_store(mut response: Response) -> Response {
	response
		.headers_mut()
		.insert("cache-control", "no-store".parse().unwrap());
	response
}

/// Authorize routes against the namespace the request named and the route's operation. A
/// direct actor ID must exist in that namespace, read through its group, and is then authorized
/// in it. Records name no namespace, so the group the ID was read through is what proves which
/// namespace it is in. Credential management is reserved for presented persistent credentials.
pub async fn authorize(
	ctx: &EngineContext,
	credential: &Credential,
	method: &str,
	uri: &axum::http::Uri,
) -> anyhow::Result<()> {
	if !credential.is_namespace_scoped() {
		return Ok(());
	}
	let path = uri.path();
	let query: std::collections::HashMap<String, String> =
		url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes())
			.into_owned()
			.collect();
	let namespace = query
		.get("namespace")
		.cloned()
		.unwrap_or_else(|| pegboard::DEFAULT_NAMESPACE.into());
	// A route outside the map is refused before any read.
	route_access(method, path)?;
	// The id names the resource's copy in the request's namespace, whose group holds its
	// volumes; the same id in another namespace is another resource.
	let target = if path.starts_with("/actors/") && path != "/actors/names" {
		let id = Id::parse(path["/actors/".len()..].split('/').next().unwrap_or(""))?;
		let standing = pegboard::catalog::namespace_standing(ctx, &namespace).await?;
		let (_group, read) = super::routes::resolve_actor(ctx, &standing, id).await?;
		let found = read
			.and_then(|read| read.data)
			.is_some_and(|data| super::routes::belongs(&data, &standing));
		Some(existing_target(found, id)?)
	} else {
		None
	};
	authorize_access(
		credential,
		ctx.env().clock().observe()?.unix_ms,
		&namespace,
		method,
		path,
		target,
	)
}

/// Checks every grant the route `method` `path` needs on `target`: its own operation, and for a
/// fork also create access for the new actor. The new actor's id is not known before the fork,
/// so only a create grant for every actor in the namespace allows it.
pub(crate) fn authorize_access(
	credential: &Credential,
	now_unix_ms: i64,
	namespace: &str,
	method: &str,
	path: &str,
	target: Option<Id>,
) -> anyhow::Result<()> {
	let (resource, operation) = route_access(method, path)?;
	credential.authorize(now_unix_ms, namespace, resource, target, operation)?;
	if is_fork(method, path) {
		credential.authorize(
			now_unix_ms,
			namespace,
			ResourceKind::Actor,
			None,
			OperationKind::Create,
		)?;
	}
	Ok(())
}

fn is_fork(method: &str, path: &str) -> bool {
	method == "POST" && path.starts_with("/actors/") && path.ends_with("/fork")
}

/// The target of a direct actor route, refused like a missing grant when the request namespace
/// holds no such actor. A foreign record in the shared unprefixed key space counts as missing, so
/// a token cannot probe for IDs.
fn existing_target(found: bool, id: Id) -> anyhow::Result<Id> {
	if !found {
		return Err(auth::InsufficientPermissions.build());
	}
	Ok(id)
}

/// The resource and operation a route needs a grant for. Routes outside this map stay
/// administrative.
pub(crate) fn route_access(
	method: &str,
	path: &str,
) -> anyhow::Result<(ResourceKind, OperationKind)> {
	Ok(match (method, path) {
		("GET", "/actors") => (ResourceKind::Actor, OperationKind::List),
		("POST" | "PUT", "/actors") => (ResourceKind::Actor, OperationKind::Create),
		("GET", "/actors/names") => (ResourceKind::Actor, OperationKind::List),
		("GET", "/envoys" | "/nodes") => (ResourceKind::Runner, OperationKind::List),
		("GET", "/pools") => (ResourceKind::RunnerPool, OperationKind::List),
		("GET", "/datacenters") => (ResourceKind::Datacenter, OperationKind::List),
		("GET", p) if p.starts_with("/pools/") && p.ends_with("/nodes") => {
			(ResourceKind::Runner, OperationKind::List)
		}
		(_, p) if p.starts_with("/pools/") => (
			ResourceKind::RunnerPool,
			match method {
				"GET" => OperationKind::Read,
				"PUT" => OperationKind::Update,
				"DELETE" => OperationKind::Delete,
				_ => return Err(auth::InsufficientPermissions.build()),
			},
		),
		// A fork copies the source the path names into a new actor, so it needs fork access to
		// the source; `authorize_access` also requires create access for the new actor.
		(method, p) if is_fork(method, p) => (ResourceKind::Actor, OperationKind::Fork),
		(_, p) if p.starts_with("/actors/") => (
			ResourceKind::Actor,
			match method {
				"GET" => OperationKind::Read,
				"DELETE" => OperationKind::Delete,
				"POST" => OperationKind::Update,
				_ => return Err(auth::InsufficientPermissions.build()),
			},
		),
		// Namespace catalog management and issuance remain administrative.
		_ => return Err(auth::InsufficientPermissions.build()),
	})
}

#[derive(Debug, rivet_error::RivetError, Serialize, Deserialize)]
#[error("validation", "invalid_input", "Invalid input.", "{message}")]
struct InvalidInput {
	message: String,
}

fn validate_request(body: &CreateRequest, max_duration: u64) -> anyhow::Result<()> {
	if body
		.subject
		.as_ref()
		.is_some_and(|subject| subject.len() > rivet_auth_jwt::MAX_SUBJECT_BYTES)
	{
		return Err(InvalidInput {
			message: "subject exceeds 256 UTF-8 bytes".into(),
		}
		.build());
	}
	if body
		.duration
		.is_some_and(|duration| duration == 0 || duration > max_duration)
	{
		return Err(InvalidInput {
			message: format!("duration must be between 1 and {max_duration} seconds"),
		}
		.build());
	}
	Ok(())
}
