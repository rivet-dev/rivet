use std::time::Duration;

use chirp_workflow::prelude::*;
use cluster::types::GuardPublicHostname;
use fdb_util::{FormalKey, SNAPSHOT};
use foundationdb::{self as fdb};
use global_error::GlobalResult;
use pegboard::types::EndpointType;
use pegboard::util::build_actor_hostname_and_path_regex;
use rivet_config::config::AccessKind;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};
use uuid::Uuid;

/// Route requests to actor services based on hostname and path
#[tracing::instrument(skip_all)]
pub async fn route_actor_request(
	ctx: &StandaloneCtx,
	host: &str,
	path: &str,
	dc_id: Uuid,
) -> GlobalResult<Option<RoutingOutput>> {
	// Get DC information for the current datacenter
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	// HACK: If in local dev, force the hostname to be 127.0.0.1 since it may be routed to via a
	// different hostname (e.g. localhost, rivet-guard)
	let host = match (
		&ctx.config().server()?.rivet.auth.access_kind,
		&dc.guard_public_hostname,
	) {
		(AccessKind::Development, GuardPublicHostname::Static(dc_hostname)) => &dc_hostname,
		_ => host,
	};

	// Get the guard public hostname from the datacenter config
	let guard_public_hostname = &dc.guard_public_hostname;

	// Try routing with Hostname endpoint type
	if let Some(routing_result) = try_route_with_endpoint_type(
		ctx,
		host,
		path,
		EndpointType::Hostname,
		guard_public_hostname,
	)
	.await?
	{
		return Ok(Some(routing_result));
	}

	// Try routing with Path endpoint type
	if let Some(routing_result) =
		try_route_with_endpoint_type(ctx, host, path, EndpointType::Path, guard_public_hostname)
			.await?
	{
		return Ok(Some(routing_result));
	}

	// No matching route found
	Ok(None)
}

/// Try to route the request using the specified endpoint type
#[tracing::instrument(skip_all, fields(?endpoint_type, ?guard_public_hostname))]
async fn try_route_with_endpoint_type(
	ctx: &StandaloneCtx,
	hostname: &str,
	path: &str,
	endpoint_type: EndpointType,
	guard_public_hostname: &GuardPublicHostname,
) -> GlobalResult<Option<RoutingOutput>> {
	// Build regexes for the endpoint type
	let (hostname_regex, path_regex) =
		match build_actor_hostname_and_path_regex(endpoint_type, guard_public_hostname) {
			Ok(Some(regexes)) => regexes,
			Ok(None) => return Ok(None),
			Err(e) => {
				tracing::error!(
					?endpoint_type,
					"Failed to build actor hostname and path regex: {}",
					e
				);
				return Ok(None);
			}
		};

	// Check if hostname matches the pattern
	if !hostname_regex.is_match(hostname) {
		return Ok(None);
	}

	// Extract actor_id and port_name based on the endpoint type
	let (actor_id, port_name) = match endpoint_type {
		EndpointType::Hostname => {
			// For hostname-based routing, extract from hostname
			if let Some(captures) = hostname_regex.captures(hostname) {
				match (captures.name("actor_id"), captures.name("port_name")) {
					(Some(actor_id), Some(port_name)) => match Uuid::parse_str(actor_id.as_str()) {
						Ok(actor_id) => (actor_id, port_name.as_str().to_string()),
						Err(_) => return Ok(None),
					},
					_ => return Ok(None),
				}
			} else {
				return Ok(None);
			}
		}
		EndpointType::Path => {
			// For path-based routing, verify hostname and extract from path
			if !hostname_regex.is_match(hostname) {
				return Ok(None);
			}

			// Get the path_regex (should exist for path-based routing)
			let path_regex = match path_regex {
				Some(re) => re,
				None => {
					tracing::error!("Path regex is missing for path-based routing");
					return Ok(None);
				}
			};

			if let Some(captures) = path_regex.captures(path) {
				match (captures.name("actor_id"), captures.name("port_name")) {
					(Some(actor_id), Some(port_name)) => match Uuid::parse_str(actor_id.as_str()) {
						Ok(actor_id) => (actor_id, port_name.as_str().to_string()),
						Err(_) => return Ok(None),
					},
					_ => return Ok(None),
				}
			} else {
				return Ok(None);
			}
		}
	};

	// Build the path for the route target based on endpoint type
	let path_to_forward = match endpoint_type {
		EndpointType::Hostname => path.to_string(),
		EndpointType::Path => {
			// For path-based routing, we need to remove the actor prefix from the path
			let prefix = format!("/{}-{}", actor_id, port_name);
			if path.starts_with(&prefix) {
				if path.len() > prefix.len() {
					path[prefix.len()..].to_string()
				} else {
					"/".to_string()
				}
			} else {
				path.to_string()
			}
		}
	};

	// Now that we have the actor_id and port_name, lookup the actor
	match find_actor(ctx, &actor_id, &port_name, path_to_forward).await? {
		Some(target) => Ok(Some(RoutingOutput::Route(RouteConfig {
			targets: vec![target],
			timeout: RoutingTimeout {
				routing_timeout: 10,
			},
		}))),
		None => Ok(None),
	}
}

/// Find an actor by actor_id and port_name - this would call into the actor registry
#[tracing::instrument(skip_all, fields(?actor_id, %port_name, %path_to_forward))]
async fn find_actor(
	ctx: &StandaloneCtx,
	actor_id: &Uuid,
	port_name: &str,
	path_to_forward: String,
) -> GlobalResult<Option<RouteTarget>> {
	let actor_exists = tokio::time::timeout(
		Duration::from_secs(5),
		ctx.fdb()
			.await?
			.run(|tx, _mc| async move {
				let create_ts_key = pegboard::keys::actor::CreateTsKey::new(*actor_id);
				let exists = tx
					.get(&pegboard::keys::subspace().pack(&create_ts_key), SNAPSHOT)
					.await?
					.is_some();

				Ok(exists)
			})
			.custom_instrument(tracing::info_span!("actor_exists_tx")),
	)
	.await??;

	if !actor_exists {
		return Ok(None);
	}

	// Create subs before checking for proxied ports
	let mut ready_sub = ctx
		.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", actor_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<pegboard::workflows::actor::Failed>(("actor_id", actor_id))
		.await?;
	let mut destroy_sub = ctx
		.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id))
		.await?;

	let proxied_ports = if let Some(proxied_ports) =
		tokio::time::timeout(Duration::from_secs(5), fetch_proxied_ports(ctx, actor_id)).await??
	{
		proxied_ports
	} else {
		tracing::info!(?actor_id, "waiting for actor to become ready");

		// Wait for ready, fail, or destroy
		tokio::select! {
			res = ready_sub.next() => { res?; },
			res = fail_sub.next() => {
				let msg = res?;
				bail_with!(ACTOR_FAILED_TO_CREATE, error = msg.message);
			}
			res = destroy_sub.next() => {
				res?;
				bail_with!(ACTOR_FAILED_TO_CREATE, error = "Actor failed before reaching a ready state.");
			}
			// Ready timeout
			_ = tokio::time::sleep(Duration::from_secs(15)) => {
				return Ok(None);
			}
		}

		// Fetch again after ready
		let Some(proxied_ports) =
			tokio::time::timeout(Duration::from_secs(5), fetch_proxied_ports(ctx, actor_id))
				.await??
		else {
			return Ok(None);
		};

		proxied_ports
	};

	tracing::info!(?actor_id, "actor ready");

	// Find the port
	let Some(proxied_port) = proxied_ports.iter().find(|pp| pp.port_name == port_name) else {
		// TODO: Return error port not found
		return Ok(None);
	};

	// TODO: Validate protocol based on the incoming port

	Ok(Some(RouteTarget {
		actor_id: Some(*actor_id),
		server_id: None,
		host: proxied_port.lan_hostname.parse()?,
		port: proxied_port.source,
		path: path_to_forward,
	}))
}

#[tracing::instrument(skip_all, fields(?actor_id))]
async fn fetch_proxied_ports(
	ctx: &StandaloneCtx,
	actor_id: &Uuid,
) -> GlobalResult<Option<Vec<pegboard::keys::actor::ProxiedPort>>> {
	// Fetch ports
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let proxied_ports_key = pegboard::keys::actor::ProxiedPortsKey::new(*actor_id);
			let raw = tx
				.get(
					&pegboard::keys::subspace().pack(&proxied_ports_key),
					// NOTE: This is not SERIALIZABLE because we don't want to conflict with port updates
					// and its not important if its slightly stale
					SNAPSHOT,
				)
				.await?;
			if let Some(raw) = raw {
				Ok(Some(proxied_ports_key.deserialize(&raw).map_err(|x| {
					fdb::FdbBindingError::CustomError(x.into())
				})?))
			} else {
				Ok(None)
			}
		})
		.custom_instrument(tracing::info_span!("fetch_proxied_ports_tx"))
		.await
		.map_err(Into::into)
}
