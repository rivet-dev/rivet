//! Admin-issued namespace JWTs. Claims are stateless; the coordinator owns the signing ring.
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
	// Issuance requires the actual admin credential, including in insecure deployments.
	let token = auth::transport_token(&headers).ok_or_else(|| auth::InvalidToken.build())?;
	if !auth::is_admin_bearer(ctx.config(), &format!("Bearer {token}")) {
		return Err(auth::InvalidToken.build().into());
	}
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
	let (token, issued_ts, expires_ts) = ctx
		.jwt()
		.issue(
			body.namespace,
			grants,
			body.subject,
			body.duration
				.unwrap_or_else(|| jwt.default_duration().as_secs()),
		)
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

/// Authorize routes against their namespace and operation. Direct actor IDs resolve the actual
/// persisted namespace before authorization; a query parameter never changes an actor's scope.
pub async fn authorize(
	ctx: &EngineContext,
	credential: &Credential,
	method: &str,
	uri: &axum::http::Uri,
) -> anyhow::Result<()> {
	if matches!(credential, Credential::Admin) {
		return Ok(());
	}
	let path = uri.path();
	let query: std::collections::HashMap<String, String> =
		url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes())
			.into_owned()
			.collect();
	let mut namespace = query
		.get("namespace")
		.cloned()
		.unwrap_or_else(|| pegboard::DEFAULT_NAMESPACE.into());
	let (resource, operation) = match (method, path) {
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
	};
	let target = if path.starts_with("/actors/") && path != "/actors/names" {
		let id = Id::parse(path["/actors/".len()..].split('/').next().unwrap_or(""))?;
		let actor = super::routes::read_actor(ctx, id)
			.await?
			.ok_or_else(|| auth::InsufficientPermissions.build())?;
		namespace = actor.data.namespace.to_string();
		Some(id)
	} else {
		None
	};
	credential.authorize(
		ctx.env().clock().observe()?.unix_ms,
		&namespace,
		resource,
		target,
		operation,
	)
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


