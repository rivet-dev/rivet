use chirp_worker::prelude::*;
use proto::backend::{self, cdn::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let site_res = op!([ctx] faker_cdn_site {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
	let site_id = site_res.site_id.unwrap();

	let version_id = Into::<common::Uuid>::into(Uuid::new_v4());

	op!([ctx] cdn_version_publish {
		version_id: Some(version_id),
		config: Some(backend::cdn::VersionConfig {
			site_id: Some(site_id),
			routes: vec![
				Route {
					glob: Some(util::glob::Glob::parse("test-glob").unwrap().into()),
					priority: 0,
					middlewares: vec![
						Middleware {
							kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
								headers: vec![custom_headers_middleware::Header {
									name: "header-name".to_string(),
									value: "header-value".to_string(),
								}],
							})),
						},
					],
				},
				Route {
					glob: Some(util::glob::Glob::parse("test-glob2").unwrap().into()),
					priority: 1,
					middlewares: vec![
						Middleware {
							kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
								headers: vec![custom_headers_middleware::Header {
									name: "header-name2".to_string(),
									value: "header-value2".to_string(),
								}],
							})),
						},
					],
				},
			],
		}),
		config_ctx: Some(backend::cdn::VersionConfigCtx {}),
	})
	.await
	.unwrap();

	let res = op!([ctx] cdn_version_get {
		version_ids: vec![version_id]
	})
	.await
	.unwrap();

	let version = res.versions.first().expect("version not found");

	version
		.config
		.as_ref()
		.unwrap()
		.routes
		.first()
		.expect("no custom headers added");
}
