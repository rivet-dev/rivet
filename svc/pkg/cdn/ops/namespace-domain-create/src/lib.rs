use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "cdn-namespace-domain-create")]
async fn handle(
	ctx: OperationContext<cdn::namespace_domain_create::Request>,
) -> GlobalResult<cdn::namespace_domain_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	assert_with!(util::check::domain(&ctx.domain, true), CDN_INVALID_DOMAIN);

	let game_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let game = internal_unwrap_owned!(game_res.games.first());
	let game_id = internal_unwrap!(game.game_id).as_uuid();

	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = internal_unwrap_owned!(game_res.games.first());
	let developer_team_id = internal_unwrap!(game.developer_team_id).as_uuid();

	let crdb = ctx.crdb("db-cdn").await?;
	let (domain_count,) = sqlx::query_as::<_, (i64,)>(
		"SELECT COUNT(*) FROM game_namespace_domains WHERE namespace_id = $1",
	)
	.bind(namespace_id)
	.fetch_one(&crdb)
	.await?;

	assert_with!(domain_count < 10, CDN_TOO_MANY_DOMAINS);

	sqlx::query(indoc!(
		"
		INSERT INTO game_namespace_domains (namespace_id, domain, create_ts)
		VALUES ($1, $2, $3)
		"
	))
	.bind(namespace_id)
	.bind(&ctx.domain)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	// Create a cloudflare custom hostname
	{
		let custom_hostname_res = msg!([ctx] cf_custom_hostname::msg::create(namespace_id, &ctx.domain) -> Result<cf_custom_hostname::msg::create_complete, cf_custom_hostname::msg::create_fail> {
			namespace_id: ctx.namespace_id,
			hostname: ctx.domain.clone(),
			bypass_pending_cap: false,
		}).await?;

		match custom_hostname_res {
			Ok(_) => {}
			Err(msg) => {
				use cf_custom_hostname::msg::create_fail::ErrorCode::*;

				let code =
					cf_custom_hostname::msg::create_fail::ErrorCode::from_i32(msg.error_code);
				match internal_unwrap_owned!(code) {
					Unknown => internal_panic!("unknown custom hostname create error code"),
					AlreadyExists => {
						rollback(&crdb, namespace_id, &ctx.domain).await?;
						panic_with!(CLOUD_HOSTNAME_TAKEN)
					}
					TooManyPendingHostnames => {
						rollback(&crdb, namespace_id, &ctx.domain).await?;
						panic_with!(CLOUD_TOO_MANY_PENDING_HOSTNAMES_FOR_GROUP)
					}
				}
			}
		};
	}

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "cdn.domain.update".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"namespace_id": namespace_id,
					"domain": ctx.domain,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(cdn::namespace_domain_create::Response {})
}

async fn rollback(crdb: &CrdbPool, namespace_id: Uuid, domain: &str) -> GlobalResult<()> {
	// Rollback
	sqlx::query("DELETE FROM game_namespace_domains WHERE namespace_id = $1 AND domain = $2")
		.bind(namespace_id)
		.bind(domain)
		.execute(crdb)
		.await?;

	Ok(())
}
