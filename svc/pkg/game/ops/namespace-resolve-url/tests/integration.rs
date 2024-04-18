use chirp_worker::prelude::*;

#[worker_test]
async fn test_no_ns(ctx: TestCtx) {
	if !util::feature::dns() {
		return;
	}

	let game_data = prepare_game(&ctx).await;

	let domain_main_res = test_url(
		&ctx,
		format!(
			"http://{}.{}/hello-world",
			game_data.name_id,
			util::env::domain_cdn().expect("no domain cdn")
		),
	)
	.await
	.expect("base domain did not resolve");
	assert_eq!(game_data.game_id, domain_main_res.game_id);
	assert_eq!(
		game_data.prod_ns().namespace_id,
		domain_main_res.namespace_id
	);
}

#[worker_test]
async fn test_with_ns(ctx: TestCtx) {
	if !util::feature::dns() {
		return;
	}

	let game_data = prepare_game(&ctx).await;
	for ns in &game_data.namespaces {
		let domain_main_res = test_url(
			&ctx,
			format!(
				"http://{}--{}.{}/hello-world",
				game_data.name_id,
				ns.name_id,
				util::env::domain_cdn().unwrap()
			),
		)
		.await
		.expect("base domain did not resolve");
		assert_eq!(game_data.game_id, domain_main_res.game_id);
		assert_eq!(ns.namespace_id, domain_main_res.namespace_id);
	}
}

#[worker_test]
async fn test_with_custom_domain(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let game_data = prepare_game(&ctx).await;
	let domain = format!("{}.com", util::faker::ident());

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(game_data.prod_ns().namespace_id.into()),
		domain: domain.clone(),
	})
	.await
	.unwrap();

	let domain_main_res = test_url(&ctx, format!("http://{}/hello-world", domain))
		.await
		.expect("custom domain did not resolve");
	assert_eq!(game_data.game_id, domain_main_res.game_id);
	assert_eq!(
		game_data.prod_ns().namespace_id,
		domain_main_res.namespace_id
	);
}

struct GameData {
	game_id: Uuid,
	name_id: String,
	namespaces: Vec<NsData>,
}

impl GameData {
	fn prod_ns(&self) -> &NsData {
		self.namespaces
			.iter()
			.find(|ns| ns.name_id == "prod")
			.expect("missing prod ns")
	}
}

struct NsData {
	namespace_id: Uuid,
	name_id: String,
}

async fn prepare_game(ctx: &TestCtx) -> GameData {
	let create_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let game_res = op!([ctx] game_get {
		game_ids: vec![create_res.game_id.unwrap()],
	})
	.await
	.unwrap();
	let game_data = game_res.games.first().unwrap();

	let ns_list_res = op!([ctx] game_namespace_list {
		game_ids: vec![game_data.game_id.unwrap()],
	})
	.await
	.unwrap();
	let ns_list = &ns_list_res.games.first().unwrap().namespace_ids;

	let ns_get_res = op!([ctx] game_namespace_get {
		namespace_ids: ns_list.clone(),
	})
	.await
	.unwrap();

	GameData {
		game_id: game_data.game_id.as_ref().unwrap().as_uuid(),
		name_id: game_data.name_id.clone(),
		namespaces: ns_get_res
			.namespaces
			.iter()
			.map(|ns| NsData {
				namespace_id: ns.namespace_id.as_ref().unwrap().as_uuid(),
				name_id: ns.name_id.clone(),
			})
			.collect::<Vec<_>>(),
	}
}

struct Resolution {
	game_id: Uuid,
	namespace_id: Uuid,
}

async fn test_url(ctx: &TestCtx, url: impl ToString) -> Option<Resolution> {
	let res = op!([ctx] game_namespace_resolve_url {
		url: url.to_string(),
	})
	.await
	.unwrap();

	res.resolution.as_ref().map(|r| Resolution {
		game_id: r.game_id.as_ref().unwrap().as_uuid(),
		namespace_id: r.namespace_id.as_ref().unwrap().as_uuid(),
	})
}
