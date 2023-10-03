use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-namespace-resolve-url")]
async fn handle(
	ctx: OperationContext<game::namespace_resolve_url::Request>,
) -> GlobalResult<game::namespace_resolve_url::Response> {
	// Parse the URL
	let url = url::Url::parse(&ctx.url)?;
	let domain = match url.domain() {
		Some(x) => x,
		None => {
			tracing::info!(%url, "missing domain");
			return Ok(game::namespace_resolve_url::Response { resolution: None });
		}
	};

	// Attempt to resolve the URL by the rivet.game subdomain or the custom CDN
	// domain
	let mut resolution = resolve_rivet_game_subdomain(&ctx, domain).await?;
	if resolution.is_none() {
		resolution = resolve_custom_domain(&ctx, domain).await?;
	}

	Ok(game::namespace_resolve_url::Response { resolution })
}

/// Attempt to resolve from a rivet.game subdomain.
#[tracing::instrument]
async fn resolve_rivet_game_subdomain(
	ctx: &OperationContext<game::namespace_resolve_url::Request>,
	domain: &str,
) -> GlobalResult<Option<game::namespace_resolve_url::response::Resolution>> {
	let domain_cdn = internal_unwrap_owned!(util::env::domain_cdn());
	let strip_suffix = format!(".{domain_cdn}");
	tracing::info!(%domain, %strip_suffix, "attempting to strip base domain");
	let specifier = if let Some(x) = domain.strip_suffix(&strip_suffix) {
		x
	} else {
		tracing::info!(
			%domain,
			"base component is not {domain_cdn} or {{ns}}.{domain_cdn}",
		);
		return Ok(None);
	};
	if specifier.contains('.') {
		tracing::info!(%specifier, "domain has extra components");
		return Ok(None);
	}

	parse_game_and_namespace(ctx, specifier).await
}

#[tracing::instrument]
async fn parse_game_and_namespace(
	ctx: &OperationContext<game::namespace_resolve_url::Request>,
	specifier: &str,
) -> GlobalResult<Option<game::namespace_resolve_url::response::Resolution>> {
	// Split the game and namespace IDs
	let (game_name_id, ns_name_id) = specifier.split_once("--").unwrap_or((specifier, "prod"));

	let game_resolve_res = op!([ctx] game_resolve_name_id {
		name_ids: vec![game_name_id.to_string()],
	})
	.await?;
	let game_id = match game_resolve_res.games.first() {
		Some(x) => internal_unwrap!(x.game_id).as_uuid(),
		None => {
			tracing::info!(%game_name_id, "game with name id does not exist");
			return Ok(None);
		}
	};

	let ns_resolve_res = op!([ctx] game_namespace_resolve_name_id {
		game_id: Some(game_id.into()),
		name_ids: vec![ns_name_id.to_string()],
	})
	.await?;
	let ns_id = match ns_resolve_res.namespaces.first() {
		Some(x) => internal_unwrap!(x.namespace_id).as_uuid(),
		None => {
			tracing::info!(%game_name_id, %game_id, %ns_name_id, "namespace with name id does not exist for game");
			return Ok(None);
		}
	};

	Ok(Some(game::namespace_resolve_url::response::Resolution {
		game_id: Some(game_id.into()),
		namespace_id: Some(ns_id.into()),
	}))
}

#[tracing::instrument]
async fn resolve_custom_domain(
	ctx: &OperationContext<game::namespace_resolve_url::Request>,
	domain: &str,
) -> GlobalResult<Option<game::namespace_resolve_url::response::Resolution>> {
	let resolve_res = op!([ctx] cdn_namespace_resolve_domain {
		domains: vec![domain.into()],
	})
	.await?;
	let namespace_id = if let Some(domain) = resolve_res.namespaces.first() {
		internal_unwrap!(domain.namespace_id).as_uuid()
	} else {
		tracing::info!(%domain, "no matching cdn domain");
		return Ok(None);
	};

	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns_data = ns_res.namespaces.first();
	let ns_data = internal_unwrap!(ns_data, "missing matching game namespace for cdn domain");
	let game_id = internal_unwrap!(ns_data.game_id).as_uuid();

	Ok(Some(game::namespace_resolve_url::response::Resolution {
		game_id: Some(game_id.into()),
		namespace_id: Some(namespace_id.into()),
	}))
}
