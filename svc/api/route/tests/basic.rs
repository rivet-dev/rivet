use std::sync::Once;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use std::time::Duration;

/// How long to wait for a CDN configuration to apply.
const CDN_SLEEP_DURATION: Duration = Duration::from_secs(5);

/// How frequently to poll the CDN for the new configuration to be applied.
const CDN_POLL_INTERVAL: Duration = Duration::from_millis(500);

static GLOBAL_INIT: Once = Once::new();

const API_ROUTE_URL: &'static str = "http://rivet-api-route.rivet-service.svc.cluster.local";

struct Ctx {
	op_ctx: OperationContext<()>,
}

impl Ctx {
	async fn init() -> Ctx {
		GLOBAL_INIT.call_once(|| {
			tracing_subscriber::fmt()
				.pretty()
				.with_max_level(tracing::Level::INFO)
				.with_target(false)
				.without_time()
				.init();
		});

		let pools = rivet_pools::from_env("api-auth-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-auth-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-auth-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-auth-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		Ctx { op_ctx }
	}

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn cdn() {
	struct Namespace {
		namespace_id: Uuid,
		domain: String,
	}

	let ctx = Ctx::init().await;

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	// let game_id = game_res.game_id.as_ref().unwrap().as_uuid();
	let namespaces = game_res
		.namespace_ids
		.clone()
		.into_iter()
		.map(|namespace_id| Namespace {
			namespace_id: *namespace_id,
			domain: format!("{}.com", util::faker::ident()),
		})
		.collect::<Vec<_>>();

	for ns in &namespaces {
		op!([ctx] cdn_namespace_domain_create {
			namespace_id: Some(ns.namespace_id.into()),
			domain: ns.domain.clone(),
		})
		.await
		.unwrap();
	}

	tokio::time::sleep(CDN_SLEEP_DURATION).await;

	// MARK: GET /traefik/config/core
	{
		tracing::info!("fetching traefik config");

		// TODO: Arguments
		let token = util::env::read_secret(&["rivet", "api_route", "token"])
			.await
			.unwrap();
		let res = reqwest::Client::new()
			.get(&format!(
				"{API_ROUTE_URL}/traefik/config/core?token={token}"
			))
			.send()
			.await
			.unwrap()
			.error_for_status()
			.unwrap()
			.json::<api_route::route::traefik::TraefikHttpNullified>()
			.await
			.unwrap();
		let routers = res.routers.as_ref().expect("no routers");
		let middlewares = res
			.http()
			.unwrap()
			.middlewares
			.as_ref()
			.expect("no middlewares");

		for ns in &namespaces {
			let rewrite_middleware_key = format!("ns-rewrite:{}", ns.namespace_id);
			let ns_router_insecure = routers
				.get(&format!("ns:{}-insecure", ns.namespace_id))
				.expect("missing insecure router");
			let ns_router_secure = routers
				.get(&format!("ns:{}-secure", ns.namespace_id))
				.expect("missing secure router");
			let rewrite_middleware = middlewares
				.get(&rewrite_middleware_key)
				.expect("missing rewrite middleware");

			{
				let custom_headers_key = format!("ns-custom-headers:{}-secure:", ns.namespace_id);

				routers
					.iter()
					.find(|(key, _)| key.starts_with(&custom_headers_key))
					.expect("missing custom headers routers");
				routers
					.iter()
					.find(|(key, _)| key.starts_with(&custom_headers_key))
					.expect("missing custom headers middleware");
			}

			tracing::info!(
				?ns_router_insecure,
				?ns_router_secure,
				?rewrite_middleware,
				"found namespace"
			);

			let domain_part = format!("Host(`{}`)", ns.domain);
			assert!(ns_router_insecure.rule().unwrap().contains(&domain_part));
			assert!(ns_router_secure.rule().unwrap().contains(&domain_part));

			assert_eq!(
				"traffic-server-traffic-server@kubernetescrd",
				ns_router_insecure.service().unwrap()
			);
			assert_eq!(
				"traffic-server-traffic-server@kubernetescrd",
				ns_router_secure.service().unwrap()
			);

			ns_router_insecure
				.middlewares()
				.unwrap()
				.contains(&rewrite_middleware_key);
			ns_router_secure
				.middlewares()
				.unwrap()
				.contains(&rewrite_middleware_key);
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn job_run() {
	struct Namespace {
		namespace_id: Uuid,
		domain: String,
	}

	let ctx = Ctx::init().await;

	// Run a job and wait for the ports to register
	let run_id = Uuid::new_v4();
	let mut resolved_sub = subscribe!([ctx] job_run::msg::ports_resolved(run_id))
		.await
		.unwrap();
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region = region_res.region.as_ref().unwrap();
	op!([ctx] faker_job_run {
		run_id: Some(run_id.into()),
		region_id: region_res.region_id,
		proxied_ports: vec![
			job_run::msg::create::ProxiedPort {
				target_nomad_port_label: Some("http".into()),
				ingress_port: None,
				ingress_hostnames: vec!["test1.com".into(), "test2.com".into()],
				proxy_protocol: backend::job::ProxyProtocol::Http as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
			job_run::msg::create::ProxiedPort {
				target_nomad_port_label: Some("http".into()),
				ingress_port: None,
				ingress_hostnames: vec!["test1.com".into(), "test2.com".into()],
				proxy_protocol: backend::job::ProxyProtocol::Https as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
		],
		..Default::default()
	})
	.await
	.unwrap();
	resolved_sub.next().await.unwrap();

	// MARK: GET /traefik/config
	{
		tracing::info!("fetching traefik config");

		let res = ctx
			.http_client
			.traefik_config()
			.token(
				util::env::read_secret(&["rivet", "api_route", "token"])
					.await
					.unwrap(),
			)
			.region(&region.name_id)
			.pool("ing-job")
			.send()
			.await
			.unwrap();
		let services = res.http().unwrap().services.as_ref().expect("no services");
		let routers = res.http().unwrap().routers.as_ref().expect("no routers");

		tracing::info!(
			keys = ?services.keys().collect::<Vec<_>>(),
			"all http services"
		);

		// Validate service exists. We don't know the target port, so we search
		// for a matching prefix.
		let service_id = format!("job-run:{run_id}:http");
		assert!(
			services.contains_key(&service_id),
			"missing job run service"
		);

		// Validate HTTP
		let router_http = routers
			.get(&format!("job-run:{run_id}:http:http"))
			.expect("missing http router");
		assert_eq!(vec!["lb-80"], router_http.entry_points().unwrap());
		assert_eq!(
			"Host(`test1.com`) || Host(`test2.com`)",
			router_http.rule().unwrap()
		);
		assert_eq!(service_id, router_http.service().unwrap());
		assert!(router_http.tls.is_none());

		// Validate HTTPS
		let _router_https = routers
			.get(&format!("job-run:{run_id}:http:https"))
			.expect("missing http router");
	}
}

mod cdn_suite {

	use super::Ctx;
	use proto::backend::{self, cdn::*, pkg::*};
	use rivet_operation::prelude::*;

	struct CdnVersion {
		game: backend::game::Game,
		namespace_id: Uuid,
		base: String,
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn cdn_all() {
		let ctx = Ctx::init().await;

		// Create CDN
		let version = create_cdn_version(
			&ctx,
			vec![
				Route {
					glob: Some(util::glob::Glob::parse("**/*.html").unwrap().into()),
					priority: 1,
					middlewares: vec![Middleware {
						kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
							headers: vec![custom_headers_middleware::Header {
								name: "header-name".to_string(),
								value: "header-value".to_string(),
							}],
						})),
					}],
				},
				Route {
					glob: Some(util::glob::Glob::parse("**/index.html").unwrap().into()),
					priority: 2,
					middlewares: vec![Middleware {
						kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
							headers: vec![custom_headers_middleware::Header {
								name: "header-name2".to_string(),
								value: "header-value2".to_string(),
							}],
						})),
					}],
				},
			],
		)
		.await;

		// TODO: Figure out
		// // Enable CDN auth
		// op!([ctx] cdn_ns_auth_type_set {
		// 	namespace_id: Some(version.namespace_id.into()),
		// 	auth_type: backend::cdn::namespace_config::AuthType::Basic as i32,
		// })
		// .await
		// .unwrap();

		// Add auth user
		let (password, password_hash) = util::faker::bcrypt();
		op!([ctx] cdn_namespace_auth_user_update {
			namespace_id: Some(version.namespace_id.into()),
			user: "root".to_owned(),
			password: password_hash,
		})
		.await
		.unwrap();

		// Test HTML
		{
			let res = test_cdn_path(&version, "/index.html", |req| {
				req.basic_auth("root", Some(&password))
			})
			.await;

			assert_ne!(
				http::status::StatusCode::UNAUTHORIZED,
				res.status(),
				"failed to authenticate"
			);
			assert_eq!(http::status::StatusCode::OK, res.status());

			let headers = res.headers();
			assert_eq!(
				None,
				headers.get("header-name"),
				"header should not exist based on priority",
			);
			assert_eq!(
				"header-value2",
				headers.get("header-name2").expect("header not found"),
			);
		}

		// Test TXT
		{
			let res = test_cdn_path(&version, "/hello/world.txt", |req| {
				req.basic_auth("root", Some(&password))
			})
			.await;

			assert_ne!(
				res.status(),
				http::status::StatusCode::UNAUTHORIZED,
				"failed to authenticate"
			);
			assert_eq!(res.status(), http::status::StatusCode::OK);

			let headers = res.headers();
			assert_eq!(headers.get("header-name"), None, "header should not exist");
			assert_eq!(headers.get("header-name2"), None, "header should not exist");
		}
	}

	async fn create_cdn_version(ctx: &Ctx, routes: Vec<backend::cdn::Route>) -> CdnVersion {
		let game_create_res = op!([ctx] faker_game {
			skip_namespaces_and_versions: true,
			..Default::default()
		})
		.await
		.unwrap();

		let site_res = op!([ctx] faker_cdn_site {
			game_id: game_create_res.game_id,
		})
		.await
		.unwrap();

		let version_create_res = op!([ctx] faker_game_version {
			game_id: game_create_res.game_id,
			override_cdn_config: Some(faker::game_version::request::OverrideCdnConfig {
				config: Some(backend::cdn::VersionConfig {
					site_id: site_res.site_id,
					routes,
				}),
			}),
			..Default::default()
		})
		.await
		.unwrap();

		let ns_create_res = op!([ctx] faker_game_namespace {
			game_id: game_create_res.game_id,
			version_id: version_create_res.version_id,
			override_name_id: "prod".to_owned(),
			..Default::default()
		})
		.await
		.unwrap();

		let game_res = op!([ctx] game_get {
			game_ids: vec![game_create_res.game_id.unwrap()],
		})
		.await
		.unwrap();
		let game = game_res.games.first().unwrap().clone();

		CdnVersion {
			namespace_id: *ns_create_res.namespace_id.unwrap(),
			base: format!("https://{}.{}", game.name_id, util::env::domain_cdn()),
			game,
		}
	}

	#[tracing::instrument(skip_all, fields(base = version.base, path = ?path))]
	async fn test_cdn_path<F>(
		version: &CdnVersion,
		path: impl AsRef<str> + std::fmt::Debug,
		req_cb: F,
	) -> reqwest::Response
	where
		F: Fn(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
	{
		let url = format!("{}{}", version.base, path.as_ref());
		tracing::info!(?url, "testing url");

		// Try for 10s
		let mut tries = 0;
		while tries < 20 {
			tries += 1;

			// Create a new client every request or else the request will be cached
			let client = reqwest::Client::new();
			let req = req_cb(client.get(&url));
			let res = req.try_clone().unwrap().send().await.unwrap();
			tracing::info!(status = ?res.status(), "received status");

			if res.status() != http::status::StatusCode::BAD_GATEWAY
				&& res.status() != http::status::StatusCode::NOT_FOUND
			{
				return res;
			}

			tokio::time::sleep(super::CDN_POLL_INTERVAL).await;
		}

		panic!("failed to fetch");
	}
}
