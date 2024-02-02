use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
	collections::{hash_map::DefaultHasher, HashMap},
	convert::TryInto,
	fmt::Write,
	hash::{Hash, Hasher},
};
use util::glob::Traefik;

use crate::{auth::Auth, route::traefik};

const BASE_ROUTER_PRIORITY: usize = 100;
const HTML_ROUTER_PRIORITY: usize = 150;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuery {
	token: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	ConfigQuery { token }: ConfigQuery,
) -> GlobalResult<super::TraefikConfigResponseNullified> {
	ensure_eq_with!(
		token,
		util::env::read_secret(&["rivet", "api_route", "token"]).await?,
		API_FORBIDDEN,
		reason = "Invalid token"
	);

	// Fetch configs and catch any errors
	let config = build_cdn(&ctx).await?;

	// tracing::info!(
	// 	http_services = ?config.http.services.len(),
	// 	http_routers = config.http.routers.len(),
	// 	http_middlewares = ?config.http.middlewares.len(),
	// 	tcp_services = ?config.tcp.services.len(),
	// 	tcp_routers = config.tcp.routers.len(),
	// 	tcp_middlewares = ?config.tcp.middlewares.len(),
	// 	udp_services = ?config.udp.services.len(),
	// 	udp_routers = config.udp.routers.len(),
	// 	udp_middlewares = ?config.udp.middlewares.len(),
	// 	"traefik config"
	// );

	Ok(super::TraefikConfigResponseNullified {
		http: config.http.nullified(),
		tcp: config.tcp.nullified(),
		udp: config.udp.nullified(),
	})
}

/// Builds configuration for CDN routes.
#[tracing::instrument(skip(ctx))]
pub async fn build_cdn(ctx: &Ctx<Auth>) -> GlobalResult<traefik::TraefikConfigResponse> {
	let mut config = traefik::TraefikConfigResponse::default();
	let s3_client = s3_util::Client::from_env("bucket-cdn").await?;

	let redis_cdn = ctx.op_ctx().redis_cdn().await?;
	let cdn_fetch = fetch_cdn(redis_cdn).await?;

	// Process namespaces
	for ns in &cdn_fetch {
		let register_res = register_namespace(ns, &mut config, &s3_client);
		match register_res {
			Ok(_) => {}
			Err(err) => tracing::error!(?err, ?ns, "failed to register namespace route"),
		}
	}

	// Register common middleware
	//
	// Many of these are the same as the `cdn` middleware chain in the Traefik
	// file configuration.
	config.http.middlewares.insert(
		"cdn-in-flight".to_owned(),
		traefik::TraefikMiddlewareHttp::InFlightReq {
			// This number needs to be high to allow for parallel requests
			amount: 128,
			source_criterion: traefik::InFlightReqSourceCriterion::RequestHeaderName(
				if util::env::dns_provider() == Some("cloudflare") {
					"cf-connecting-ip".to_string()
				} else {
					"x-forwarded-for".to_string()
				},
			),
		},
	);
	config.http.middlewares.insert(
		"cdn-retry".to_owned(),
		traefik::TraefikMiddlewareHttp::Retry {
			attempts: 4,
			initial_interval: "1s".into(),
		},
	);
	config.http.middlewares.insert(
		"cdn-compress".to_owned(),
		traefik::TraefikMiddlewareHttp::Compress {},
	);

	let base_headers = {
		let mut x = HashMap::new();
		// Allow embedding in iframes and cross-site requests
		x.insert("Content-Security-Policy".to_owned(), String::new());
		x.insert("X-Frame-Options".to_owned(), String::new());
		x.insert("X-XSS-Protection".to_owned(), String::new());
		x
	};

	config.http.middlewares.insert(
		"cdn-cache-control".to_owned(),
		traefik::TraefikMiddlewareHttp::Headers(traefik::TraefikMiddlewareHeaders {
			custom_response_headers: Some({
				let mut x = base_headers.clone();
				// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#caching_static_assets
				// and https://imagekit.io/blog/ultimate-guide-to-http-caching-for-static-assets/
				x.insert(
					"Cache-Control".to_owned(),
					"public, max-age=604800, immutable".to_owned(),
				);
				x
			}),
			..Default::default()
		}),
	);

	config.http.middlewares.insert(
		"cdn-cache-control-html".to_owned(),
		traefik::TraefikMiddlewareHttp::Headers(traefik::TraefikMiddlewareHeaders {
			custom_response_headers: Some({
				let mut x = base_headers;
				// See above
				x.insert("Cache-Control".to_owned(), "no-cache, no-store".to_owned());
				x
			}),
			..Default::default()
		}),
	);

	config.http.middlewares.insert(
		"cdn-append-index".to_owned(),
		traefik::TraefikMiddlewareHttp::ReplacePathRegex {
			regex: "(.*)/$".into(),
			replacement: "${1}/index.html".into(),
		},
	);

	tracing::info!(
		services = ?config.http.services.len(),
		routers = config.http.routers.len(),
		middlewares = ?config.http.middlewares.len(),
		"cdn traefik config"
	);

	Ok(config)
}

#[tracing::instrument(skip(redis_cdn))]
async fn fetch_cdn(
	mut redis_cdn: RedisPool,
) -> GlobalResult<Vec<cdn::redis_cdn::NamespaceCdnConfig>> {
	let ns = redis_cdn
		.hvals::<_, Vec<Vec<u8>>>(util_cdn::key::ns_cdn_configs())
		.await?
		.into_iter()
		.filter_map(
			|buf| match cdn::redis_cdn::NamespaceCdnConfig::decode(buf.as_slice()) {
				Ok(x) => Some(x),
				Err(err) => {
					tracing::error!(?err, "failed to decode run NamespaceCdnConfig from redis");
					None
				}
			},
		)
		.collect::<Vec<_>>();

	Ok(ns)
}

#[tracing::instrument(skip(config, s3_client))]
fn register_namespace(
	ns: &cdn::redis_cdn::NamespaceCdnConfig,
	config: &mut traefik::TraefikConfigResponse,
	s3_client: &s3_util::Client,
) -> GlobalResult<()> {
	let Some(domain_cdn) = util::env::domain_cdn() else {
		return Ok(());
	};

	let ns_id = **unwrap_ref!(ns.namespace_id);
	let ns_auth = unwrap!(backend::cdn::namespace_config::AuthType::from_i32(
		ns.auth_type
	));

	// Create router rule
	let router_rule = {
		let mut router_rule = "Method(`GET`, `HEAD`) && (".to_string();

		// Match namespace
		write!(
			&mut router_rule,
			"Host(`{game_name_id}--{ns_name_id}.{domain_cdn}`)",
			game_name_id = ns.game_name_id,
			ns_name_id = ns.namespace_name_id,
		)?;

		// Match all production domains
		if ns.namespace_name_id == "prod" {
			write!(
				&mut router_rule,
				" || Host(`{game_name_id}.{domain_cdn}`)",
				game_name_id = ns.game_name_id,
			)?;
		}

		// Match all custom domains
		for domain in &ns.domains {
			write!(
				&mut router_rule,
				" || Host(`{domain}`)",
				domain = domain.domain
			)?;
		}

		write!(&mut router_rule, ")")?;

		router_rule
	};

	// Write rule that matches both paths ending with a slash and HTML files.
	// These paths will have different cache control settings and will append
	// index.html if needed.
	let router_rule_html = format!("({}) && Path(`/{{xyz:(.*/|.*\\.html|)$}}`)", router_rule);

	// Create middleware
	let rewrite_middleware_key = format!("ns-rewrite:{}", ns_id);
	let auth_middleware_key = format!("ns-auth:{}", ns_id);
	let router_middlewares_base = vec![
		"cdn-in-flight".into(),
		"cdn-retry".into(),
		"cdn-compress".into(),
		rewrite_middleware_key.clone(),
		auth_middleware_key.clone(),
	];

	// Don't add caching headers to static assets since it caches non-200 responses
	let router_middlewares_cdn = [router_middlewares_base.clone(), vec![]].concat();
	let router_middlewares_html = [
		router_middlewares_base,
		vec!["cdn-cache-control-html".into(), "cdn-append-index".into()],
	]
	.concat();

	let upload_id = unwrap_ref!(ns.upload_id);
	let service = "traffic-server-traffic-server@kubernetescrd";
	let path_prefix = format!("/s3-cache/{}/{}", s3_client.bucket(), *upload_id);

	// Create default routers
	{
		config.http.routers.insert(
			format!("ns:{}-insecure", ns_id),
			traefik::TraefikRouter {
				entry_points: vec!["web".into()],
				rule: Some(router_rule.clone()),
				priority: Some(BASE_ROUTER_PRIORITY),
				service: service.to_owned(),
				middlewares: router_middlewares_cdn.clone(),
				tls: None,
			},
		);
		config.http.routers.insert(
			format!("ns:{}-insecure-html", ns_id),
			traefik::TraefikRouter {
				entry_points: vec!["web".into()],
				rule: Some(router_rule_html.clone()),
				priority: Some(HTML_ROUTER_PRIORITY),
				service: service.to_owned(),
				middlewares: router_middlewares_html.clone(),
				tls: None,
			},
		);
		config.http.routers.insert(
			format!("ns:{}-secure", ns_id),
			traefik::TraefikRouter {
				entry_points: vec!["websecure".into()],
				rule: Some(router_rule),
				priority: Some(BASE_ROUTER_PRIORITY),
				service: service.to_owned(),
				middlewares: router_middlewares_cdn.clone(),
				tls: Some(traefik::TraefikTls::build_cloudflare()),
			},
		);
		config.http.routers.insert(
			format!("ns:{}-secure-html", ns_id),
			traefik::TraefikRouter {
				entry_points: vec!["websecure".into()],
				rule: Some(router_rule_html),
				priority: Some(HTML_ROUTER_PRIORITY),
				service: service.to_owned(),
				middlewares: router_middlewares_html.clone(),
				tls: Some(traefik::TraefikTls::build_cloudflare()),
			},
		);
	}

	// Create middleware
	config.http.middlewares.insert(
		rewrite_middleware_key,
		traefik::TraefikMiddlewareHttp::AddPrefix {
			prefix: path_prefix,
		},
	);

	let auth_middleware = match ns_auth {
		backend::cdn::namespace_config::AuthType::None => {
			// Removes the authorization header.
			//
			// This prevents getting a `SignatureDoesNotMatch` error from S3
			// when attempting to request a resource with cached auth headers.
			// This can happen immediately after signing in, disabling ns
			// authorization, then visiting the site again.
			traefik::TraefikMiddlewareHttp::Headers(traefik::TraefikMiddlewareHeaders {
				custom_request_headers: Some({
					let mut x = HashMap::new();
					x.insert("Authorization".to_owned(), String::new());
					x
				}),
				..Default::default()
			})
		}
		backend::cdn::namespace_config::AuthType::Basic => {
			traefik::TraefikMiddlewareHttp::BasicAuth {
				users: ns
					.auth_user_list
					.iter()
					.map(|user| format!("{}:{}", user.user, user.password))
					.collect::<Vec<_>>(),
				realm: Some("RivetCdn".to_string()),
				remove_header: true,
			}
		}
	};
	config
		.http
		.middlewares
		.insert(auth_middleware_key, auth_middleware);

	for route in &ns.routes {
		register_custom_cdn_route(
			ns,
			config,
			service,
			router_middlewares_cdn.clone(),
			router_middlewares_html.clone(),
			route,
		)?;
	}

	Ok(())
}

#[tracing::instrument(skip(config))]
fn register_custom_cdn_route(
	ns: &cdn::redis_cdn::NamespaceCdnConfig,
	config: &mut traefik::TraefikConfigResponse,
	service: &str,
	router_middlewares_cdn: Vec<String>,
	router_middlewares_html: Vec<String>,
	route: &backend::cdn::Route,
) -> GlobalResult<()> {
	let Some(domain_cdn) = util::env::domain_cdn() else {
		return Ok(());
	};

	let ns_id = **unwrap_ref!(ns.namespace_id);

	if let Some(glob) = route.glob.clone() {
		match TryInto::<util::glob::Glob>::try_into(glob) {
			Ok(glob) => {
				let traefik_glob = glob.as_traefik()?;

				let glob_hash = {
					let mut hasher = DefaultHasher::new();
					traefik_glob.hash(&mut hasher);
					hasher.finish()
				};

				// Create router rule
				let router_rule = {
					// Match all domains
					let mut router_rule = format!(
						"Host(`{game_name_id}--{ns_name_id}.{domain_cdn}`",
						game_name_id = ns.game_name_id,
						ns_name_id = ns.namespace_name_id,
					);

					// Match all production domains
					if ns.namespace_name_id == "prod" {
						write!(
							&mut router_rule,
							", `{game_name_id}.{domain_cdn}`",
							game_name_id = ns.game_name_id,
						)?;
					}

					// Match all custom domains
					for domain in &ns.domains {
						write!(&mut router_rule, ", `{domain}`", domain = domain.domain,)?;
					}

					// Match glob path
					write!(
						&mut router_rule,
						") && Path(`/{glob}`)",
						glob = traefik_glob
					)?;

					router_rule
				};

				// Write rule that matches both paths ending with a slash and HTML files.
				// These paths will have different cache control settings and will append
				// index.html if needed.
				let router_rule_html =
					format!("({}) && Path(`/{{xyz:(.*/|.*\\.html|)$}}`)", router_rule);

				// Add middleware
				let mut custom_headers_router_middlewares_cdn = router_middlewares_cdn;
				let mut custom_headers_router_middlewares_html = router_middlewares_html;
				for middleware in &route.middlewares {
					match &middleware.kind {
						Some(backend::cdn::middleware::Kind::CustomHeaders(custom_headers)) => {
							let custom_header_key =
								format!("ns-custom-headers:{}:{}", ns_id, glob_hash);

							// Create headers middleware
							let headers = traefik::TraefikMiddlewareHttp::Headers(
								traefik::TraefikMiddlewareHeaders {
									custom_response_headers: Some(
										custom_headers
											.headers
											.clone()
											.into_iter()
											.map(|header| (header.name, header.value))
											.collect::<HashMap<_, _>>(),
									),
									..Default::default()
								},
							);

							config
								.http
								.middlewares
								.insert(custom_header_key.clone(), headers);
							custom_headers_router_middlewares_cdn.push(custom_header_key.clone());
							custom_headers_router_middlewares_html.push(custom_header_key);
						}
						None => tracing::warn!(?middleware, "invalid middleware"),
					}
				}

				// Create routers
				config.http.routers.insert(
					format!("ns-custom-headers:{}-insecure:{}", ns_id, glob_hash),
					traefik::TraefikRouter {
						entry_points: vec!["web".into()],
						rule: Some(router_rule.clone()),
						priority: Some(
							(BASE_ROUTER_PRIORITY + 1).saturating_add(route.priority.try_into()?),
						),
						service: service.to_owned(),
						middlewares: custom_headers_router_middlewares_cdn.clone(),
						tls: None,
					},
				);
				config.http.routers.insert(
					format!("ns-custom-headers:{}-insecure-html:{}", ns_id, glob_hash),
					traefik::TraefikRouter {
						entry_points: vec!["web".into()],
						rule: Some(router_rule_html.clone()),
						priority: Some(
							(HTML_ROUTER_PRIORITY + 1).saturating_add(route.priority.try_into()?),
						),
						service: service.to_owned(),
						middlewares: custom_headers_router_middlewares_html.clone(),
						tls: None,
					},
				);
				config.http.routers.insert(
					format!("ns-custom-headers:{}-secure:{}", ns_id, glob_hash),
					traefik::TraefikRouter {
						entry_points: vec!["websecure".into()],
						rule: Some(router_rule),
						priority: Some(
							(BASE_ROUTER_PRIORITY + 1).saturating_add(route.priority.try_into()?),
						),
						service: service.to_owned(),
						middlewares: custom_headers_router_middlewares_cdn.clone(),
						tls: Some(traefik::TraefikTls::build_cloudflare()),
					},
				);
				config.http.routers.insert(
					format!("ns-custom-headers:{}-secure-html:{}", ns_id, glob_hash),
					traefik::TraefikRouter {
						entry_points: vec!["websecure".into()],
						rule: Some(router_rule_html),
						priority: Some(
							(HTML_ROUTER_PRIORITY + 1).saturating_add(route.priority.try_into()?),
						),
						service: service.to_owned(),
						middlewares: custom_headers_router_middlewares_html.clone(),
						tls: Some(traefik::TraefikTls::build_cloudflare()),
					},
				);
			}
			Err(err) => tracing::error!(?ns_id, ?err, "could not parse glob"),
		}
	} else {
		tracing::warn!(?route, "glob not found");
	}

	Ok(())
}
