//! Resolves query gateway paths to concrete actor IDs.
//!
//! This module handles the "get" and "getOrCreate" query methods by looking up
//! or creating actors through the engine's ops layer. It mirrors the TypeScript
//! resolution in `rivetkit-typescript/packages/rivetkit/src/manager/gateway.ts`
//! (`resolveQueryActorId`).

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use gas::prelude::*;
use rivet_types::actors::CrashPolicy;

use crate::routing::actor_path::QueryActorQuery;

/// Resolve a parsed query gateway path to a concrete actor ID.
///
/// Dispatches to the appropriate resolution strategy based on the query method
/// (Get or GetOrCreate).
pub async fn resolve_query_actor_id(ctx: &StandaloneCtx, query: &QueryActorQuery) -> Result<Id> {
	match query {
		QueryActorQuery::Get {
			namespace,
			name,
			key,
		} => resolve_query_get_actor_id(ctx, namespace, name, key).await,
		QueryActorQuery::GetOrCreate {
			namespace,
			name,
			runner_name,
			key,
			input,
			region,
			crash_policy,
		} => {
			resolve_query_get_or_create_actor_id(
				ctx,
				namespace,
				name,
				runner_name,
				key,
				input.as_deref(),
				region.as_deref(),
				crash_policy.unwrap_or(CrashPolicy::Sleep),
			)
			.await
		}
	}
}

/// Resolve a namespace name to its ID via the namespace ops layer.
async fn resolve_namespace_id(ctx: &StandaloneCtx, namespace_name: &str) -> Result<Id> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: namespace_name.to_string(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;
	Ok(namespace.namespace_id)
}

/// Look up an existing actor by key. Returns `None` if no actor matches.
async fn get_actor_id_for_key(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	name: &str,
	serialized_key: &str,
) -> Result<Option<Id>> {
	let existing = ctx
		.op(pegboard::ops::actor::get_for_key::Input {
			namespace_id,
			name: name.to_string(),
			key: serialized_key.to_string(),
			fetch_error: true,
		})
		.await?;
	Ok(existing.actor.map(|a| a.actor_id))
}

/// Resolve a "get" query to an existing actor ID. Returns an error if no actor
/// matches the given key.
async fn resolve_query_get_actor_id(
	ctx: &StandaloneCtx,
	namespace_name: &str,
	name: &str,
	key: &[String],
) -> Result<Id> {
	let namespace_id = resolve_namespace_id(ctx, namespace_name).await?;
	let serialized_key = serialize_actor_key(key)?;

	get_actor_id_for_key(ctx, namespace_id, name, &serialized_key)
		.await?
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())
}

/// Resolve a "getOrCreate" query. Tries to find an existing actor by key first,
/// then creates one if none exists. Handles duplicate-key races by retrying the
/// lookup after a failed create.
async fn resolve_query_get_or_create_actor_id(
	ctx: &StandaloneCtx,
	namespace_name: &str,
	name: &str,
	runner_name: &str,
	key: &[String],
	input: Option<&[u8]>,
	region: Option<&str>,
	crash_policy: CrashPolicy,
) -> Result<Id> {
	let namespace_id = resolve_namespace_id(ctx, namespace_name).await?;
	let serialized_key = serialize_actor_key(key)?;

	if let Some(actor_id) = get_actor_id_for_key(ctx, namespace_id, name, &serialized_key).await? {
		return Ok(actor_id);
	}

	let target_dc_label =
		resolve_query_target_dc_label(ctx, namespace_id, namespace_name, runner_name, region)
			.await?;
	let encoded_input = input.map(|input| STANDARD.encode(input));

	if target_dc_label == ctx.config().dc_label() {
		let actor_id = Id::new_v1(target_dc_label);
		match ctx
			.op(pegboard::ops::actor::create::Input {
				actor_id,
				namespace_id,
				name: name.to_string(),
				key: Some(serialized_key.clone()),
				runner_name_selector: runner_name.to_string(),
				crash_policy,
				input: encoded_input,
				forward_request: true,
				datacenter_name: None,
			})
			.await
		{
			Ok(res) => Ok(res.actor.actor_id),
			Err(err) if is_duplicate_key_error(&err) => {
				get_actor_id_for_key(ctx, namespace_id, name, &serialized_key)
					.await?
					.ok_or_else(|| pegboard::errors::Actor::NotFound.build())
			}
			Err(err) => Err(err),
		}
	} else {
		let response = rivet_api_util::request_remote_datacenter::<
			rivet_api_types::actors::get_or_create::GetOrCreateResponse,
		>(
			ctx.config(),
			target_dc_label,
			"/actors",
			rivet_api_util::Method::PUT,
			Some(&rivet_api_types::actors::get_or_create::GetOrCreateQuery {
				namespace: namespace_name.to_string(),
			}),
			Some(
				&rivet_api_types::actors::get_or_create::GetOrCreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: serialized_key,
					input: encoded_input,
					runner_name_selector: runner_name.to_string(),
					crash_policy,
				},
			),
		)
		.await?;
		Ok(response.actor.actor_id)
	}
}

/// Determine which datacenter to target for actor creation. Uses the explicit
/// region if provided, otherwise picks the first datacenter that has the runner
/// config enabled.
async fn resolve_query_target_dc_label(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	namespace_name: &str,
	runner_name_selector: &str,
	region: Option<&str>,
) -> Result<u16> {
	if let Some(region) = region {
		return Ok(ctx
			.config()
			.dc_for_name(region)
			.ok_or_else(|| rivet_api_util::errors::Datacenter::NotFound.build())?
			.datacenter_label);
	}

	let res = ctx
		.op(
			pegboard::ops::runner::list_runner_config_enabled_dcs::Input {
				namespace_id,
				runner_name: runner_name_selector.to_string(),
			},
		)
		.await?;

	if let Some(dc_label) = res.dc_labels.into_iter().next() {
		Ok(dc_label)
	} else {
		Err(pegboard::errors::Actor::NoRunnerConfigConfigured {
			namespace: namespace_name.to_string(),
			pool_name: runner_name_selector.to_string(),
		}
		.build())
	}
}

fn serialize_actor_key(key: &[String]) -> Result<String> {
	const EMPTY_KEY: &str = "/";
	const KEY_SEPARATOR: char = '/';

	if key.is_empty() {
		return Ok(EMPTY_KEY.to_string());
	}

	let mut escaped_parts = Vec::with_capacity(key.len());
	for part in key {
		if part.is_empty() {
			escaped_parts.push(String::from("\\0"));
			continue;
		}

		let escaped = part.replace('\\', "\\\\").replace(KEY_SEPARATOR, "\\/");
		escaped_parts.push(escaped);
	}

	Ok(escaped_parts.join(EMPTY_KEY))
}

fn is_duplicate_key_error(err: &anyhow::Error) -> bool {
	err.chain()
		.find_map(|x| x.downcast_ref::<rivet_error::RivetError>())
		.map(|err| err.group() == "actor" && err.code() == "duplicate_key")
		.unwrap_or(false)
}
