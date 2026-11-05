use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result, bail, ensure};
use axum::{
	Json,
	extract::{Path, Query, State},
	http::{HeaderMap, Method},
	response::{IntoResponse, Response},
};
use base64::Engine;
use bytes::Bytes;
use futures_util::StreamExt;
use pegboard::{
	commands::{ActorCreate, ActorEvent, ActorRead, is_volume_gone},
	errors::ActorMigratingFromV2,
	keys::partition::ActorVolume,
	wire,
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
use std::time::Duration;

use crate::ApiError;

/// How long a create waits for the actor's data record, across retries.
const CREATE_TIMEOUT: Duration = Duration::from_secs(30);

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
	let authenticated = auth::check_edge(ctx.config(), auth::transport_token(&headers)).is_ok();
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
			&pegboard::keys::partition::EnvoyPoolIndexVolume::new(namespace).volume_key(),
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
			&namespace,
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
	let (ctx_ref, namespace_ref, pool_ref) = (&ctx, namespace.as_str(), pool.as_str());
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
			nodes.push(node_json(&namespace, &pool, state, &record));
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
		json!({ "names": actor_names(&ctx, &namespace).await? }),
	))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNamespaceRequest {
	name: String,
}

pub async fn create_namespace(
	State(ctx): State<EngineContext>,
	Json(body): Json<CreateNamespaceRequest>,
) -> Result<Json<Value>, ApiError> {
	pegboard::validation::namespace_name(&body.name)?;
	let reply = ctx
		.depot()
		.command::<pegboard::catalog::NamespaceCreate>(
			&pegboard::keys::partition::NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceCreateRequest {
				name: body.name.clone().into(),
				create_unix_ms: ctx.env().clock().observe()?.unix_ms,
			},
		)
		.await?;
	ctx.remember_namespace(&body.name);
	if !reply.created {
		return Err(NamespaceNameNotUnique.build().into());
	}
	Ok(Json(
		json!({"namespace": {"name": body.name, "create_ts": reply.record.create_unix_ms}}),
	))
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
	let volume = pegboard::keys::partition::NamespaceCatalogVolume::new().volume_key();
	if let Some(name) = query.name {
		pegboard::validation::namespace_name(&name)?;
		let reply = ctx
			.depot()
			.command::<pegboard::catalog::NamespaceRead>(
				&volume,
				wire::NamespaceReadRequest {
					name: name.clone().into(),
				},
			)
			.await?;
		if reply.record.is_some() {
			ctx.remember_namespace(&name);
		}
		let namespaces = reply
			.record
			.map(|record| json!({"name": name,"create_ts":record.create_unix_ms}))
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
	let reply = ctx
		.depot()
		.command::<pegboard::catalog::NamespaceList>(
			&volume,
			wire::NamespaceListRequest {
				after: query.cursor.map(Into::into),
				limit: query.limit.unwrap_or(100),
			},
		)
		.await?;
	for entry in &reply.namespaces {
		ctx.remember_namespace(&entry.name);
	}
	let namespaces = reply
		.namespaces
		.iter()
		.map(|entry| json!({"name":entry.name,"create_ts":entry.record.create_unix_ms}))
		.collect::<Vec<_>>();
	Ok(Json(
		json!({"namespaces":namespaces,"pagination":{"cursor":reply.cursor}}),
	))
}

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"namespace",
	"name_not_unique",
	"A namespace with this name already exists."
)]
struct NamespaceNameNotUnique;
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

/// The validated namespace of a request, defaulting to the default namespace.
async fn namespace(ctx: &EngineContext, query: Option<&str>) -> Result<String> {
	let namespace = query.unwrap_or(pegboard::DEFAULT_NAMESPACE);
	pegboard::catalog::ensure_namespace(ctx, namespace).await?;
	Ok(namespace.to_owned())
}

pub async fn create_actor(
	State(ctx): State<EngineContext>,
	Query(query): Query<NamespaceQuery>,
	headers: HeaderMap,
	raw: Bytes,
) -> Result<Response, ApiError> {
	Ok(create_or_proxy(&ctx, query, &headers, raw, false).await?)
}

pub async fn get_or_create_actor(
	State(ctx): State<EngineContext>,
	Query(query): Query<NamespaceQuery>,
	headers: HeaderMap,
	raw: Bytes,
) -> Result<Response, ApiError> {
	Ok(create_or_proxy(&ctx, query, &headers, raw, true).await?)
}

/// Creates the actor here, or proxies the create to V2's public API while a V2 namespace
/// migrates in and the configured share sends it there. Keyless creates draw against the share;
/// keyed creates stay with V2 until it reaches 1.0, and then go to V2 only for a key V2 already
/// holds. A key this cluster holds is always served here, and one being moved in is refused as
/// the retryable `actor.migrating_from_v2`. A create that sends `x-rivet-actor-version: 3`
/// decides as if the share were 1.0.
async fn create_or_proxy(
	ctx: &EngineContext,
	query: NamespaceQuery,
	headers: &HeaderMap,
	raw: Bytes,
	get_or_create: bool,
) -> Result<Response> {
	let body = match Json::<CreateActorRequest>::from_bytes(&raw) {
		Ok(Json(body)) => body,
		Err(rejection) => return Ok(rejection.into_response()),
	};
	let namespace = namespace(ctx, query.namespace.as_deref()).await?;
	let dynamic = migrate_from_v2::snapshot(ctx);
	let Some(config) = migrate_from_v2::applies(dynamic.as_deref(), &namespace) else {
		return Ok(create_actor_inner(ctx, query, body, get_or_create)
			.await?
			.into_response());
	};
	let create_ratio = migrate_from_v2::create_ratio(config, headers)?;
	let keyed = body.key.is_some();
	let count_create = |target: &'static str| {
		migrate_from_v2::metrics::CREATES
			.with_label_values(&[
				namespace.as_str(),
				target,
				if keyed { "true" } else { "false" },
			])
			.inc();
	};

	let route = match &body.key {
		None => {
			let draw = (ctx.env().rng("migrate_from_v2.create").next_u64() >> 11) as f64
				/ (1u64 << 53) as f64;
			decision::keyless_create(draw, create_ratio)
		}
		Some(key) => {
			// The API speaks the slash key form; the keyed id hashes the canonical form, as
			// `create_actor_inner` does.
			let canonical = canonical_key(key)?;
			pegboard::validation::actor_name(&body.name)?;
			pegboard::validation::actor_key(Some(&canonical))?;
			let actor_id = pegboard::actor_key::keyed_actor_id(
				ctx.config().dc_label(),
				&namespace,
				&body.name,
				&canonical,
			);
			let state = migrate_from_v2::read_v3_actor(ctx, actor_id).await?;
			let route = match decision::keyed(state, true, create_ratio) {
				decision::Route::AskV2 => {
					let found = proxy::key_exists(ctx, config, &body.name, key).await?;
					decision::keyed_after_lookup(found)
				}
				route => route,
			};
			// An existing actor here is read, not created.
			if let (decision::Route::V3, V3Actor::Active) = (route, state) {
				return Ok(create_actor_inner(ctx, query, body, get_or_create)
					.await?
					.into_response());
			}
			route
		}
	};

	match route {
		decision::Route::V3 => {
			count_create("v3");
			Ok(create_actor_inner(ctx, query, body, get_or_create)
				.await?
				.into_response())
		}
		decision::Route::V2 => {
			count_create("v2");
			let method = if get_or_create {
				Method::PUT
			} else {
				Method::POST
			};
			let path = format!("/actors?{}", v2_namespace_query(config)?);
			let response = proxy::api(ctx, config, method, &path, Some(raw)).await?;
			Ok(response.into_response())
		}
		decision::Route::RetryLater => Err(ActorMigratingFromV2::error()),
		decision::Route::NotFound | decision::Route::AskV2 => Err(ActorNotFound.build()),
	}
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
	let response = proxy::api(ctx, config, Method::GET, &path, None).await?;
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

/// Creates the actor and waits for its data record.
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
) -> Result<Json<Value>> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	// The API speaks the slash key form; the key is hashed and stored in its canonical form.
	body.key = body.key.as_deref().map(canonical_key).transpose()?;
	let pool_name = body
		.runner_name_selector
		.clone()
		.unwrap_or_else(|| pegboard::DEFAULT_POOL.to_owned());
	pegboard::validation::actor_name(&body.name)?;
	pegboard::validation::actor_key(body.key.as_deref())?;
	pegboard::validation::pool_name(&pool_name)?;
	let input = body
		.input
		.clone()
		.map(|input| base64::engine::general_purpose::STANDARD.decode(input))
		.transpose()
		.context("invalid base64 actor input")?;

	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), CREATE_TIMEOUT)?;

	rivet_env::timeout_at(clock.as_ref(), deadline, async {
		let Some(key) = body.key.clone() else {
			// A keyless actor has a drawn id and nothing to reserve.
			let actor_id = ctx.env().rng("actor.id").id_v1(ctx.config().dc_label());
			create(ctx, actor_id, &namespace, &pool_name, &body, input.clone()).await?;
			let data = wait_for_data(ctx, actor_id)
				.await?
				.context("keyless actor was destroyed during creation")?;
			return Ok(Json(
				json!({ "actor": actor_json(&data, None)?, "created": true }),
			));
		};

		let actor_id = pegboard::actor_key::keyed_actor_id(
			ctx.config().dc_label(),
			&namespace,
			&body.name,
			&key,
		);
		let existing = read_actor(ctx, actor_id)
			.await?
			.filter(|actor| !actor.destroyed);
		// A key being moved in from V2 is not this cluster's to create or return yet.
		if existing.as_ref().is_some_and(|actor| actor.held) {
			return Err(ActorMigratingFromV2::error());
		}
		if let Some(actor) = existing {
			if !get_or_create {
				return Err(DuplicateKey.build());
			}
			return Ok(Json(
				json!({ "actor": actor_json(&actor.data, None)?, "created": false }),
			));
		}

		// A create of a destroyed actor starts it fresh. One still being destroyed answers
		// `Destroyed`, so the create is sent again shortly.
		loop {
			let outcome =
				create(ctx, actor_id, &namespace, &pool_name, &body, input.clone()).await?;
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
				wire::TryStartOutcome::TryStartMigratingFromV2 => {
					return Err(ActorMigratingFromV2::error());
				}
				wire::TryStartOutcome::TryStartWaiting
				| wire::TryStartOutcome::TryStartStarting(_)
				| wire::TryStartOutcome::TryStartRunning(_)
				| wire::TryStartOutcome::TryStartFull => {}
			}
			let Some(data) = wait_for_data(ctx, actor_id).await? else {
				rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
				continue;
			};
			return Ok(Json(
				json!({ "actor": actor_json(&data, None)?, "created": true }),
			));
		}
	})
	.await
	.context("timed out waiting for actor creation")?
}

/// Sends the create of `actor_id`. A create of an actor that exists keeps the existing actor's
/// data and wakes it.
async fn create(
	ctx: &EngineContext,
	actor_id: Id,
	namespace: &str,
	pool_name: &str,
	body: &CreateActorRequest,
	input: Option<Vec<u8>>,
) -> Result<wire::TryStartOutcome> {
	let created = ctx
		.depot()
		.command::<ActorCreate>(
			&ActorVolume::new(actor_id).volume_key(),
			wire::ActorCreateRequest {
				init: wire::ActorCreateData {
					actor_id: actor_id.as_bytes().into(),
					namespace: namespace.to_owned().into(),
					pool_name: pool_name.to_owned().into(),
					name: body.name.clone().into(),
					key: body.key.clone().map(Into::into),
					input: input.map(Into::into),
				},
				waiter: None,
			},
		)
		.await?;
	Ok(created.outcome)
}

/// Polls the actor until its data record is written. Returns `None` if it was destroyed first.
async fn wait_for_data(ctx: &EngineContext, actor_id: Id) -> Result<Option<wire::ActorData>> {
	let clock = ctx.env().clock();
	loop {
		match read(ctx, actor_id).await? {
			Read::Actor(actor) if actor.destroyed => return Ok(None),
			Read::Actor(actor) => return Ok(Some(actor.data)),
			Read::Gone => return Ok(None),
			Read::NoData => {}
		}
		rivet_env::sleep(clock.as_ref(), CREATE_POLL_INTERVAL).await?;
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
	let migration = migrate_from_v2::applies(dynamic.as_deref(), &namespace);

	let mut actors = Vec::new();
	let mut next_cursor = None;
	match selector {
		ActorListing::Ids(ids) => {
			let mut missing = Vec::new();
			for id in ids.split(',') {
				match lookup_actor(&ctx, Id::parse(id)?, &namespace, include_destroyed).await? {
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
			let actor_id = pegboard::actor_key::keyed_actor_id(
				ctx.config().dc_label(),
				&namespace,
				name,
				&key,
			);
			if let Some(actor) = lookup_actor(&ctx, actor_id, &namespace, include_destroyed).await?
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
			let reads = futures_util::stream::iter(page.actors.iter().cloned())
				.map(|entry| async move {
					let actor_id = Id::from_slice(&entry.actor_id)?;
					let actor = read_actor(ctx_ref, actor_id).await?;
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
				if &*actor.data.namespace != namespace.as_str() {
					continue;
				}
				match page.destroyed.get(&entry) {
					Some(&destroy_unix_ms) => actors.push(destroyed_entry_json(
						&actor.data,
						entry.create_unix_ms,
						destroy_unix_ms,
					)?),
					None if actor.destroyed && !include_destroyed => {}
					None => actors.push(actor_json(&actor.data, None)?),
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

/// Looks up one actor by ID for a listing. A destroyed actor is listed only when the listing
/// includes destroyed actors, with the time its incarnation left the index.
async fn lookup_actor(
	ctx: &EngineContext,
	actor_id: Id,
	namespace: &str,
	include_destroyed: bool,
) -> Result<Option<Value>> {
	let Some(actor) = read_actor(ctx, actor_id).await? else {
		return Ok(None);
	};
	if &*actor.data.namespace != namespace {
		return Ok(None);
	}
	if !actor.destroyed {
		return Ok(Some(actor_json(&actor.data, None)?));
	}
	if !include_destroyed {
		return Ok(None);
	}

	let destroy_unix_ms = pegboard::index::read_destroyed_actor(
		ctx,
		&actor.data.namespace,
		&actor.data.name,
		actor_id,
		actor.data.create_unix_ms,
	)
	.await?;
	Ok(Some(actor_json(&actor.data, destroy_unix_ms)?))
}

/// A destroyed index entry joined with its actor's record. A keyed actor keeps its ID across
/// incarnations, so the record may already describe a later one; the entry's own creation time
/// is listed, with the record's name, key and pool, which a keyed actor shares across
/// incarnations.
fn destroyed_entry_json(
	data: &wire::ActorData,
	create_unix_ms: i64,
	destroy_unix_ms: i64,
) -> Result<Value> {
	let mut actor = actor_json(data, Some(destroy_unix_ms))?;
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
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
	let Some(actor) = read_actor(&ctx, actor_id)
		.await?
		.filter(|actor| !actor.destroyed && actor.data.namespace.as_str() == namespace)
	else {
		// While a V2 namespace migrates in, V2 may still hold it.
		let dynamic = migrate_from_v2::snapshot(&ctx);
		if let Some(config) = migrate_from_v2::applies(dynamic.as_deref(), &namespace) {
			let id = actor_id.to_string();
			let found = v2_actors(&ctx, config, &[("actor_ids", &id)]).await?;
			if let Some(actor) = found.into_iter().next() {
				return Ok(Json(json!({ "actor": actor })));
			}
		}
		return Err(ActorNotFound.build().into());
	};

	Ok(Json(json!({ "actor": actor_json(&actor.data, None)? })))
}

pub async fn destroy_actor(
	State(ctx): State<EngineContext>,
	Path(actor_id): Path<String>,
	Query(query): Query<NamespaceQuery>,
) -> Result<Response, ApiError> {
	let namespace = namespace(&ctx, query.namespace.as_deref()).await?;
	let actor_id = Id::parse(&actor_id)?;
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

	let res = match ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::new(actor_id).volume_key(), ())
		.await
	{
		Ok(res) => res,
		Err(error) if is_volume_gone(&error) => return Err(ActorNotFound.build().into()),
		Err(error) => return Err(error.into()),
	};
	let (Some(data), Some(phase)) = (res.data, res.phase) else {
		return Err(ActorNotFound.build().into());
	};
	if data.namespace.as_str() != namespace {
		return Err(ActorNotFound.build().into());
	}

	if let Some(event) = reschedule_event(&phase, res.epoch) {
		ctx.depot()
			.command::<ActorEvent>(&ActorVolume::new(actor_id).volume_key(), event)
			.await?;
	}

	Ok(Json(json!({})))
}

/// The event that reschedules an actor in `phase`, or `None` when there is nothing to move.
///
/// The `Reallocate` carries the epoch the read saw, so an actor that moved on since the read
/// ignores it instead of being rescheduled twice.
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
		// V2 serves a held actor, so there is nothing here to move.
		wire::ActorPhase::MigratingFromV2(_) => None,
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
	namespace: &str,
	actor_id: Id,
	event: wire::ActorEvent,
	v2_call: (Method, String),
) -> Result<Response> {
	let dynamic = migrate_from_v2::snapshot(ctx);
	if let Some(config) = migrate_from_v2::applies(dynamic.as_deref(), namespace) {
		match migrate_from_v2::read_v3_actor(ctx, actor_id).await? {
			V3Actor::Active => {}
			V3Actor::Held => return Err(ActorMigratingFromV2::error()),
			V3Actor::None => {
				let (method, path) = v2_call;
				let path = format!("{path}?{}", v2_namespace_query(config)?);
				let body = if method == Method::POST {
					Some(Bytes::from_static(b"{}"))
				} else {
					None
				};
				let response = proxy::api(ctx, config, method, &path, body).await?;
				return Ok(response.into_response());
			}
		}
	}
	if read_actor(ctx, actor_id)
		.await?
		.is_none_or(|actor| actor.data.namespace.as_str() != namespace)
	{
		return Err(ActorNotFound.build());
	}

	ctx.depot()
		.command::<ActorEvent>(&ActorVolume::new(actor_id).volume_key(), event)
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
		&namespace,
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
		pegboard::catalog::pool(&ctx, &namespace, &name, wire::PoolOperation::PoolRead).await?;
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
	let reply =
		pegboard::catalog::pool(&ctx, &namespace, &name, wire::PoolOperation::PoolDelete).await?;
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

async fn read(ctx: &EngineContext, actor_id: Id) -> Result<Read> {
	let res = ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::new(actor_id).volume_key(), ())
		.await;
	let res = match res {
		Ok(res) => res,
		Err(error) if is_volume_gone(&error) => return Ok(Read::Gone),
		Err(error) => return Err(error),
	};
	let destroyed = matches!(
		res.phase,
		Some(wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed)
	);
	let held = matches!(res.phase, Some(wire::ActorPhase::MigratingFromV2(_)));
	Ok(match res.data {
		Some(data) => Read::Actor(ReadActor {
			data,
			destroyed,
			held,
		}),
		None => Read::NoData,
	})
}

/// Reads an actor. `None` is an actor with no state volume or no data record yet.
pub(super) async fn read_actor(ctx: &EngineContext, actor_id: Id) -> Result<Option<ReadActor>> {
	Ok(match read(ctx, actor_id).await? {
		Read::Actor(actor) => Some(actor),
		Read::NoData | Read::Gone => None,
	})
}

/// An actor in main's public shape. v3 does not record when an actor started, became
/// connectable, slept or waited for an allocation, so those fields are null.
pub fn actor_json(actor: &wire::ActorData, destroy_unix_ms: Option<i64>) -> Result<Value> {
	Ok(json!({
		"actor_id": Id::from_slice(&actor.actor_id)?.to_string(),
		"name": actor.name,
		"key": actor.key.as_deref().map(actor_key::to_slash).unwrap_or_default(),
		"namespace": actor.namespace,
		"pool": actor.pool_name,
		"create_ts": actor.create_unix_ms,
		"start_ts": null,
		"pending_allocation_ts": null,
		"connectable_ts": null,
		"sleep_ts": null,
		"reschedule_ts": null,
		"destroy_ts": destroy_unix_ms,
	}))
}
