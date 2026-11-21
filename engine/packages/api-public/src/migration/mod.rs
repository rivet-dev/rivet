//! The `/migration/` API the V3 migrator drives to move one namespace's actors out of V2.
//!
//! Every route is a stateless handler over existing pegboard, gasoline and depot state, so the
//! module does nothing until called. Bodies are raw BARE from `v2-migration-protocol`, pinned by the
//! path: the `/v1` routes move actors and the `/v2` routes delete the ones already moved (see
//! [`parked`]).
//!
//! The protocol per actor is list (or a read of one candidate by id), export, cutover, confirm, and
//! optionally restore:
//! - **Cutover** sets `DestroyTsKey` to the migrator's `cutoverTs`, so V2's gateway stops waking the
//!   actor. It writes no gasoline state.
//! - **Confirm** runs after a delay that outlasts any wake already in flight. It re-checks the
//!   generation, the allocation, the workflow's worker and pending signals, and the deadline. If all
//!   hold it parks the actor in the same transaction: the workflow's wake rows move into a parked
//!   record, so no worker runs it again. If any fails it clears the cutover so V2 keeps serving.
//! - **Restore** writes the wake rows back and clears the cutover.
//!
//! Each route serves only actors in this datacenter. A multi-datacenter namespace is migrated by
//! calling every datacenter's public API.

use anyhow::Result;
use axum::{
	body::Bytes,
	http::header::CONTENT_TYPE,
	response::{IntoResponse, Response},
};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Path, Query},
};
use rivet_auth::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope};
use rivet_util::Id;
use serde::Deserialize;
use universaldb::utils::IsolationLevel::Snapshot;

use crate::{ctx::ApiCtx, errors};

mod candidates;
mod export;
mod facts;
pub mod keys;
mod park;
mod parked;

#[derive(Debug, Deserialize)]
pub struct NamespacePath {
	pub namespace: String,
}

#[derive(Debug, Deserialize)]
pub struct ActorPath {
	pub actor_id: Id,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidatesQuery {
	pub after: Option<String>,
	pub limit: Option<usize>,
	/// Lists only actors with this name, oldest first.
	pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParkedQuery {
	pub after: Option<String>,
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorQuery {
	pub namespace: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportQuery {
	pub namespace: String,
	pub resume: Option<String>,
}

pub async fn candidates(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<NamespacePath>,
	Query(query): Query<CandidatesQuery>,
) -> Response {
	respond(candidates::candidates(ctx, path.namespace, query).await)
}

pub async fn candidate(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ActorQuery>,
) -> Response {
	respond(candidates::candidate(ctx, path.actor_id, query.namespace).await)
}

pub async fn export(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ExportQuery>,
) -> Response {
	match export::export(ctx, path.actor_id, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

pub async fn cutover(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ActorQuery>,
	body: Bytes,
) -> Response {
	respond(park::cutover(ctx, path.actor_id, query.namespace, body).await)
}

pub async fn confirm(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ActorQuery>,
	body: Bytes,
) -> Response {
	respond(park::confirm(ctx, path.actor_id, query.namespace, body).await)
}

pub async fn restore(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ActorQuery>,
	body: Bytes,
) -> Response {
	respond(park::restore(ctx, path.actor_id, query.namespace, body).await)
}

pub async fn parked(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<NamespacePath>,
	Query(query): Query<ParkedQuery>,
) -> Response {
	respond(parked::list(ctx, path.namespace, query).await)
}

pub async fn get_delete_cursor(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<NamespacePath>,
) -> Response {
	respond(parked::get_delete_cursor(ctx, path.namespace).await)
}

pub async fn put_delete_cursor(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<NamespacePath>,
	body: Bytes,
) -> Response {
	respond(parked::put_delete_cursor(ctx, path.namespace, body).await)
}

pub async fn delete(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<ActorQuery>,
	body: Bytes,
) -> Response {
	respond(parked::delete(ctx, path.actor_id, query.namespace, body).await)
}

fn respond(result: Result<Vec<u8>>) -> Response {
	match result {
		Ok(body) => (
			[(CONTENT_TYPE, v2_migration_protocol::BARE_CONTENT_TYPE)],
			body,
		)
			.into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

/// Requires a token that may read and update every actor and every actor KV in the namespace, then
/// resolves the namespace id.
///
/// The routes read all of a namespace's actor data and park its actors, so a token scoped to some
/// actors is refused. Use the admin token or a token scoped to the whole namespace.
async fn authorize(ctx: &ApiCtx, namespace: &str) -> Result<Id> {
	for (resource, operation) in [
		(ResourceKind::Actor, OperationKind::List),
		(ResourceKind::Actor, OperationKind::Update),
		(ResourceKind::ActorKv, OperationKind::Read),
	] {
		ctx.auth(
			AccessNamespaceScope::Name(namespace.to_string()),
			resource,
			TargetScope::Any,
			operation,
		)
		.await?;
	}

	ensure_backfill_complete(ctx).await?;

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: namespace.to_string(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	Ok(namespace.namespace_id)
}

/// Refuses every route until `actor_migration_fix_backfill` has finished.
///
/// That backfill treats any actor with `DestroyTsKey` as destroyed. Run over a cut-over or parked
/// actor, it would delete the actor's active index entry and mark its key destroyed, and a restore
/// undoes neither. Bootstrap starts the backfill on every engine start until it completes and never
/// runs it again afterward, so once it has finished no later run can touch a parked actor.
async fn ensure_backfill_complete(ctx: &ApiCtx) -> Result<()> {
	let complete = ctx
		.udb()?
		.txn("migration_check_backfill_complete", |tx| async move {
			let tx = tx.with_subspace(rivet_types::keys::backfill::subspace());
			tx.exists(
				&rivet_types::keys::backfill::CompleteKey::new(
					pegboard::workflows::actor_migration_fix_backfill::BACKFILL_NAME,
				),
				Snapshot,
			)
			.await
		})
		.await?;

	if !complete {
		return Err(errors::Migration::BackfillIncomplete.build());
	}

	Ok(())
}

/// Refuses actors that another datacenter owns. The migrator calls the owning datacenter's API,
/// so these routes never read across datacenters.
fn ensure_local(ctx: &ApiCtx, actor_id: Id) -> Result<()> {
	let local = ctx.config().dc_label();
	if actor_id.label() != local {
		return Err(errors::Migration::WrongDatacenter {
			actor_datacenter_label: actor_id.label(),
			datacenter_label: local,
		}
		.build());
	}

	Ok(())
}

/// Refuses actors outside the token's namespace with the same error as a missing actor, so a
/// token cannot probe other namespaces' actor ids.
fn ensure_namespace(facts: Option<&facts::ActorFacts>, namespace_id: Id) -> Result<()> {
	match facts {
		Some(facts) if facts.namespace_id == namespace_id => Ok(()),
		_ => Err(errors::Migration::ActorNotFound.build()),
	}
}

fn decode_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T> {
	v2_migration_protocol::decode(body).map_err(|err| {
		errors::Migration::InvalidRequest {
			message: format!("{err:#}"),
		}
		.build()
	})
}
