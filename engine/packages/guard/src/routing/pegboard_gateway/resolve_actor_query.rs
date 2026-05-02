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

pub enum ResolveQueryActorResult {
	Found { actor_id: Id },
	Forward { dc_label: u16 },
}

/// Resolve a parsed query gateway path to a concrete actor ID.
///
/// Dispatches to the appropriate resolution strategy based on the query method
/// (Get or GetOrCreate).
pub async fn resolve_query(
	ctx: &StandaloneCtx,
	query: &QueryActorQuery,
) -> Result<ResolveQueryActorResult> {
	match query {
		QueryActorQuery::Get {
			namespace,
			name,
			key,
			..
		} => resolve_query_get(ctx, namespace, name, key).await,
		QueryActorQuery::GetOrCreate {
			namespace,
			name,
			pool_name,
			key,
			input,
			region,
			crash_policy,
			..
		} => {
			resolve_query_get_or_create(
				ctx,
				namespace,
				name,
				pool_name,
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
async fn get_actor_for_key(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	name: &str,
	serialized_key: &str,
	pool_name: Option<&str>,
) -> Result<Option<ResolveQueryActorResult>> {
	// Get the reservation ID for this key
	let res = ctx
		.op(pegboard::ops::actor::get_for_key::Input {
			namespace_id: namespace_id,
			name: name.to_string(),
			key: serialized_key.to_string(),
			pool_name: pool_name.map(|x| x.to_string()),
			fetch_error: true,
		})
		.await?;

	match res {
		pegboard::ops::actor::get_for_key::Output::Found { actor } => {
			Ok(Some(ResolveQueryActorResult::Found {
				actor_id: actor.actor_id,
			}))
		}
		pegboard::ops::actor::get_for_key::Output::NotFound => Ok(None),
		pegboard::ops::actor::get_for_key::Output::Forward { dc_label } => {
			Ok(Some(ResolveQueryActorResult::Forward { dc_label }))
		}
	}
}

/// Resolve a "get" query to an existing actor ID. Returns an error if no actor
/// matches the given key.
async fn resolve_query_get(
	ctx: &StandaloneCtx,
	namespace_name: &str,
	name: &str,
	key: &[String],
) -> Result<ResolveQueryActorResult> {
	let namespace_id = resolve_namespace_id(ctx, namespace_name).await?;
	let serialized_key = serialize_actor_key(key)?;

	get_actor_for_key(ctx, namespace_id, name, &serialized_key, None)
		.await?
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())
}

/// Resolve a "getOrCreate" query. Tries to find an existing actor by key first,
/// then creates one if none exists. Handles duplicate-key races by retrying the
/// lookup after a failed create.
async fn resolve_query_get_or_create(
	ctx: &StandaloneCtx,
	namespace_name: &str,
	name: &str,
	pool_name: &str,
	key: &[String],
	input: Option<&[u8]>,
	region: Option<&str>,
	crash_policy: CrashPolicy,
) -> Result<ResolveQueryActorResult> {
	let namespace_id = resolve_namespace_id(ctx, namespace_name).await?;
	let serialized_key = serialize_actor_key(key)?;

	if let Some(res) =
		get_actor_for_key(ctx, namespace_id, name, &serialized_key, Some(pool_name)).await?
	{
		return Ok(res);
	}

	let target_dc_label =
		resolve_query_target_dc_label(ctx, namespace_id, namespace_name, pool_name, region).await?;
	let encoded_input = input.map(|input| STANDARD.encode(input));

	if target_dc_label == ctx.config().dc_label() {
		let actor_id = Id::new_v1(target_dc_label);
		match ctx
			.op(pegboard::ops::actor::create::Input {
				actor_id,
				namespace_id,
				name: name.to_string(),
				key: Some(serialized_key.clone()),
				runner_name_selector: pool_name.to_string(),
				crash_policy,
				input: encoded_input,
				forward_request: true,
				datacenter_name: None,
			})
			.await
		{
			Ok(res) => Ok(ResolveQueryActorResult::Found {
				actor_id: res.actor.actor_id,
			}),
			Err(err) if is_duplicate_key_error(&err) => {
				get_actor_for_key(ctx, namespace_id, name, &serialized_key, Some(pool_name))
					.await?
					.ok_or_else(|| pegboard::errors::Actor::NotFound.build())
			}
			Err(err) => Err(err),
		}
	} else {
		Ok(ResolveQueryActorResult::Forward {
			dc_label: target_dc_label,
		})
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

	// Return nearest enabled dc
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
	const KEY_SEPARATOR: &str = "/";
	const KEY_SEPARATOR_CHAR: char = '/';

	if key.is_empty() {
		return Ok(EMPTY_KEY.to_string());
	}

	let mut escaped_parts = Vec::with_capacity(key.len());
	for part in key {
		if part.is_empty() {
			escaped_parts.push(String::from("\\0"));
			continue;
		}

		let escaped = part
			.replace('\\', "\\\\")
			.replace(KEY_SEPARATOR_CHAR, "\\/");
		escaped_parts.push(escaped);
	}

	Ok(escaped_parts.join(KEY_SEPARATOR))
}

fn is_duplicate_key_error(err: &anyhow::Error) -> bool {
	err.chain()
		.find_map(|x| x.downcast_ref::<rivet_error::RivetError>())
		.map(|err| err.group() == "actor" && err.code() == "duplicate_key")
		.unwrap_or(false)
}

// Tests are inline because `serialize_actor_key` is a private function and
// cannot be reached from the integration `tests/` directory without widening
// visibility. Keep these in sync with the TypeScript suite at
// `rivetkit-typescript/packages/rivetkit/src/actor/keys.test.ts`.
#[cfg(test)]
mod tests {
	use super::*;

	fn s(parts: &[&str]) -> Vec<String> {
		parts.iter().map(|p| (*p).to_string()).collect()
	}

	#[test]
	fn serializes_empty_array_as_empty_key_sentinel() {
		assert_eq!(serialize_actor_key(&[]).unwrap(), "/");
	}

	#[test]
	fn serializes_single_part_unchanged() {
		assert_eq!(serialize_actor_key(&s(&["test"])).unwrap(), "test");
	}

	#[test]
	fn serializes_multiple_parts_separated_by_slash() {
		assert_eq!(serialize_actor_key(&s(&["a", "b", "c"])).unwrap(), "a/b/c");
	}

	#[test]
	fn escapes_slash_in_part() {
		assert_eq!(serialize_actor_key(&s(&["a/b"])).unwrap(), "a\\/b");
	}

	#[test]
	fn escapes_slash_in_part_with_neighbors() {
		assert_eq!(serialize_actor_key(&s(&["a/b", "c"])).unwrap(), "a\\/b/c");
	}

	#[test]
	fn escapes_part_equal_to_separator() {
		assert_eq!(serialize_actor_key(&s(&["/"])).unwrap(), "\\/");
	}

	#[test]
	fn handles_empty_string_part_with_marker() {
		assert_eq!(serialize_actor_key(&s(&[""])).unwrap(), "\\0");
	}

	#[test]
	fn empty_string_marker_is_distinct_from_empty_key_sentinel() {
		assert_ne!(
			serialize_actor_key(&[]).unwrap(),
			serialize_actor_key(&s(&[""])).unwrap()
		);
	}

	#[test]
	fn escapes_backslash_before_separator() {
		assert_eq!(serialize_actor_key(&s(&["a\\b"])).unwrap(), "a\\\\b");
	}

	#[test]
	fn handles_mixed_empty_and_escaped_parts() {
		assert_eq!(
			serialize_actor_key(&s(&["a/b", "", "c/d"])).unwrap(),
			"a\\/b/\\0/c\\/d"
		);
	}
}
