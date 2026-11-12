use pegboard::NamespaceStanding;
use pegboard::legacy_volumes::legacy_resolve_actor;
use rivet_keys::keys::{VolumeKey, codec::GroupId};
use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result, bail, ensure};
use axum::{
	Extension, Json,
	extract::{Path, Query, State},
	http::{HeaderMap, Method, StatusCode},
	response::{IntoResponse, Response},
};
use base64::Engine;
use bytes::Bytes;
use futures_util::StreamExt;
use pegboard::{
	commands::{ActorCreate, ActorEvent, ActorRead, is_volume_gone},
	errors::ActorMigratingFromV2,
	keys::partition::ActorVolume,
	namespace_wire, wire,
};
use pegboard_gateway::migrate_from_v2::{self, V3Actor, decision, proxy};
use rand::RngCore;
use rivet_config::config::MigrateFromV2;
use rivet_engine_context::EngineContext;
use rivet_guard_core::auth;
use rivet_util::{Id, rng::RngStreamIdExt};
use rivetkit_shared_types::actor_key;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

use crate::ApiError;

/// How long a create waits for the actor's data record, across retries.
const CREATE_TIMEOUT: Duration = Duration::from_secs(30);

/// How long before an actor fork's deadline its last read of the forked actor starts, so the
/// read answers before the deadline does.
const FORK_LAST_READ_MARGIN: Duration = Duration::from_secs(2);

/// Entries per actor listing page.
const LIST_PAGE_SIZE: usize = 100;

/// The single datacenter v3 serves, as `/datacenters` names it.
const DATACENTER_NAME: &str = "default";

/// Active index pages one node listing reads at most. Each scans up to 100 entries.
const NODE_INDEX_PAGES_PER_REQUEST: usize = 4;

/// Actor records an actor listing reads at once.
const ACTOR_READ_CONCURRENCY: usize = 16;

/// Node records a listing reads at once.
const NODE_READ_CONCURRENCY: usize = 16;

/// How often a create reads the actor while waiting for its data record.
const CREATE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Retry only a refused storage step at the HTTP edge, within the request's shared budget.
/// The caller retains the actor id and create body; a successful mutation is never replayed
/// to recover a subsequent read.
async fn create_step<T, F, Fut>(
	ctx: &EngineContext,
	retries: &mut pegboard_gateway::Retries<'_>,
	operation: &'static str,
	mut step: F,
) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	loop {
		match step().await {
			Ok(value) => return Ok(value),
			Err(error) => {
				retries.retry(ctx.env().as_ref(), error).await?;
				tracing::debug!(operation, "retrying refused actor API storage step");
			}
		}
	}
}

/// Whether this HTTP request may already have submitted its actor's create.
#[derive(Clone, Copy, Debug)]
enum CreateProgress {
	/// No actor create has been sent; a refusal is safe to return with its retry hint.
	NotSubmitted,
	/// A create may have been sent. An uncertain outcome may already have created this actor.
	Submitted(Id),
	/// A create was acknowledged. Reading its result must not invite another logical create.
	Acknowledged(Id),
}

#[derive(Debug, rivet_error::RivetError, serde::Serialize)]
#[error(
	"actor",
	"create_pending",
	"Actor creation may still be in progress; do not retry creation automatically.",
	"Actor creation may still be in progress; inspect actor {actor_id} instead of retrying creation."
)]
struct ActorCreatePending {
	actor_id: String,
}

/// A known or uncertain create cannot be turned into a fresh user-request retry by a
/// subsequent storage failure. Preserve its id without claiming that it already has data.
fn create_error(progress: CreateProgress, error: anyhow::Error) -> anyhow::Error {
	let uncertain = error
		.downcast_ref::<rivet_error::RivetError>()
		.map_or(true, |error| {
			error.group() == "depot"
				&& matches!(
					error.code(),
					"unavailable" | "request_deadline_exceeded" | "scope_revoked"
				)
		});
	let actor_id = match progress {
		CreateProgress::Acknowledged(actor_id)
			if uncertain || depot_client::retry_after_ms(&error).is_some() =>
		{
			Some(actor_id)
		}
		CreateProgress::Submitted(actor_id) if uncertain => Some(actor_id),
		_ => None,
	};
	match actor_id {
		Some(actor_id) => ActorCreatePending {
			actor_id: actor_id.to_string(),
		}
		.build(),
		None => error,
	}
}

/// Preserve local submission progress when a request deadline cancels a storage step.
fn create_result<T>(
	progress: CreateProgress,
	result: Result<Result<T>, rivet_env::TimeoutError>,
) -> Result<T> {
	match result {
		Ok(result) => result,
		Err(rivet_env::TimeoutError::Elapsed) => Err(depot_client::RequestDeadlineExceeded.build()),
		Err(rivet_env::TimeoutError::Clock(error)) => Err(error.into()),
	}
	.map_err(|error| create_error(progress, error))
}

/// Only refusals left before an actor create may be returned as retryable overloads.
fn create_response(result: Result<Response>) -> Response {
	match result {
		Ok(response) => response,
		Err(error) => {
			let refused = depot_client::retry_after_ms(&error).is_some();
			let deadline_elapsed =
				error
					.downcast_ref::<rivet_error::RivetError>()
					.is_some_and(|error| {
						error.group() == "depot" && error.code() == "request_deadline_exceeded"
					});
			let mut response = ApiError::from(error).into_response();
			if refused {
				*response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
			} else if deadline_elapsed {
				*response.status_mut() = StatusCode::GATEWAY_TIMEOUT;
			}
			response
		}
	}
}

/// Serves without authentication so clients can discover the engine before they authenticate.
/// The actor names read every active envoy's record, so only a caller with the admin token gets
/// them.
pub async fn metadata(
	State(ctx): State<EngineContext>,
	headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
	let mut body = json!({
		"runtime": "engine",
		"version": env!("CARGO_PKG_VERSION"),
		"envoyProtocolVersion": rivet_envoy_protocol::PROTOCOL_VERSION,
	});
	let authenticated =
		auth::admit_admin_or_insecure(ctx.config(), auth::transport_token(&headers)).is_ok();
	if authenticated {
		body["actorNames"] = actor_names(&ctx, pegboard::DEFAULT_NAMESPACE).await?;
	}
	Ok(Json(body))
}

pub async fn list_datacenters() -> Result<Json<Value>, ApiError> {
	Ok(Json(
		json!({ "datacenters": [{ "name": DATACENTER_NAME }] }),
	))
}

/// Lists a bounded page of configured node pools, ordered by name.
pub async fn list_pools(
	State(ctx): State<EngineContext>,
	Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let limit = parse_limit(query.limit.as_deref())?;
	if let Some(cursor) = &query.cursor {
		pegboard::validation::pool_name(cursor)?;
	}
	let reply = ctx
		.depot()
		.command::<pegboard::catalog::PoolList>(
			&pegboard::keys::partition::EnvoyPoolIndexVolume::new(namespace.name.clone())
				.volume_key(),
			wire::PoolListRequest {
				after: query.cursor.map(Into::into),
				limit: limit as u32,
			},
		)
		.await?;
	let pools = reply
		.pools
		.iter()
		.map(|entry| pool_json(&entry.name, &entry.config))
		.collect::<Result<Vec<_>>>()?;
	Ok(Json(
		json!({"pools": pools, "pagination": {"cursor": reply.cursor}}),
	))
}

/// Lists the live nodes of one pool, one page at a time, in node key order.
///
/// A node is listed while it is inside the heartbeat window of the pool's active index, the
/// same set allocation picks from. Each listed node's record is read for its creation time, so
/// a page costs at most a few index pages plus one read per node. Allocation never uses this
/// listing.
pub async fn list_pool_nodes(
	State(ctx): State<EngineContext>,
	Path(pool): Path<String>,
	Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	pegboard::validation::pool_name(&pool)?;
	let limit = parse_limit(query.limit.as_deref())?;
	let mut after = query.cursor.as_deref().map(parse_node_cursor).transpose()?;

	// Collect up to `limit` live nodes. Expired entries are skipped inside the index, so a page
	// can come back short; a bounded number of further pages fills it.
	let mut states = Vec::new();
	for _ in 0..NODE_INDEX_PAGES_PER_REQUEST {
		let page = pegboard::index::list_active_envoy_states_page(
			&ctx,
			&namespace.name,
			&pool,
			after.as_deref(),
		)
		.await?;
		states.extend(page.envoys);
		after = page.after;
		if states.len() >= limit {
			states.truncate(limit);
			after = states.last().map(|state| state.envoy_key.to_string());
			break;
		}
		if after.is_none() {
			break;
		}
	}

	let keys = states
		.iter()
		.map(|state| state.envoy_key.to_string())
		.collect::<Vec<_>>();
	let (ctx_ref, namespace_ref, pool_ref) = (&ctx, namespace.name.as_str(), pool.as_str());
	let records = futures_util::stream::iter(keys)
		.map(|key| async move {
			pegboard::index::read_envoy(ctx_ref, namespace_ref, pool_ref, &key).await
		})
		.buffered(NODE_READ_CONCURRENCY)
		.collect::<Vec<_>>()
		.await;
	let mut nodes = Vec::with_capacity(states.len());
	for (state, record) in states.iter().zip(records) {
		// A node whose record is gone disconnected after the index page was read.
		if let Some(record) = record? {
			nodes.push(node_json(&namespace.name, &pool, state, &record));
		}
	}

	Ok(Json(json!({
		"nodes": nodes,
		"pagination": { "cursor": after.map(|key| format_node_cursor(&key)) },
	})))
}

/// One node of a pool listing, with main's runner fields under node names.
fn node_json(
	namespace: &str,
	pool: &str,
	state: &wire::ActiveEnvoyState,
	record: &wire::EnvoyData,
) -> Value {
	json!({
		"key": state.envoy_key.to_string(),
		"pool": pool,
		"namespace": namespace,
		"datacenter": DATACENTER_NAME,
		"version": record.version,
		"create_ts": record.create_unix_ms,
		"last_ping_ts": state.heartbeat_unix_ms,
		"connected": record.connected,
		"stopping": record.stopping,
	})
}

/// Encodes the node key a listing resumes after. Node keys are arbitrary strings, so the cursor
/// is their URL-safe base64.
fn format_node_cursor(envoy_key: &str) -> String {
	base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(envoy_key)
}

fn parse_node_cursor(cursor: &str) -> Result<String> {
	let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
		.decode(cursor)
		.map_err(|_| InvalidCursor.build())?;
	String::from_utf8(bytes).map_err(|_| InvalidCursor.build())
}

pub async fn list_actor_names(
	State(ctx): State<EngineContext>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	Ok(Json(
		json!({ "names": actor_names(&ctx, &namespace.name).await? }),
	))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNamespaceRequest {
	name: String,
}

/// How long a create waits for the namespace to leave `Creating` or `Forking` before answering
/// with it as it stands.
const START_WAIT: Duration = Duration::from_secs(30);
/// How long a request waits for its namespace's group create to settle before it is refused
/// with the retryable `namespace.creating`. A namespace is creating for moments after its
/// create, and after the move of its legacy catalog row, which its first lookup after an upgrade
/// from 3.0 makes. The bound stays well under the 30 seconds a released node host gives its
/// startup pool upsert, retries included, so a host refused once still has time to retry.
const NAMESPACE_SETTLE_WAIT: Duration = Duration::from_secs(5);
/// Namespace states a listing page reads at once.
const LIST_READ_CONCURRENCY: usize = 16;

pub async fn create_namespace(
	State(ctx): State<EngineContext>,
	Extension(waits): Extension<Arc<pegboard::namespace_waits::StartWaits>>,
	Json(body): Json<CreateNamespaceRequest>,
) -> Result<Json<Value>, ApiError> {
	pegboard::validation::namespace_name(&body.name)?;
	let state = start_namespace(&ctx, &waits, &body.name, None).await?;
	Ok(Json(
		json!({ "namespace": namespace_json(&body.name, &state)? }),
	))
}

/// Creates namespace `name`, a fork of `parent` when one is given, and answers with its state
/// once it is live, or as it stands after `START_WAIT`.
async fn start_namespace(
	ctx: &EngineContext,
	waits: &Arc<pegboard::namespace_waits::StartWaits>,
	name: &str,
	parent: Option<namespace_wire::NamespaceParent>,
) -> Result<namespace_wire::NamespaceState, ApiError> {
	let create_unix_ms = ctx.env().clock().observe()?.unix_ms;

	// Until every worker reads namespace volumes, a create writes a legacy catalog row, which
	// older binaries can read, and a fork waits for the fleet.
	if !pegboard::catalog::namespace_volumes_open(ctx)? {
		if parent.is_some() {
			return Err(pegboard::validation::NamespacePending.build().into());
		}
		let state = pegboard::catalog::legacy_create_namespace(ctx, name, create_unix_ms)
			.await?
			.ok_or_else(|| pegboard::validation::NamespaceNameNotUnique.build())?;
		return Ok(state);
	}

	// A name that lives only as a legacy row is taken; it moves to its volume, where the create
	// below finds it. The create's get-or-create of the volume answers every other taken name.
	pegboard::catalog::legacy_move_before_create(ctx, name).await?;

	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), START_WAIT)?;
	let expires_unix_ms = create_unix_ms.saturating_add(i64::try_from(START_WAIT.as_millis())?);
	// Every create waits: a root for its group record and a fork for its group writes, so the
	// call answers once the namespace is `live`.
	let mut wait = waits.join(name);
	let reply = ctx
		.depot()
		.command::<pegboard::catalog::NamespaceCreate>(
			&pegboard::keys::partition::NamespaceVolume::new(name.to_owned()).volume_key(),
			namespace_wire::NamespaceCreateRequest {
				create_unix_ms,
				fork_from: parent,
				waiter: wait.as_ref().map(|wait| wait.waiter(expires_unix_ms)),
			},
		)
		.await?;
	if !reply.created {
		// A name whose fork failed is retired for good, which the caller is told apart from a
		// name another create holds.
		let refusal = match &reply.state.phase {
			namespace_wire::NamespacePhase::NamespaceFailed(_) => {
				pegboard::validation::NamespaceFailed.build()
			}
			namespace_wire::NamespacePhase::NamespaceCreating(_)
			| namespace_wire::NamespacePhase::NamespaceForking(_)
			| namespace_wire::NamespacePhase::NamespaceLive
			| namespace_wire::NamespacePhase::NamespaceArchiving(_)
			| namespace_wire::NamespacePhase::NamespaceArchived(_) => {
				pegboard::validation::NamespaceNameNotUnique.build()
			}
		};
		return Err(refusal.into());
	}
	pegboard::catalog::remember(ctx, name, reply.state.clone()).await?;

	let mut state = reply.state;
	if starting(&state) {
		state = wait_for_start(ctx, name, wait.as_mut(), deadline).await?;
	}
	Ok(state)
}

/// Waits for the start notice of a namespace being created or forked until `deadline`, then
/// reads it once if no notice came, since a notice is lossy. Without a start wait (the waits
/// are full or there is no UPS) nothing can notice the start, so it reads at once and, if the
/// namespace is still starting, once more at `deadline`: two reads, never a poll.
async fn wait_for_start(
	ctx: &EngineContext,
	name: &str,
	wait: Option<&mut pegboard::namespace_waits::Wait>,
	deadline: u64,
) -> Result<namespace_wire::NamespaceState> {
	let clock = ctx.env().clock();
	let noticed = match wait {
		Some(wait) => rivet_env::timeout_at(clock.as_ref(), deadline, wait.notified())
			.await
			.ok(),
		None => {
			let state = read_starting_namespace(ctx, name).await?;
			if starting(&state) {
				clock.sleep_until(deadline).await?;
				None
			} else {
				Some(state)
			}
		}
	};
	let state = match noticed {
		Some(state) => state,
		None => read_starting_namespace(ctx, name).await?,
	};
	pegboard::catalog::remember(ctx, name, state.clone()).await?;
	Ok(state)
}

/// Reads a namespace this create made, which keeps its state while it starts.
async fn read_starting_namespace(
	ctx: &EngineContext,
	name: &str,
) -> Result<namespace_wire::NamespaceState> {
	pegboard::catalog::lookup_namespace(ctx, name)
		.await?
		.context("a namespace being created or forked keeps its state")
}

/// Whether a namespace has not finished its group create or fork yet.
fn starting(state: &namespace_wire::NamespaceState) -> bool {
	match state.phase {
		namespace_wire::NamespacePhase::NamespaceCreating(_)
		| namespace_wire::NamespacePhase::NamespaceForking(_) => true,
		namespace_wire::NamespacePhase::NamespaceLive
		| namespace_wire::NamespacePhase::NamespaceArchiving(_)
		| namespace_wire::NamespacePhase::NamespaceArchived(_)
		| namespace_wire::NamespacePhase::NamespaceFailed(_) => false,
	}
}

/// Archives a namespace: refused while a fork of it is live or an envoy of it is connected;
/// otherwise it refuses new work for the namespace and archives its group. Answers at once with
/// the namespace as it stands; the archive finishes on its own.
pub async fn archive_namespace(
	State(ctx): State<EngineContext>,
	Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
	pegboard::validation::namespace_name(&name)?;
	if name == pegboard::DEFAULT_NAMESPACE {
		return Err(pegboard::validation::NamespaceProtected.build().into());
	}
	let state = pegboard::catalog::lookup_namespace(&ctx, &name)
		.await?
		.ok_or_else(|| pegboard::validation::NamespaceNotFound.build())?;
	// Until every worker reads namespace volumes, a namespace may live only as a legacy row with
	// no volume to archive, and the fleet's view must be known before an archive is sent. This
	// check reads memory, so it runs before the fork and envoy reads.
	if !matches!(pegboard::catalog::namespace_volumes_open(&ctx), Ok(true)) {
		return Err(pegboard::validation::NamespacePending.build().into());
	}
	// Legacy: removed in 3.1 with the backfill; see `NamespaceBackfillPending`.
	match &state.backfill {
		namespace_wire::NamespaceBackfill::NamespaceBackfillDone(done)
			if done.skipped_count > 0 =>
		{
			let skipped_count = done.skipped_count;
			return Err(
				pegboard::validation::NamespaceBackfillSkipped { skipped_count }
					.build()
					.into(),
			);
		}
		namespace_wire::NamespaceBackfill::NamespaceBackfillDone(_) => {}
		namespace_wire::NamespaceBackfill::NamespaceBackfillPending(_)
		| namespace_wire::NamespaceBackfill::NamespaceBackfillRunning(_) => {
			return Err(pegboard::validation::NamespaceBackfillPending
				.build()
				.into());
		}
	}
	// One read of the group record answers, however many forks the namespace ever had.
	if let Some(group) = &state.group
		&& matches!(state.phase, namespace_wire::NamespacePhase::NamespaceLive)
		&& ctx
			.depot()
			.read_group(group.group)
			.await?
			.as_ref()
			.is_some_and(depot_client::groups::has_live_forks)
	{
		return Err(pegboard::validation::NamespaceHasForks.build().into());
	}
	// Archiving stops nothing an envoy hosts, so the caller stops its nodes first. An envoy that
	// connects after this check gets `group.archived` on its actors' next open.
	if matches!(state.phase, namespace_wire::NamespacePhase::NamespaceLive)
		&& pegboard::catalog::namespace_has_connected_envoys(&ctx, &name).await?
	{
		return Err(pegboard::validation::NamespaceHasNodes.build().into());
	}
	let reply = ctx
		.depot()
		.command::<pegboard::catalog::NamespaceArchive>(
			&pegboard::keys::partition::NamespaceVolume::new(name.clone()).volume_key(),
			(),
		)
		.await?;
	pegboard::catalog::remember(&ctx, &name, reply.state.clone()).await?;
	Ok(Json(
		json!({ "namespace": namespace_json(&name, &reply.state)? }),
	))
}

/// A namespace in its public shape: its status and, for a fork, what it was forked from.
fn namespace_json(name: &str, state: &namespace_wire::NamespaceState) -> Result<Value> {
	let (status, archived_ts) = match &state.phase {
		namespace_wire::NamespacePhase::NamespaceLive => ("live", None),
		namespace_wire::NamespacePhase::NamespaceCreating(_) => ("creating", None),
		namespace_wire::NamespacePhase::NamespaceForking(_) => ("forking", None),
		namespace_wire::NamespacePhase::NamespaceArchiving(_) => ("archiving", None),
		namespace_wire::NamespacePhase::NamespaceArchived(archived) => {
			("archived", Some(archived.archived_unix_ms))
		}
		namespace_wire::NamespacePhase::NamespaceFailed(_) => ("failed", None),
	};
	let forked_from = state
		.group
		.as_ref()
		.and_then(|group| group.base.as_ref())
		.map(|base| {
			json!({
				"namespace": base.parent_namespace,
				"fork_number": base.fork_number,
				"forked_ts": base.forked_unix_ms,
			})
		});
	let (skipped, skipped_count) = legacy_skipped_json(&state.backfill)?;
	Ok(json!({
		"name": name,
		"create_ts": state.create_unix_ms,
		"status": status,
		"forked_from": forked_from,
		"archived_ts": archived_ts,
		"skipped": skipped,
		"skipped_count": skipped_count,
	}))
}

/// Legacy: the actors the namespace's finished backfill gave up on, listed by id, name and key
/// in the slash form the actor API reads (up to the list's cap), and how many it still counts, so an operator can find and destroy them
/// without the error log. Empty and zero before the backfill is done and for a namespace that
/// never had one. Removed in 3.1 with the backfill.
fn legacy_skipped_json(backfill: &namespace_wire::NamespaceBackfill) -> Result<(Vec<Value>, u32)> {
	match backfill {
		namespace_wire::NamespaceBackfill::NamespaceBackfillDone(done) => {
			let skipped = done
				.skipped
				.iter()
				.map(|skipped| {
					let actor_id = Id::from_slice(&skipped.actor_id)
						.context("a skipped backfill entry has an invalid actor id")?;
					anyhow::Ok(json!({
						"actor_id": actor_id.to_string(),
						"name": skipped.name.to_string(),
						"key": skipped.key.as_deref().map(actor_key::to_slash),
					}))
				})
				.collect::<Result<Vec<_>>>()?;
			Ok((skipped, done.skipped_count))
		}
		namespace_wire::NamespaceBackfill::NamespaceBackfillPending(_)
		| namespace_wire::NamespaceBackfill::NamespaceBackfillRunning(_) => Ok((Vec::new(), 0)),
	}
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListNamespacesQuery {
	name: Option<String>,
	cursor: Option<String>,
	limit: Option<u32>,
}

pub async fn list_namespaces(
	State(ctx): State<EngineContext>,
	Query(query): Query<ListNamespacesQuery>,
) -> Result<Json<Value>, ApiError> {
	if let Some(name) = query.name {
		pegboard::validation::namespace_name(&name)?;
		let namespaces = pegboard::catalog::lookup_namespace(&ctx, &name)
			.await?
			.map(|state| namespace_json(&name, &state))
			.transpose()?
			.into_iter()
			.collect::<Vec<_>>();
		return Ok(Json(
			json!({"namespaces": namespaces,"pagination":{"cursor":null}}),
		));
	}
	if query.limit == Some(0) || query.limit.is_some_and(|limit| limit > 100) {
		return Err(InvalidNamespaceList.build().into());
	}
	if let Some(cursor) = &query.cursor {
		pegboard::validation::namespace_name(cursor)?;
	}

	// Legacy: while a storage worker from before namespace volumes lives, it may host the
	// catalog and knows only the record-row listing, and every namespace is a legacy row. An
	// unknown fleet view surfaces as it does for a create. Removed in 3.1.
	if !pegboard::catalog::namespace_volumes_open(&ctx)? {
		let (namespaces, cursor) = pegboard::catalog::legacy_list_namespaces(
			&ctx,
			query.cursor,
			query.limit.unwrap_or(100),
		)
		.await?;
		let namespaces = namespaces
			.iter()
			.map(|(name, state)| namespace_json(name, state))
			.collect::<Result<Vec<_>>>()?;
		return Ok(Json(
			json!({"namespaces":namespaces,"pagination":{"cursor":cursor}}),
		));
	}

	let page = ctx
		.depot()
		.command::<pegboard::catalog::NamespaceIndexList>(
			&pegboard::keys::partition::NamespaceCatalogVolume::new().volume_key(),
			namespace_wire::NamespaceIndexListRequest {
				after: query.cursor.map(Into::into),
				limit: query.limit.unwrap_or(100),
				legacy_index_only: false,
			},
		)
		.await?;

	// Each entry's state comes from its volume through the process cache. A name in the index
	// whose state is gone cannot happen, since names are never deleted; one that fails to read
	// fails the page so the caller retries.
	let states = futures_util::stream::iter(page.names.iter().cloned())
		.map(|name| {
			let ctx = ctx.clone();
			async move {
				let state = pegboard::catalog::namespace_state(&ctx, &name).await?;
				namespace_json(&name, &state)
			}
		})
		.buffered(LIST_READ_CONCURRENCY)
		.collect::<Vec<_>>()
		.await;
	let namespaces = states.into_iter().collect::<Result<Vec<_>>>()?;
	Ok(Json(
		json!({"namespaces":namespaces,"pagination":{"cursor":page.cursor}}),
	))
}

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"namespace",
	"invalid",
	"Namespace list limit must be between 1 and 100."
)]
struct InvalidNamespaceList;

#[derive(Deserialize)]
pub struct CreateActorRequest {
	name: String,
	key: Option<String>,
	/// Base64-encoded actor input.
	input: Option<String>,
	/// The node pool the actor runs in; `default` when absent.
	#[serde(rename = "pool", alias = "runner_name_selector")]
	runner_name_selector: Option<String>,
}

#[derive(Deserialize)]
pub struct NamespaceQuery {
	namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct PageQuery {
	namespace: Option<String>,
	limit: Option<String>,
	cursor: Option<String>,
}

/// The validated namespace of a request that creates, wakes or lists, defaulting to the
/// default namespace; refused while the namespace's status forbids new work.
async fn namespace(ctx: &EngineContext, query: Option<&str>) -> Result<Arc<NamespaceStanding>> {
	let namespace = query.unwrap_or(pegboard::DEFAULT_NAMESPACE);
	let deadline = rivet_env::deadline_after(ctx.env().clock().as_ref(), NAMESPACE_SETTLE_WAIT)?;
	pegboard::catalog::ensure_settled_namespace(ctx, namespace, deadline).await
}

/// The validated namespace of a request that reads or destroys one actor, which keeps working
/// while the namespace archives.
async fn namespace_for_read(
	ctx: &EngineContext,
	query: Option<&str>,
) -> Result<Arc<NamespaceStanding>> {
	let namespace = query.unwrap_or(pegboard::DEFAULT_NAMESPACE);
	let deadline = rivet_env::deadline_after(ctx.env().clock().as_ref(), NAMESPACE_SETTLE_WAIT)?;
	let standing = pegboard::catalog::settled_namespace_standing(ctx, namespace, deadline).await?;
	// Reads and destroys keep working while a namespace archives; a namespace still being
	// created or forked has no group to read yet, and a failed or archived one is closed.
	match &standing.phase {
		pegboard::namespace_wire::NamespacePhase::NamespaceLive
		| pegboard::namespace_wire::NamespacePhase::NamespaceArchiving(_) => Ok(standing),
		pegboard::namespace_wire::NamespacePhase::NamespaceCreating(_)
		| pegboard::namespace_wire::NamespacePhase::NamespaceForking(_)
		| pegboard::namespace_wire::NamespacePhase::NamespaceArchived(_)
		| pegboard::namespace_wire::NamespacePhase::NamespaceFailed(_) => {
			standing.admit()?;
			Ok(standing)
		}
	}
}

pub async fn create_actor(
	State(ctx): State<EngineContext>,
	Extension(budget): Extension<Arc<pegboard_gateway::RetryBudget>>,
	Query(query): Query<NamespaceQuery>,
	headers: HeaderMap,
	raw: Bytes,
) -> Result<Response, ApiError> {
	Ok(create_response(
		create_or_proxy(&ctx, &budget, query, &headers, raw, false).await,
	))
}

pub async fn get_or_create_actor(
	State(ctx): State<EngineContext>,
	Extension(budget): Extension<Arc<pegboard_gateway::RetryBudget>>,
	Query(query): Query<NamespaceQuery>,
	headers: HeaderMap,
	raw: Bytes,
) -> Result<Response, ApiError> {
	Ok(create_response(
		create_or_proxy(&ctx, &budget, query, &headers, raw, true).await,
	))
}

/// Creates the actor here, or proxies the create to V2's public API while a V2 namespace
/// migrates in. A keyless create draws against the share. A keyed create is sent here, and its
/// state machine settles whether the key belongs here or to V2; one it places with V2 goes there.
/// A key being moved in is refused as the retryable `actor.migrating_from_v2`. A create that
/// sends `x-rivet-actor-version: 3` is placed here unless its key already belongs to V2.
async fn create_or_proxy(
	ctx: &EngineContext,
	budget: &pegboard_gateway::RetryBudget,
	query: NamespaceQuery,
	headers: &HeaderMap,
	raw: Bytes,
	get_or_create: bool,
) -> Result<Response> {
	let body = match Json::<CreateActorRequest>::from_bytes(&raw) {
		Ok(Json(body)) => body,
		Err(rejection) => return Ok(rejection.into_response()),
	};
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), CREATE_TIMEOUT)?;
	budget.deposit();
	let mut retries = budget.retries(deadline);
	let namespace = {
		let ctx = &ctx.with_depot_command_deadline(deadline);
		create_result(
			CreateProgress::NotSubmitted,
			rivet_env::timeout_at(
				clock.as_ref(),
				deadline,
				create_step(ctx, &mut retries, "namespace", || {
					namespace(ctx, query.namespace.as_deref())
				}),
			)
			.await,
		)?
	};
	let dynamic = migrate_from_v2::snapshot(ctx);
	let migration = migrate_from_v2::applies(dynamic.as_deref(), &namespace.name);
	let force_actor_v3 = match migration {
		Some(_) => migrate_from_v2::forces_actor_v3(headers)?,
		None => false,
	};

	if let Some(config) = migration
		&& body.key.is_none()
	{
		let draw =
			(ctx.env().rng("migrate_from_v2.create").next_u64() >> 11) as f64 / (1u64 << 53) as f64;
		let create_ratio = migrate_from_v2::create_ratio(config, headers)?;
		if decision::keyless_create(draw, create_ratio) == decision::Route::V2 {
			return proxy_create(ctx, config, get_or_create, raw).await;
		}
	}

	let mut progress = CreateProgress::NotSubmitted;
	// The deadline covers the local storage work. Forwarding retains its existing timeout.
	let result = {
		let ctx = &ctx.with_depot_command_deadline(deadline);
		rivet_env::timeout_at(
			clock.as_ref(),
			deadline,
			create_actor_inner(
				ctx,
				query,
				body,
				get_or_create,
				force_actor_v3,
				&mut retries,
				&mut progress,
			),
		)
		.await
	};
	match create_result(progress, result)? {
		Created::Actor(json) => Ok(json.into_response()),
		Created::DelegatedToV2 => match migration {
			Some(config) => proxy_create(ctx, config, get_or_create, raw).await,
			// A key is only delegated to V2 while its namespace migrates in, so a create that
			// raced the migration's end is refused and its retry creates the actor here.
			None => Err(ActorMigratingFromV2::error()),
		},
	}
}

/// Sends the create to V2's public API in the migrated V2 namespace.
async fn proxy_create(
	ctx: &EngineContext,
	config: &MigrateFromV2,
	get_or_create: bool,
	raw: Bytes,
) -> Result<Response> {
	let method = if get_or_create {
		Method::PUT
	} else {
		Method::POST
	};
	let path = format!("/actors?{}", v2_namespace_query(config)?);
	let response = pegboard::migrate_from_v2::api(ctx, config, method, &path, Some(raw)).await?;
	Ok(proxy::api_response(response))
}

/// The V2 public API query naming the migrated V2 namespace.
fn v2_namespace_query(config: &MigrateFromV2) -> Result<String> {
	Ok(serde_html_form::to_string([(
		"namespace",
		config.namespace.as_str(),
	)])?)
}

/// Lists live actors in the migrated V2 namespace through V2's public API, filtered by
/// `params`. They keep V2's actor shape.
async fn v2_actors(
	ctx: &EngineContext,
	config: &MigrateFromV2,
	params: &[(&str, &str)],
) -> Result<Vec<Value>> {
	let mut query = vec![("namespace", config.namespace.as_str())];
	query.extend_from_slice(params);
	let path = format!("/actors?{}", serde_html_form::to_string(&query)?);
	let response = pegboard::migrate_from_v2::api(ctx, config, Method::GET, &path, None).await?;
	ensure!(
		response.status.is_success(),
		"V2 actor listing failed with status {}",
		response.status
	);
	let mut listed: Value =
		serde_json::from_slice(&response.body).context("invalid V2 actor listing")?;
	match listed.get_mut("actors").map(Value::take) {
		Some(Value::Array(actors)) => Ok(actors),
		Some(_) | None => bail!("V2 actor listing has no actors array"),
	}
}

/// What a create here came to.
enum Created {
	Actor(Json<Value>),
	/// The key belongs to V2 while its namespace migrates in, so V2 serves the create.
	DelegatedToV2,
}

/// Creates the actor and waits for its data record. `force_actor_v3` asks for the actor here while
/// its namespace migrates in from V2.
///
/// A keyed actor's id is derived from its key, so a getOrCreate of an existing actor reads it
/// and a create of a missing one converges with any concurrent create. The create is a
/// `TryStart`, so a getOrCreate also wakes a sleeping actor. A key owned by another datacenter
/// fails, since forwarding across datacenters is not built yet.
async fn create_actor_inner(
	ctx: &EngineContext,
	query: NamespaceQuery,
	mut body: CreateActorRequest,
	get_or_create: bool,
	force_actor_v3: bool,
	retries: &mut pegboard_gateway::Retries<'_>,
	progress: &mut CreateProgress,
) -> Result<Created> {
	let namespace = create_step(ctx, retries, "namespace", || {
		namespace(ctx, query.namespace.as_deref())
	})
	.await?;
	// The API speaks the slash key form; the key is hashed and stored in its canonical form.
	body.key = body.key.as_deref().map(canonical_key).transpose()?;
	let pool_name = body
		.runner_name_selector
		.clone()
		.unwrap_or_else(|| pegboard::DEFAULT_POOL.to_owned());
	pegboard::validation::actor_name(&body.name)?;
	pegboard::validation::actor_key(body.key.as_deref())?;
	pegboard::validation::pool_name(&pool_name)?;
	let input = decode_input(body.input.clone())?;

	let clock = ctx.env().clock();

	let Some(key) = body.key.clone() else {
		// A keyless actor has a drawn id and nothing to reserve.
		let actor_id = ctx.env().rng("actor.id").id_v1(ctx.config().dc_label());
		*progress = CreateProgress::Submitted(actor_id);
		create_step(ctx, retries, "actor_create", || {
			create(
				ctx,
				actor_id,
				&namespace,
				&pool_name,
				&body,
				input.clone(),
				false,
				None,
			)
		})
		.await?;
		*progress = CreateProgress::Acknowledged(actor_id);
		let data = wait_for_data(ctx, namespace.group_id(), actor_id, retries)
			.await?
			.context("keyless actor was destroyed during creation")?;
		return Ok(Created::Actor(Json(
			json!({ "actor": actor_json(&data, &namespace.name, None)?, "created": true }),
		)));
	};

	// Legacy: a lookup that finds no live actor in a namespace whose cached standing has no
	// group reads the namespace again, so the create below agrees with processes that
	// already see its group; see `legacy_find_keyed_actor`. Removed in 3.1 with the backfill.
	// The lookup only reads, so a refused attempt is retried whole.
	let (namespace, existing) = create_step(ctx, retries, "actor_lookup", || {
		legacy_find_keyed_actor(
			namespace.clone(),
			|namespace| find_keyed_actor(ctx, namespace, &body.name, &key),
			|namespace| async move { pegboard::actor_key::keyed_create_standing(ctx, &namespace).await },
		)
	})
	.await?;
	// A key being moved in from V2 is not this cluster's to create or return yet.
	if existing.as_ref().is_some_and(|actor| actor.held) {
		return Err(ActorMigratingFromV2::error());
	}
	if let Some(actor) = existing {
		if !get_or_create {
			return Err(DuplicateKey.build());
		}
		return Ok(Created::Actor(Json(
			json!({ "actor": actor_json(&actor.data, &namespace.name, None)?, "created": false }),
		)));
	}

	// A new keyed actor always takes the id of its name and key in the namespace's group,
	// never an actor not yet moved that the key resolved to. A namespace still without a
	// group after the re-read creates in the legacy shape older binaries read; see
	// `keyed_create_actor`.
	let actor_id = pegboard::actor_key::keyed_create_actor(
		ctx.config().dc_label(),
		&namespace,
		&body.name,
		&key,
	)
	.actor_id;

	// A create of a destroyed actor starts it fresh. One still being destroyed answers
	// `Destroyed`, so the key is resolved again and, with no live actor found, the create is
	// sent again shortly.
	loop {
		*progress = CreateProgress::Submitted(actor_id);
		// The create and the lookup after a `Destroyed` answer are separate storage steps, so a
		// refused lookup never sends the create again.
		let sent = create_step(ctx, retries, "actor_create", || {
			create(
				ctx,
				actor_id,
				&namespace,
				&pool_name,
				&body,
				input.clone(),
				force_actor_v3,
				None,
			)
		})
		.await?;
		let step = legacy_create_or_find(
			|| std::future::ready(Ok(sent)),
			|| {
				create_step(ctx, retries, "actor_lookup", || {
					find_keyed_actor(ctx, namespace.clone(), &body.name, &key)
				})
			},
		)
		.await?;
		let outcome = match step {
			LegacyCreateStep::Sent(outcome) => outcome,
			LegacyCreateStep::Found(actor) => {
				if !get_or_create {
					return Err(DuplicateKey.build());
				}
				return Ok(Created::Actor(Json(
					json!({ "actor": actor_json(&actor.data, &namespace.name, None)?, "created": false }),
				)));
			}
		};
		match outcome {
			wire::TryStartOutcome::TryStartEpoxyAlreadyReserved(reserved) => {
				let reservation_id = Id::from_slice(&reserved.reservation_id)?;
				return Err(ActorKeyOwnedElsewhere {
					datacenter_label: reservation_id.label(),
				}
				.build());
			}
			wire::TryStartOutcome::TryStartDestroyed => {
				*progress = CreateProgress::NotSubmitted;
				rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
				continue;
			}
			wire::TryStartOutcome::TryStartMigratingFromV2 => {
				return Err(ActorMigratingFromV2::error());
			}
			wire::TryStartOutcome::TryStartDelegatedToV2 => return Ok(Created::DelegatedToV2),
			// Legacy: only an unprefixed actor the backfill moves answers `LegacyMoving`, and
			// creates go to the group. Removed in 3.1 with the backfill.
			wire::TryStartOutcome::TryStartWaiting
			| wire::TryStartOutcome::TryStartStarting(_)
			| wire::TryStartOutcome::TryStartRunning(_)
			| wire::TryStartOutcome::TryStartFull
			| wire::TryStartOutcome::TryStartLegacyMoving => {}
		}
		*progress = CreateProgress::Acknowledged(actor_id);
		// A create whose key turns out to belong to V2 is destroyed here, so the next create
		// reports the delegation.
		let Some(data) = wait_for_data(ctx, namespace.group_id(), actor_id, retries).await? else {
			*progress = CreateProgress::NotSubmitted;
			rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
			continue;
		};
		return Ok(Created::Actor(Json(
			json!({ "actor": actor_json(&data, &namespace.name, None)?, "created": true }),
		)));
	}
}

/// The live keyed actor `name` and `key` name in `namespace`, read once through the resolve's own
/// read when it made one.
async fn find_keyed_actor(
	ctx: &EngineContext,
	namespace: Arc<NamespaceStanding>,
	name: &str,
	key: &str,
) -> Result<Option<ReadActor>> {
	let found = pegboard::actor_key::resolve_keyed_actor(
		ctx,
		ctx.config().dc_label(),
		&namespace,
		name,
		key,
	)
	.await?;
	let actor = found
		.read
		.into_read(&ctx.depot(), found.actor.group, found.actor.actor_id)
		.await?
		.and_then(read_actor_of)
		.filter(|actor| !actor.destroyed && belongs(&actor.data, &namespace));
	Ok(actor)
}

/// Legacy: finds a keyed actor for a create or getOrCreate with `find`, and only when it finds no
/// live actor reads the namespace again with `reread`
/// ([`pegboard::actor_key::keyed_create_standing`]), finding again when the namespace's group
/// changed. Returns the standing the create acts on with the live actor, if any. The legacy
/// behavior read the namespace before every keyed lookup; the new behavior spares an existing
/// actor that read, since only a create can land at the wrong id. Taken for every keyed create
/// and getOrCreate; `reread` itself reads nothing for a grouped standing or while older binaries
/// run. Removed in 3.1 with the backfill.
async fn legacy_find_keyed_actor<T, F, FindFut, R, RereadFut>(
	namespace: Arc<NamespaceStanding>,
	find: F,
	reread: R,
) -> Result<(Arc<NamespaceStanding>, Option<T>)>
where
	F: Fn(Arc<NamespaceStanding>) -> FindFut,
	FindFut: std::future::Future<Output = Result<Option<T>>>,
	R: FnOnce(Arc<NamespaceStanding>) -> RereadFut,
	RereadFut: std::future::Future<Output = Result<Arc<NamespaceStanding>>>,
{
	if let Some(actor) = find(namespace.clone()).await? {
		return Ok((namespace, Some(actor)));
	}

	let fresh = reread(namespace.clone()).await?;
	if fresh.group_id() == namespace.group_id() {
		return Ok((fresh, None));
	}
	let actor = find(fresh.clone()).await?;
	Ok((fresh, actor))
}

/// Legacy: what one pass of a keyed create loop decided ([`legacy_create_or_find`]). Removed in
/// 3.1 with the backfill.
enum LegacyCreateStep<T> {
	/// The create was sent and answered this outcome.
	Sent(wire::TryStartOutcome),
	/// The create found its actor destroying, and resolving the key again found a live actor,
	/// which the caller answers instead of creating.
	Found(T),
}

/// Legacy: sends a keyed create through `create` and, when it answers `TryStartDestroyed`,
/// resolves the key again through `find` before the caller sends it again. The backfill leaves
/// a copy of a legacy actor at the key's new id when a move fails, and that copy destroys
/// itself; the resolve reads through it to the live legacy actor once it is destroying, so a
/// create that waited it out never makes a fresh actor that shadows the legacy one, whose data
/// the next attempt's duplicate rule would destroy. The legacy behavior re-sent the create
/// without resolving. A create answered anything else costs nothing more. Removed in 3.1 with
/// the backfill.
async fn legacy_create_or_find<T, C, CF, F, FF>(create: C, find: F) -> Result<LegacyCreateStep<T>>
where
	C: FnOnce() -> CF,
	CF: std::future::Future<Output = Result<wire::TryStartOutcome>>,
	F: FnOnce() -> FF,
	FF: std::future::Future<Output = Result<Option<T>>>,
{
	let outcome = create().await?;
	match outcome {
		wire::TryStartOutcome::TryStartDestroyed => match find().await? {
			Some(actor) => Ok(LegacyCreateStep::Found(actor)),
			None => Ok(LegacyCreateStep::Sent(outcome)),
		},
		wire::TryStartOutcome::TryStartWaiting
		| wire::TryStartOutcome::TryStartStarting(_)
		| wire::TryStartOutcome::TryStartRunning(_)
		| wire::TryStartOutcome::TryStartEpoxyAlreadyReserved(_)
		| wire::TryStartOutcome::TryStartFull
		| wire::TryStartOutcome::TryStartLegacyMoving
		| wire::TryStartOutcome::TryStartMigratingFromV2
		| wire::TryStartOutcome::TryStartDelegatedToV2 => Ok(LegacyCreateStep::Sent(outcome)),
	}
}

/// Sends the create of `actor_id`. A create of an actor that exists keeps the existing actor's
/// data and wakes it. `fork_from` makes the new incarnation's database a fork of a source's.
async fn create(
	ctx: &EngineContext,
	actor_id: Id,
	namespace: &NamespaceStanding,
	pool_name: &str,
	body: &CreateActorRequest,
	input: Option<Vec<u8>>,
	force_actor_v3: bool,
	fork_from: Option<wire::ActorForkSource>,
) -> Result<wire::TryStartOutcome> {
	let (volume, request) = create_command(
		actor_id,
		namespace,
		pool_name,
		&body.name,
		body.key.as_deref(),
		input,
		force_actor_v3,
		fork_from,
		None,
	);
	let created = ctx
		.depot()
		.command::<ActorCreate>(&volume, request)
		.await
		.map_err(pegboard::commands::legacy_v3_create_refusal)?;
	Ok(created.outcome)
}

/// The state volume and request of the create of `actor_id` in `namespace`: the namespace's
/// group, or for a namespace without a group the unprefixed volume with the namespace recorded
/// as the actor's legacy namespace. `waiter` registers for the actor's start notices.
#[allow(
	clippy::too_many_arguments,
	reason = "the create's fields, passed through once"
)]
fn create_command(
	actor_id: Id,
	namespace: &NamespaceStanding,
	pool_name: &str,
	name: &str,
	key: Option<&str>,
	input: Option<Vec<u8>>,
	force_actor_v3: bool,
	fork_from: Option<wire::ActorForkSource>,
	waiter: Option<wire::StartWaiter>,
) -> (VolumeKey, wire::ActorCreateRequest) {
	let volume = ActorVolume::in_namespace(namespace.group_id(), actor_id);
	let request = wire::ActorCreateRequest {
		init: wire::ActorCreateData {
			actor_id: actor_id.as_bytes().into(),
			pool_name: pool_name.to_owned().into(),
			name: name.to_owned().into(),
			key: key.map(|key| key.to_owned().into()),
			input: input.map(Into::into),
			legacy_namespace: namespace.legacy_create_namespace().map(Into::into),
			fork_from,
			legacy_move: None,
		},
		waiter,
		force_actor_v3,
	};
	(volume, request)
}

/// The create of an actor fork. A fork can only be served by V3, so it never takes the V2 share
/// of a namespace migrating in from V2: `force_actor_v3` places it here. Its reservation still
/// asks V2, so a key V2 really holds stays V2's and the fork fails.
#[allow(
	clippy::too_many_arguments,
	reason = "the create's fields, passed through once"
)]
fn fork_create_command(
	actor_id: Id,
	namespace: &NamespaceStanding,
	pool_name: &str,
	name: &str,
	key: Option<&str>,
	input: Option<Vec<u8>>,
	fork_from: wire::ActorForkSource,
	waiter: Option<wire::StartWaiter>,
) -> (VolumeKey, wire::ActorCreateRequest) {
	create_command(
		actor_id,
		namespace,
		pool_name,
		name,
		key,
		input,
		true,
		Some(fork_from),
		waiter,
	)
}

/// Polls the actor until its data record is written. Returns `None` if it was destroyed first.
async fn wait_for_data(
	ctx: &EngineContext,
	group: Option<GroupId>,
	actor_id: Id,
	retries: &mut pegboard_gateway::Retries<'_>,
) -> Result<Option<wire::ActorData>> {
	let clock = ctx.env().clock();
	loop {
		match create_step(ctx, retries, "read_created_actor", || {
			read(ctx, group, actor_id)
		})
		.await?
		{
			Read::Actor(actor) if actor.destroyed => return Ok(None),
			Read::Actor(actor) => return Ok(Some(actor.data)),
			Read::Gone => return Ok(None),
			Read::NoData => {}
		}
		rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
	}
}

#[derive(Deserialize)]
pub struct ForkActorRequest {
	key: Option<String>,
	/// Base64-encoded fork input, given to the forked actor's fork hook.
	input: Option<String>,
}

/// Forks one actor: creates a new actor in the source's namespace, with the source's name and
/// pool, whose database is a Depot fork of the source's latest committed version, and starts it.
/// Answers once the database fork finished. A key held by any live actor, including an earlier
/// fork of the same source, answers `actor.duplicate_key`: forks are not idempotent. When the
/// request's deadline passes after the new actor was created, the fork goes on and the request
/// answers `actor.fork_pending` with the new actor's id, which the caller reads later.
pub async fn fork_actor(
	State(ctx): State<EngineContext>,
	Extension(waits): Extension<Arc<pegboard_gateway::start_waits::StartWaits>>,
	Path(source_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
	Json(body): Json<ForkActorRequest>,
) -> Result<Json<Value>, ApiError> {
	// Set once the namespace resolves, so a request naming no real namespace adds no label.
	let mut namespace_label = String::new();
	let result = fork_actor_inner(&ctx, &waits, query, source_id, body, &mut namespace_label).await;
	let outcome = match &result {
		Ok(_) => "ok",
		Err(error) => fork_result_label(error),
	};
	pegboard::ACTOR_FORKS
		.with_label_values(&[namespace_label.as_str(), outcome])
		.inc();
	Ok(result?)
}

/// The `result` label of a fork request that failed.
fn fork_result_label(error: &anyhow::Error) -> &'static str {
	let structured = rivet_error::RivetError::extract(error);
	match (structured.group(), structured.code()) {
		("actor", "duplicate_key") => "duplicate_key",
		("actor", "fork_too_deep") => "too_deep",
		("actor", "fork_failed") => "failed",
		("actor", "fork_pending") => "pending",
		("actor", "fork_unavailable") => "unavailable",
		("actor", "not_found") => "not_found",
		("guard", "service_unavailable") => "moving",
		_ => "error",
	}
}

/// Decodes base64 actor or fork input, answering `actor.invalid_input` for anything else.
fn decode_input(input: Option<String>) -> Result<Option<Vec<u8>>> {
	input
		.map(|input| {
			base64::engine::general_purpose::STANDARD
				.decode(input)
				.map_err(|_| pegboard::validation::ActorInvalidInput.build())
		})
		.transpose()
}

/// Whether a read of the source shows that its current incarnation was never sent a start, so
/// no runner created its database volume. An actor whose database is a fork's copy has one
/// from its create, and a response from a version that does not report the index proves
/// nothing.
fn source_never_started(read: &wire::ActorReadResponse) -> bool {
	let empty = match &read.database_origin {
		Some(wire::ActorDatabaseOrigin::DatabaseEmpty) => true,
		Some(
			wire::ActorDatabaseOrigin::DatabaseForked
			| wire::ActorDatabaseOrigin::DatabaseForkFailed(_),
		)
		| None => false,
	};
	empty && read.command_index == Some(0)
}

/// Legacy: whether a fork source is a keyed actor the backfill has not moved yet. Such a source
/// is read at its unprefixed volume (no group) under its 3.0.0 id, which the move retires for the
/// id its name and key derive in the namespace's group. A fork records its source's id in the
/// copy's `forked_from` and fork chain for good, so it is refused with a retryable refusal until
/// the move made the source's lasting id. A keyless source keeps its id when moved, and one the
/// finished backfill skipped is never moved, so neither waits. The new behavior has no such
/// move. Removed in 3.1 with the backfill.
fn legacy_source_awaits_move(
	source_group: Option<GroupId>,
	source: &wire::ActorData,
	namespace: &NamespaceStanding,
) -> bool {
	source_group.is_none() && source.key.is_some() && !namespace.backfill_done()
}

async fn fork_actor_inner(
	ctx: &EngineContext,
	waits: &Arc<pegboard_gateway::start_waits::StartWaits>,
	query: NamespaceQuery,
	source_id: String,
	body: ForkActorRequest,
	namespace_label: &mut String,
) -> Result<Json<Value>> {
	// The forked actor's start carries its fork, which an engine worker from before actor forks
	// cannot read, and its host is not known yet. Refuse before any work until no listed worker
	// reads an older envoy protocol; this reads memory only.
	if !pegboard::envoy_protocol::fleet_current(ctx) {
		return Err(pegboard::validation::ActorForkUnavailable {
			retry_after_ms: pegboard::envoy_protocol::FLEET_PENDING_RETRY_AFTER_MS,
		}
		.build()
		.into());
	}
	let namespace = namespace(ctx, query.namespace.as_deref()).await?;
	namespace_label.clone_from(&namespace.name);
	let source_id = Id::parse(&source_id)?;
	let key = body.key.as_deref().map(canonical_key).transpose()?;
	pegboard::validation::actor_key(key.as_deref())?;
	let input = decode_input(body.input)?;

	// The request's one deadline bounds everything below, including the wait for the fork.
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), CREATE_TIMEOUT)?;
	let expires_unix_ms = clock
		.observe()?
		.unix_ms
		.saturating_add(i64::try_from(CREATE_TIMEOUT.as_millis())?);

	// The source must be live in this namespace. A source in another namespace is not found,
	// since a fork never crosses namespaces. A source that never started has no database to
	// copy, so the fork is refused before anything is created.
	let admitted = rivet_env::timeout_at(clock.as_ref(), deadline, async {
		let (source_group, read) = resolve_actor(ctx, &namespace, source_id).await?;
		let Some(read) = read else {
			return Err(ActorNotFound.build());
		};
		let never_started = source_never_started(&read);
		let Some(source) = read_actor_of(read)
			.filter(|actor| !actor.destroyed && belongs(&actor.data, &namespace))
		else {
			return Err(ActorNotFound.build());
		};
		if never_started {
			return Err(pegboard::validation::ActorForkFailed::source_never_started().build());
		}
		let source = source.data;
		if legacy_source_awaits_move(source_group, &source, &namespace) {
			return Err(pegboard_gateway::legacy_actor_moving_error());
		}
		// The source's chain is read from its record, not its database, so a fork of a fork
		// that never committed its first start keeps its root. A chain at the depth limit is
		// refused here, before anything is created.
		let source_chain = pegboard::actor_machine::source_fork_chain(&source)?;

		let (actor_id, create_namespace) = match &key {
			// A keyless fork has a drawn id and nothing to reserve.
			None => (
				ctx.env().rng("actor.id").id_v1(ctx.config().dc_label()),
				namespace.clone(),
			),
			Some(key) => {
				// Legacy: a namespace whose cached standing has no group is read again first;
				// see `keyed_create_standing`. Removed in 3.1 with the backfill.
				let create_namespace =
					pegboard::actor_key::keyed_create_standing(ctx, &namespace).await?;
				let found = pegboard::actor_key::resolve_keyed_actor(
					ctx,
					ctx.config().dc_label(),
					&create_namespace,
					&source.name,
					key,
				)
				.await?;
				let existing = found
					.read
					.into_read(&ctx.depot(), found.actor.group, found.actor.actor_id)
					.await?
					.and_then(read_actor_of)
					.filter(|actor| !actor.destroyed && belongs(&actor.data, &create_namespace));
				if existing.is_some() {
					return Err(DuplicateKey.build());
				}
				let actor_id = pegboard::actor_key::keyed_create_actor(
					ctx.config().dc_label(),
					&create_namespace,
					&source.name,
					key,
				)
				.actor_id;
				(actor_id, create_namespace)
			}
		};
		Ok((
			source_group,
			source,
			source_chain,
			actor_id,
			create_namespace,
		))
	})
	.await
	.context("timed out before the actor fork was created")??;
	let (source_group, source, source_chain, actor_id, namespace) = admitted;

	let fork_id = ctx.env().rng("actor.fork").uuid_v4();
	let fork_from = wire::ActorForkSource {
		fork_id: fork_id.as_bytes().to_vec().into(),
		actor_id: source.actor_id.clone(),
		key: source.key.clone(),
		database_volume: pegboard::keys::partition::ActorDatabaseVolume::in_namespace(
			source_group,
			source_id,
			source.create_unix_ms,
		),
		chain: source_chain,
	};

	// The create registers a start waiter, which pegboard stores and dedupes across the create's
	// retries, so the request wakes when the forked actor's fork finishes, whether or not a node
	// can start it yet, or on its destroy, which follows a failed one, instead of reading it over
	// and over. A process at its capacity of waits reads the actor again just short of the
	// deadline.
	let mut wait = waits
		.join(pegboard_gateway::start_waits::WaitKey {
			group: namespace.group_id(),
			actor_id,
		})
		.await;
	let waiter = match &wait {
		Some(guard) => Some(wire::StartWaiter {
			worker_id: ctx.ups()?.worker_id().as_bytes().to_vec().into(),
			wait_id: guard.wait_id(),
			expires_unix_ms,
		}),
		None => None,
	};
	let read_by = deadline.saturating_sub(u64::try_from(FORK_LAST_READ_MARGIN.as_millis())?);

	// Once the create lands, a deadline that passes answers with the new actor's id for the
	// caller to read later, while the fork goes on. Before that, no actor of this fork exists.
	let mut created = false;
	let forked = rivet_env::timeout_at(clock.as_ref(), deadline, async {
		// A keyed actor still being destroyed answers `Destroyed`, so the create is sent again
		// shortly, as an ordinary create's is.
		loop {
			let send = || async {
				let (volume, request) = fork_create_command(
					actor_id,
					&namespace,
					&source.pool_name,
					&source.name,
					key.as_deref(),
					input.clone(),
					fork_from.clone(),
					waiter.clone(),
				);
				let response = ctx.depot().command::<ActorCreate>(&volume, request).await?;
				Ok(response.outcome)
			};
			// A keyless fork has no key to resolve again.
			let find = || async {
				match &key {
					Some(key) => find_keyed_actor(ctx, namespace.clone(), &source.name, key).await,
					None => Ok(None),
				}
			};
			let outcome = match legacy_create_or_find(send, find).await? {
				LegacyCreateStep::Sent(outcome) => outcome,
				LegacyCreateStep::Found(_actor) => return Err(DuplicateKey.build()),
			};
			match outcome {
				wire::TryStartOutcome::TryStartEpoxyAlreadyReserved(reserved) => {
					let reservation_id = Id::from_slice(&reserved.reservation_id)?;
					return Err(ActorKeyOwnedElsewhere {
						datacenter_label: reservation_id.label(),
					}
					.build());
				}
				wire::TryStartOutcome::TryStartDestroyed => {
					rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
					continue;
				}
				// A fork cannot be served by V2, so a fork target whose key V2 holds fails as one
				// the V2 migrator holds does.
				wire::TryStartOutcome::TryStartMigratingFromV2
				| wire::TryStartOutcome::TryStartDelegatedToV2 => {
					return Err(ActorMigratingFromV2::error());
				}
				// Legacy: only an unprefixed actor the backfill moves answers `LegacyMoving`, and
				// creates go to the group. Removed in 3.1 with the backfill.
				wire::TryStartOutcome::TryStartWaiting
				| wire::TryStartOutcome::TryStartStarting(_)
				| wire::TryStartOutcome::TryStartRunning(_)
				| wire::TryStartOutcome::TryStartFull
				| wire::TryStartOutcome::TryStartLegacyMoving => {}
			}
			created = true;

			let data = wait_for_fork(
				ctx,
				namespace.group_id(),
				actor_id,
				fork_id.as_bytes(),
				wait.as_mut(),
				read_by,
			)
			.await?;
			return Ok(Json(
				json!({ "actor": actor_json(&data, &namespace.name, None)? }),
			));
		}
	})
	.await;
	match forked {
		Ok(result) => result,
		Err(rivet_env::TimeoutError::Elapsed) => Err(fork_deadline_error(created, actor_id)),
		Err(rivet_env::TimeoutError::Clock(error)) => {
			Err(error).context("the actor fork's deadline could not be read")
		}
	}
}

/// The answer of a fork whose deadline passed. Once its create landed, `actor.fork_pending`
/// names the new actor, which keeps forking; before that, no actor of this fork exists, so it
/// answers the create route's timeout and names none.
fn fork_deadline_error(created: bool, actor_id: Id) -> anyhow::Error {
	if created {
		return pegboard::validation::ActorForkPending {
			actor_id: actor_id.to_string(),
		}
		.build();
	}
	anyhow::Error::new(rivet_env::TimeoutError::Elapsed)
		.context("timed out waiting for actor creation")
}

/// Waits for the forked actor's database fork to have an outcome, and answers the actor once
/// the fork finished, `actor.duplicate_key` when the actor belongs to another create that took
/// the key first, and the fork's error when it failed and the actor was destroyed. Pegboard
/// notifies the waiter when the fork's outcome commits and when a failed fork's destroy starts,
/// so it is read once now and once after each notice `wait` receives. Notices are lossy: without a wait, or with
/// no notice by `read_by`, it is read once more then, and a fork still pending at that read
/// answers `actor.fork_pending` with the new actor's id.
async fn wait_for_fork(
	ctx: &EngineContext,
	group: Option<GroupId>,
	actor_id: Id,
	fork_id: &[u8],
	mut wait: Option<&mut pegboard_gateway::start_waits::WaitGuard>,
	read_by: u64,
) -> Result<wire::ActorData> {
	let clock = ctx.env().clock();
	if let Some(data) = read_fork(ctx, group, actor_id, fork_id).await? {
		return Ok(data);
	}
	loop {
		let noticed = match wait.as_deref_mut() {
			Some(wait) => rivet_env::timeout_at(clock.as_ref(), read_by, wait.notified())
				.await
				.is_ok(),
			None => {
				clock.sleep_until(read_by).await?;
				false
			}
		};
		if let Some(data) = read_fork(ctx, group, actor_id, fork_id).await? {
			return Ok(data);
		}
		if !noticed {
			return Err(pegboard::validation::ActorForkPending {
				actor_id: actor_id.to_string(),
			}
			.build());
		}
	}
}

/// Reads the forked actor once: its data once the fork finished, `None` while it runs, and the
/// errors [`wait_for_fork`] answers.
async fn read_fork(
	ctx: &EngineContext,
	group: Option<GroupId>,
	actor_id: Id,
	fork_id: &[u8],
) -> Result<Option<wire::ActorData>> {
	let res = ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::in_namespace(group, actor_id), ())
		.await;
	let res = match res {
		Ok(res) => res,
		Err(error) if is_volume_gone(&error) => {
			return Err(pegboard::validation::ActorForkFailed::copy_failed().build());
		}
		Err(error) => return Err(error),
	};
	let Some(data) = &res.data else {
		return Ok(None);
	};
	let ours = data
		.forked_from
		.as_ref()
		.is_some_and(|source| &source.fork_id[..] == fork_id);
	if !ours {
		return Err(DuplicateKey.build());
	}
	match fork_outcome(&res) {
		ForkOutcome::Pending => Ok(None),
		ForkOutcome::Forked => Ok(Some(data.clone())),
		ForkOutcome::Failed(error) => Err(error),
	}
}

/// What a read of a forked actor shows about its database fork.
enum ForkOutcome {
	/// The fork has not finished.
	Pending,
	/// The fork finished; the actor is starting or past it.
	Forked,
	/// The actor was destroyed instead, with the error the request answers.
	Failed(anyhow::Error),
}

fn fork_outcome(res: &wire::ActorReadResponse) -> ForkOutcome {
	let Some(phase) = &res.phase else {
		return ForkOutcome::Pending;
	};
	match phase {
		wire::ActorPhase::Initializing(init) => {
			let forked = init.fork_database.as_ref().is_some_and(|fork| {
				matches!(
					fork.state.phase,
					wire::ForkDatabasePhase::ForkDatabasePhaseForked
				)
			});
			if forked {
				ForkOutcome::Forked
			} else {
				ForkOutcome::Pending
			}
		}
		wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Running(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_) => ForkOutcome::Forked,
		// A fork's own actor is never held or delegated, since neither is forked; one at its id
		// belongs to V2.
		wire::ActorPhase::MigratingFromV2(_) | wire::ActorPhase::DelegatedToV2 => {
			ForkOutcome::Failed(ActorMigratingFromV2::error())
		}
		wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed => {
			let error = match &res.database_origin {
				Some(wire::ActorDatabaseOrigin::DatabaseForkFailed(failed)) => {
					match failed.failure {
						wire::ForkDatabaseFailure::ChainTooDeep => {
							pegboard::validation::ActorForkTooDeep {
								max_depth: depot_fs_core::MAX_FORK_CHAIN_DEPTH,
							}
							.build()
						}
						wire::ForkDatabaseFailure::SourceGone
						| wire::ForkDatabaseFailure::Rejected
						| wire::ForkDatabaseFailure::MaxAttemptsReached => {
							pegboard::validation::ActorForkFailed::copy_failed().build()
						}
					}
				}
				// Destroyed for another reason while the fork ran, such as a destroy request.
				Some(
					wire::ActorDatabaseOrigin::DatabaseEmpty
					| wire::ActorDatabaseOrigin::DatabaseForked,
				)
				| None => pegboard::validation::ActorForkFailed::copy_failed().build(),
			};
			ForkOutcome::Failed(error)
		}
	}
}

pub async fn list_actors(
	State(ctx): State<EngineContext>,
	Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
	let selector = actor_listing_selector(&query)?;
	let namespace = namespace(&ctx, query.get("namespace").map(String::as_str)).await?;
	let include_destroyed =
		parse_include_destroyed(query.get("include_destroyed").map(String::as_str))?;
	let limit = parse_limit(query.get("limit").map(String::as_str))?;
	let after = query
		.get("cursor")
		.map(|cursor| parse_cursor(cursor))
		.transpose()?;

	// Actors this cluster does not hold are looked up in V2 while a V2 namespace migrates in.
	let dynamic = migrate_from_v2::snapshot(&ctx);
	let migration = migrate_from_v2::applies(dynamic.as_deref(), &namespace.name);

	let mut actors = Vec::new();
	let mut next_cursor = None;
	match selector {
		ActorListing::Ids(ids) => {
			let mut missing = Vec::new();
			for id in ids.split(',') {
				let actor_id = Id::parse(id)?;
				let (_group, read) = resolve_actor(&ctx, &namespace, actor_id).await?;
				match lookup_actor(&ctx, actor_id, read, &namespace, include_destroyed).await? {
					Some(actor) => actors.push(actor),
					None => missing.push(id),
				}
			}
			if let Some(config) = migration
				&& !missing.is_empty()
			{
				let missing = missing.join(",");
				actors.extend(v2_actors(&ctx, config, &[("actor_ids", &missing)]).await?);
			}
		}
		ActorListing::Key { name, key } => {
			// A key owned by another datacenter has no actor listed here. V2 is asked with the
			// key as the client wrote it.
			let v2_key = key;
			let key = canonical_key(key)?;
			let found = pegboard::actor_key::resolve_keyed_actor(
				&ctx,
				ctx.config().dc_label(),
				&namespace,
				name,
				&key,
			)
			.await?;
			let read = found
				.read
				.into_read(&ctx.depot(), found.actor.group, found.actor.actor_id)
				.await?;
			if let Some(actor) = lookup_actor(
				&ctx,
				found.actor.actor_id,
				read,
				&namespace,
				include_destroyed,
			)
			.await?
			{
				actors.push(actor);
			} else if let Some(config) = migration {
				actors.extend(v2_actors(&ctx, config, &[("name", name), ("key", v2_key)]).await?);
			}
		}
		ActorListing::Name(name) => {
			let page = pegboard::index::list_actors(
				&ctx,
				&namespace,
				name,
				after,
				limit,
				include_destroyed,
			)
			.await?;

			// Join each entry with its actor's record. The reads run in parallel and keep the
			// index order.
			let ctx_ref = &ctx;
			let namespace_ref = &namespace;
			let reads = futures_util::stream::iter(page.actors.iter().cloned())
				.map(|entry| async move {
					let actor_id = Id::from_slice(&entry.actor_id)?;
					let (_group, read) = resolve_actor(ctx_ref, namespace_ref, actor_id).await?;
					let actor = read.and_then(read_actor_of);
					anyhow::Ok((entry, actor))
				})
				.buffered(ACTOR_READ_CONCURRENCY)
				.collect::<Vec<_>>()
				.await;
			for read in reads {
				let (entry, actor) = read?;
				// An entry whose actor has no data yet, or is gone, is left out. The page may come
				// back short; the cursor still advances.
				let Some(actor) = actor else {
					continue;
				};
				if !belongs(&actor.data, &namespace) {
					continue;
				}
				match page.destroyed.get(&entry) {
					Some(&destroy_unix_ms) => actors.push(destroyed_entry_json(
						&actor.data,
						&namespace.name,
						entry.create_unix_ms,
						destroy_unix_ms,
					)?),
					None if actor.destroyed && !include_destroyed => {}
					None => actors.push(actor_json(&actor.data, &namespace.name, None)?),
				}
			}
			next_cursor = page
				.cursor
				.map(|cursor| format_cursor(&cursor))
				.transpose()?;
		}
	}

	Ok(Json(json!({
		"actors": actors,
		"pagination": { "cursor": next_cursor },
	})))
}

/// Lists one actor by ID from `read`, its state volume's read. A destroyed actor is listed only
/// when the listing includes destroyed actors, with the time its incarnation left the index.
async fn lookup_actor(
	ctx: &EngineContext,
	actor_id: Id,
	read: Option<wire::ActorReadResponse>,
	namespace: &NamespaceStanding,
	include_destroyed: bool,
) -> Result<Option<Value>> {
	let Some(actor) = read.and_then(read_actor_of) else {
		return Ok(None);
	};
	if !belongs(&actor.data, namespace) {
		return Ok(None);
	}
	if !actor.destroyed {
		return Ok(Some(actor_json(&actor.data, &namespace.name, None)?));
	}
	if !include_destroyed {
		return Ok(None);
	}

	let destroy_unix_ms = pegboard::index::read_destroyed_actor(
		ctx,
		namespace,
		&actor.data.name,
		actor_id,
		actor.data.create_unix_ms,
	)
	.await?;
	Ok(Some(actor_json(
		&actor.data,
		&namespace.name,
		destroy_unix_ms,
	)?))
}

/// A destroyed index entry joined with its actor's record. A keyed actor keeps its ID across
/// incarnations, so the record may already describe a later one; the entry's own creation time
/// is listed, with the record's name, key and pool, which a keyed actor shares across
/// incarnations.
fn destroyed_entry_json(
	data: &wire::ActorData,
	namespace: &str,
	create_unix_ms: i64,
	destroy_unix_ms: i64,
) -> Result<Value> {
	let mut actor = actor_json(data, namespace, Some(destroy_unix_ms))?;
	actor["create_ts"] = json!(create_unix_ms);
	Ok(actor)
}

#[derive(Debug, PartialEq, Eq)]
enum ActorListing<'a> {
	Ids(&'a str),
	Key { name: &'a str, key: &'a str },
	Name(&'a str),
}

/// ID lookups do not require an actor name. Key and name listings do.
fn actor_listing_selector(query: &HashMap<String, String>) -> Result<ActorListing<'_>> {
	if let Some(ids) = query.get("actor_ids").or(query.get("actor_id")) {
		return Ok(ActorListing::Ids(ids));
	}
	let name = query
		.get("name")
		.ok_or_else(|| InvalidActorListing.build())?;
	Ok(match query.get("key") {
		Some(key) => ActorListing::Key { name, key },
		None => ActorListing::Name(name),
	})
}

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"actor",
	"invalid_listing",
	"Listing actors requires a name or actor IDs."
)]
struct InvalidActorListing;

#[derive(Debug, rivet_error::RivetError)]
#[error("actor", "not_found", "The actor does not exist or was destroyed.")]
pub(crate) struct ActorNotFound;

/// The actor API receives keys in the slash form. One that would not come back unchanged, so
/// that two spellings could name one actor, is refused.
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"actor",
	"invalid_key_encoding",
	"Actor key is not in the canonical slash-separated form produced by serializeActorKey."
)]
struct InvalidKeyEncoding;

/// The canonical key of a key the actor API received in the slash form.
fn canonical_key(key: &str) -> Result<String> {
	actor_key::from_slash(key).map_err(|_| InvalidKeyEncoding.build())
}

#[derive(Debug, rivet_error::RivetError)]
#[error("actor", "duplicate_key", "Actor key already in use.")]
pub(crate) struct DuplicateKey;

/// Another datacenter owns the actor key. Forwarding across datacenters is not built yet.
#[derive(Debug, rivet_error::RivetError, serde::Serialize, serde::Deserialize)]
#[error(
	"actor",
	"key_owned_elsewhere",
	"The actor key is owned by another datacenter.",
	"The actor key is owned by datacenter {datacenter_label}."
)]
struct ActorKeyOwnedElsewhere {
	datacenter_label: u16,
}

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"actor",
	"invalid_include_destroyed",
	"The actor listing include_destroyed flag must be true or false."
)]
struct InvalidIncludeDestroyed;

#[derive(Debug, rivet_error::RivetError)]
#[error("actor", "invalid_cursor", "The actor listing cursor is malformed.")]
struct InvalidCursor;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"actor",
	"invalid_limit",
	"The actor listing limit must be an integer from 1 to 100."
)]
struct InvalidLimit;

fn parse_include_destroyed(value: Option<&str>) -> Result<bool> {
	match value {
		None | Some("false") => Ok(false),
		Some("true") => Ok(true),
		Some(_) => Err(InvalidIncludeDestroyed.build()),
	}
}

/// Parses the listing limit, defaulting to and capped at one page.
fn parse_limit(value: Option<&str>) -> Result<usize> {
	let Some(value) = value else {
		return Ok(LIST_PAGE_SIZE);
	};
	match value.parse::<usize>() {
		Ok(limit) if (1..=LIST_PAGE_SIZE).contains(&limit) && limit.to_string() == value => {
			Ok(limit)
		}
		Ok(_) | Err(_) => Err(InvalidLimit.build()),
	}
}

/// Encodes a listing position as the opaque public cursor `"{create_ms}.{actor_id}"`.
fn format_cursor(cursor: &wire::IndexActorCursor) -> Result<String> {
	let actor_id = Id::from_slice(&cursor.actor_id)?;
	Ok(format!("{}.{}", cursor.create_unix_ms, actor_id))
}

/// Parses a public cursor: the exact encoding [`format_cursor`] produces, or a V2 cursor.
///
/// V2 clients page by creation time alone and build the next cursor themselves: a bare
/// `{create_ms}` lists the actors created strictly before that millisecond. Listings run in
/// descending `(create_unix_ms, actor_id)` order and resume strictly after the cursor, so the
/// smallest actor ID at that millisecond is the same bound.
fn parse_cursor(value: &str) -> Result<wire::IndexActorCursor> {
	if let Ok(create_unix_ms) = value.parse::<i64>()
		&& create_unix_ms.to_string() == value
	{
		return Ok(wire::IndexActorCursor {
			create_unix_ms,
			actor_id: Id::V1([0; 18]).as_bytes().into(),
		});
	}

	let parsed = value.split_once('.').and_then(|(ms, actor_id)| {
		let create_unix_ms = ms.parse::<i64>().ok()?;
		let actor_id = Id::parse(actor_id).ok()?;
		Some(wire::IndexActorCursor {
			create_unix_ms,
			actor_id: actor_id.as_bytes().into(),
		})
	});
	match parsed {
		Some(cursor) if format_cursor(&cursor).ok().as_deref() == Some(value) => Ok(cursor),
		Some(_) | None => Err(InvalidCursor.build()),
	}
}

pub async fn get_actor(
	State(ctx): State<EngineContext>,
	Path(actor_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace_for_read(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
	let (_group, read) = resolve_actor(&ctx, &namespace, actor_id).await?;
	let Some(actor) = read
		.and_then(read_actor_of)
		.filter(|actor| !actor.destroyed && belongs(&actor.data, &namespace))
	else {
		// While a V2 namespace migrates in, V2 may still hold it.
		let dynamic = migrate_from_v2::snapshot(&ctx);
		if let Some(config) = migrate_from_v2::applies(dynamic.as_deref(), &namespace.name) {
			let id = actor_id.to_string();
			let found = v2_actors(&ctx, config, &[("actor_ids", &id)]).await?;
			if let Some(actor) = found.into_iter().next() {
				return Ok(Json(json!({ "actor": actor })));
			}
		}
		return Err(ActorNotFound.build().into());
	};

	Ok(Json(
		json!({ "actor": actor_json(&actor.data, &namespace.name, None)? }),
	))
}

pub async fn destroy_actor(
	State(ctx): State<EngineContext>,
	Path(actor_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Response, ApiError> {
	let namespace = namespace_for_read(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
	if pegboard::catalog::legacy_lists_skipped(&namespace, actor_id) {
		legacy_destroy_skipped(&ctx, &namespace, actor_id).await?;
		return Ok(Json(json!({})).into_response());
	}
	let v2_call = (Method::DELETE, format!("/actors/{actor_id}"));
	Ok(send_actor_event(
		&ctx,
		&namespace,
		actor_id,
		wire::ActorEvent::Destroy,
		v2_call,
	)
	.await?)
}

/// Legacy: destroys `actor_id`, an actor the finished backfill of `namespace` lists as skipped,
/// at its unprefixed volume, then removes it from that list, so forks and archives of the
/// namespace stop waiting for it and its opens stop falling back. Idempotent: an actor already
/// destroyed, or whose volume is already gone, is not sent the destroy again but still has its
/// entry removed, so a retry after a lost step, or after the actor destroyed itself through its
/// envoy, clears the list. Removed in 3.1 with the backfill.
async fn legacy_destroy_skipped(
	ctx: &EngineContext,
	namespace: &NamespaceStanding,
	actor_id: Id,
) -> Result<()> {
	let found = pegboard::legacy_volumes::legacy_resolve_ungrouped_actor(
		&ctx.depot(),
		namespace.group_id(),
		&namespace.name,
		actor_id,
	)
	.await?;
	if legacy_skipped_needs_destroy(found.read) {
		ctx.depot()
			.command::<ActorEvent>(
				&ActorVolume::in_namespace(None, actor_id),
				wire::ActorEvent::Destroy,
			)
			.await?;
	}
	pegboard::catalog::legacy_clear_skipped(ctx, &namespace.name, actor_id).await
}

/// Legacy: whether a listed skipped actor, as its unprefixed volume read, still needs the
/// destroy sent: only a live one does. An actor already destroying or destroyed, or whose volume
/// is gone, only has its skipped entry removed. Removed in 3.1 with the backfill.
fn legacy_skipped_needs_destroy(read: pegboard::legacy_volumes::ResolvedRead) -> bool {
	match read {
		pegboard::legacy_volumes::ResolvedRead::Found(read) => {
			read_actor_of(read).is_some_and(|actor| !actor.destroyed)
		}
		pegboard::legacy_volumes::ResolvedRead::Gone
		| pegboard::legacy_volumes::ResolvedRead::Unread => false,
	}
}

pub async fn sleep_actor(
	State(ctx): State<EngineContext>,
	Path(actor_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Response, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
	let v2_call = (Method::POST, format!("/actors/{actor_id}/sleep"));
	Ok(send_actor_event(&ctx, &namespace, actor_id, wire::ActorEvent::Sleep, v2_call).await?)
}

/// Moves the actor to a new allocation, as main's reschedule does: a running or stopping actor
/// is stopped and allocated again, and a sleeping actor is allocated now. An actor that is
/// still being created or started is left alone, since it is already allocating, and a
/// destroyed actor has nothing to reschedule. The request body is ignored.
pub async fn reschedule_actor(
	State(ctx): State<EngineContext>,
	Path(actor_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
	let (group, read) = resolve_actor(&ctx, &namespace, actor_id).await?;
	let Some(res) = read else {
		return Err(ActorNotFound.build().into());
	};
	let (Some(data), Some(phase)) = (res.data, res.phase) else {
		return Err(ActorNotFound.build().into());
	};
	if !belongs(&data, &namespace) {
		return Err(ActorNotFound.build().into());
	}

	if let Some((volume, event)) = reschedule_command(group, actor_id, &phase, res.epoch) {
		ctx.depot().command::<ActorEvent>(&volume, event).await?;
	}

	Ok(Json(json!({})))
}

/// The volume and event that reschedule an actor in `phase`, or `None` when there is nothing
/// to move.
///
/// The event goes to the actor's volume in its namespace's group, the same volume the read
/// used, so a grouped namespace's actor receives it. The `Reallocate` carries the epoch the
/// read saw, so an actor that moved on since the read ignores it instead of being rescheduled
/// twice.
fn reschedule_command(
	group: Option<GroupId>,
	actor_id: Id,
	phase: &wire::ActorPhase,
	epoch: u32,
) -> Option<(VolumeKey, wire::ActorEvent)> {
	let event = reschedule_event(phase, epoch)?;
	let volume = ActorVolume::in_namespace(group, actor_id);

	Some((volume, event))
}

/// The event that reschedules an actor in `phase`, or `None` when there is nothing to move.
fn reschedule_event(phase: &wire::ActorPhase, epoch: u32) -> Option<wire::ActorEvent> {
	match phase {
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_) => Some(wire::ActorEvent::Reallocate(wire::Reallocate {
			epoch,
			lost: false,
			waiter: None,
		})),
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed => None,
		// V2 serves a held or delegated actor, so there is nothing here to move.
		wire::ActorPhase::MigratingFromV2(_) | wire::ActorPhase::DelegatedToV2 => None,
	}
}

/// Sends `event` to an existing actor's state machine.
///
/// The existence check gives a missing actor a clear error instead of the state machine's
/// internal one. While a V2 namespace migrates in, an actor this cluster does not hold gets the
/// same call, `v2_call`, on V2's public API, and one being moved in is refused as the retryable
/// `actor.migrating_from_v2`, so a destroy during the move is not lost.
async fn send_actor_event(
	ctx: &EngineContext,
	namespace: &NamespaceStanding,
	actor_id: Id,
	event: wire::ActorEvent,
	v2_call: (Method, String),
) -> Result<Response> {
	let (group, read) = resolve_actor(ctx, namespace, actor_id).await?;
	let dynamic = migrate_from_v2::snapshot(ctx);
	if let Some(config) = migrate_from_v2::applies(dynamic.as_deref(), &namespace.name) {
		let state = match &read {
			Some(read) => migrate_from_v2::classify(read.data.is_some(), read.phase.as_ref()),
			None => V3Actor::None,
		};
		match state {
			V3Actor::Active => {}
			V3Actor::Held => return Err(ActorMigratingFromV2::error()),
			V3Actor::Delegated | V3Actor::None => {
				let (method, path) = v2_call;
				let path = format!("{path}?{}", v2_namespace_query(config)?);
				let body = if method == Method::POST {
					Some(Bytes::from_static(b"{}"))
				} else {
					None
				};
				let response =
					pegboard::migrate_from_v2::api(ctx, config, method, &path, body).await?;
				return Ok(proxy::api_response(response));
			}
		}
	}
	if read
		.and_then(read_actor_of)
		.is_none_or(|actor| !belongs(&actor.data, namespace))
	{
		return Err(ActorNotFound.build());
	}

	ctx.depot()
		.command::<ActorEvent>(&ActorVolume::in_namespace(group, actor_id), event)
		.await?;

	Ok(Json(json!({})).into_response())
}

/// Pool configuration is flat. Opaque metadata is stored without interpretation.
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PoolRequest {
	drain_on_version_upgrade: bool,
	actor_eviction_delay: u32,
	actor_eviction_period: u32,
	actor_eviction_rate: f64,
	metadata: Option<Value>,
}
impl Default for PoolRequest {
	fn default() -> Self {
		Self {
			drain_on_version_upgrade: false,
			actor_eviction_delay: 0,
			actor_eviction_period: 0,
			actor_eviction_rate: 1.0,
			metadata: None,
		}
	}
}
impl PoolRequest {
	fn validate(self) -> Result<wire::NodePoolConfig> {
		if !self.actor_eviction_rate.is_finite()
			|| self.actor_eviction_rate <= 0.0
			|| self.actor_eviction_delay > 86400
			|| self.actor_eviction_period > 86400
		{
			return Err(InvalidPoolConfig.build());
		}
		let metadata = self
			.metadata
			.map(|value| serde_json::to_vec(&value))
			.transpose()?;
		if metadata.as_ref().is_some_and(|value| value.len() > 16384) {
			return Err(InvalidPoolConfig.build());
		}
		Ok(wire::NodePoolConfig {
			drain_on_version_upgrade: self.drain_on_version_upgrade,
			actor_eviction_delay: self.actor_eviction_delay,
			actor_eviction_period: self.actor_eviction_period,
			actor_eviction_rate: self.actor_eviction_rate,
			metadata: metadata.map(Into::into),
		})
	}
}

fn parse_pool_request(body: Value) -> Result<wire::NodePoolConfig> {
	if body.get("serverless").is_some()
		|| body.get("kind").and_then(Value::as_str) == Some("serverless")
	{
		return Err(ServerlessUnsupported.build());
	}
	serde_json::from_value::<PoolRequest>(body)
		.map_err(|_| InvalidPoolConfig.build())?
		.validate()
}
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"pool",
	"serverless_unsupported",
	"Serverless node pools are not supported."
)]
struct ServerlessUnsupported;

fn pool_json(name: &str, config: &wire::NodePoolConfig) -> Result<Value> {
	let metadata = config
		.metadata
		.as_ref()
		.map(|value| serde_json::from_slice::<Value>(value))
		.transpose()?;
	Ok(json!({
		"name": name,
		"drain_on_version_upgrade": config.drain_on_version_upgrade,
		"actor_eviction_delay": config.actor_eviction_delay,
		"actor_eviction_period": config.actor_eviction_period,
		"actor_eviction_rate": config.actor_eviction_rate,
		"metadata": metadata,
	}))
}

pub async fn upsert_pool(
	State(ctx): State<EngineContext>,
	Path(name): Path<String>,
	Query(query): Query<NamespaceQuery>,
	Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let reply = pegboard::catalog::pool(
		&ctx,
		&namespace.name,
		&name,
		wire::PoolOperation::PoolPut(parse_pool_request(body)?),
	)
	.await?;
	Ok(Json(
		json!({"pool":pool_json(&name,&reply.config.context("pool put returned no config")?)?}),
	))
}

pub async fn get_pool(
	State(ctx): State<EngineContext>,
	Path(name): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let reply =
		pegboard::catalog::pool(&ctx, &namespace.name, &name, wire::PoolOperation::PoolRead)
			.await?;
	let config = reply
		.config
		.ok_or_else(|| pegboard::validation::PoolNotFound.build())?;
	Ok(Json(json!({"pool":pool_json(&name,&config)?})))
}

pub async fn delete_pool(
	State(ctx): State<EngineContext>,
	Path(name): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Json<Value>, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let reply = pegboard::catalog::pool(
		&ctx,
		&namespace.name,
		&name,
		wire::PoolOperation::PoolDelete,
	)
	.await?;
	match reply.status {
		wire::PoolStatus::Ok => Ok(Json(json!({}))),
		wire::PoolStatus::NotFound => Err(pegboard::validation::PoolNotFound.build().into()),
		wire::PoolStatus::Connected => Err(PoolConnected.build().into()),
	}
}

#[derive(Debug, rivet_error::RivetError)]
#[error("pool", "invalid", "Node pool configuration is invalid.")]
struct InvalidPoolConfig;
#[derive(Debug, rivet_error::RivetError)]
#[error("pool", "nodes_connected", "The node pool still has connected nodes.")]
struct PoolConnected;

/// The actor names every live envoy of the namespace accepts.
async fn actor_names(ctx: &EngineContext, namespace: &str) -> Result<Value> {
	let names = live_envoys(ctx, namespace)
		.await?
		.into_iter()
		.flat_map(|envoy| envoy.names)
		.collect::<BTreeSet<_>>();
	let names = names
		.into_iter()
		.map(|name| (name, json!({ "metadata": {} })))
		.collect::<serde_json::Map<_, _>>();
	Ok(Value::Object(names))
}

/// A live envoy of a listing: one inside the heartbeat window, so allocation may pick it.
struct LiveEnvoy {
	pool_name: String,
	key: String,
	names: Vec<String>,
}

/// Lists the live envoys of every pool of the namespace from the active index. Presence there
/// already means connected and not stopping: a connection writes no heartbeat in either state
/// and leaves the index on both, so no record read is needed.
async fn live_envoys(ctx: &EngineContext, namespace: &str) -> Result<Vec<LiveEnvoy>> {
	let mut live = Vec::new();
	for pool_name in pegboard::index::list_envoy_pools(ctx, namespace).await? {
		for envoy in pegboard::index::list_active_envoy_states(ctx, namespace, &pool_name).await? {
			live.push(LiveEnvoy {
				pool_name: pool_name.clone(),
				key: envoy.envoy_key.to_string(),
				names: envoy.names.iter().map(|name| name.to_string()).collect(),
			});
		}
	}
	live.sort_by(|a, b| (&a.pool_name, &a.key).cmp(&(&b.pool_name, &b.key)));
	Ok(live)
}

/// An actor's data record and whether it is on its way out.
pub(super) struct ReadActor {
	pub(super) data: wire::ActorData,
	/// The actor is being or has been destroyed.
	destroyed: bool,
	/// The V2 migrator holds the actor while its data moves in.
	held: bool,
}

/// What a read of an actor found.
enum Read {
	Actor(ReadActor),
	/// The state volume exists but the data write has not landed yet.
	NoData,
	/// No state volume: the actor never existed or was destroyed and deleted.
	Gone,
}

/// Reads the actor's record from its state volume in `group`, its namespace's group.
async fn read(ctx: &EngineContext, group: Option<GroupId>, actor_id: Id) -> Result<Read> {
	let res = ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::in_namespace(group, actor_id), ())
		.await;
	let res = match res {
		Ok(res) => res,
		Err(error) if is_volume_gone(&error) => return Ok(Read::Gone),
		Err(error) => return Err(error),
	};
	// A key delegated to V2 has no actor here.
	if let Some(wire::ActorPhase::DelegatedToV2) = res.phase {
		return Ok(Read::Gone);
	}
	Ok(match read_actor_of(res) {
		Some(actor) => Read::Actor(actor),
		None => Read::NoData,
	})
}

/// The actor a state volume read found, or `None` when its data write has not landed yet.
fn read_actor_of(res: wire::ActorReadResponse) -> Option<ReadActor> {
	let destroyed = matches!(
		res.phase,
		Some(wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed)
	);
	let held = matches!(res.phase, Some(wire::ActorPhase::MigratingFromV2(_)));
	res.data.map(|data| ReadActor {
		data,
		destroyed,
		held,
	})
}

/// Resolves the group `actor_id`'s volumes are keyed in for `namespace` and reads its state
/// volume there, reusing the read the legacy fallback made when it made one, so an actor costs
/// one read however it resolved, and one not yet moved two. `None` when the volume is missing or
/// deleted.
pub(super) async fn resolve_actor(
	ctx: &EngineContext,
	namespace: &NamespaceStanding,
	actor_id: Id,
) -> Result<(Option<GroupId>, Option<wire::ActorReadResponse>)> {
	let found = legacy_resolve_actor(ctx, namespace, actor_id).await?;
	let read = found
		.read
		.into_read(&ctx.depot(), found.group, actor_id)
		.await?;
	Ok((found.group, read))
}

/// Whether an actor read from a namespace's volumes belongs to it; see
/// [`pegboard::catalog::actor_in_namespace`].
pub(crate) fn belongs(actor: &wire::ActorData, namespace: &NamespaceStanding) -> bool {
	namespace.holds_actor(actor)
}

/// The source actor id an actor's database was forked from, or null.
fn forked_from_json(actor: &wire::ActorData) -> Result<Value> {
	match &actor.forked_from {
		Some(source) => Ok(json!(Id::from_slice(&source.actor_id)?.to_string())),
		None => Ok(Value::Null),
	}
}

/// An actor in main's public shape, in the namespace it was asked about. v3 does not record
/// when an actor started, became connectable, slept or waited for an allocation, so those
/// fields are null.
pub fn actor_json(
	actor: &wire::ActorData,
	namespace: &str,
	destroy_unix_ms: Option<i64>,
) -> Result<Value> {
	Ok(json!({
		"actor_id": Id::from_slice(&actor.actor_id)?.to_string(),
		"name": actor.name,
		"key": actor.key.as_deref().map(actor_key::to_slash).unwrap_or_default(),
		"namespace": namespace,
		"pool": actor.pool_name,
		"create_ts": actor.create_unix_ms,
		"start_ts": null,
		"pending_allocation_ts": null,
		"connectable_ts": null,
		"sleep_ts": null,
		"reschedule_ts": null,
		"destroy_ts": destroy_unix_ms,
		"forked_from": forked_from_json(actor)?,
	}))
}
