use chirp_worker::prelude::*;
use lazy_static::lazy_static;
use proto::backend::pkg::*;
use rand::{seq::IteratorRandom, Rng};
use serde_json::json;

lazy_static! {
	// Load adjectives from file
	static ref ADJECTIVES: Vec<&'static str> = include_str!("../../adjectives.txt")
		.split('\n')
		.filter(|l| !l.is_empty())
		.map(|l| l.trim())
		.collect();
}

#[worker(name = "user-create")]
async fn worker(ctx: &OperationContext<user::msg::create::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	let join_ts = ctx.ts();

	// Get customizations from game version
	let (display_name, avatar_upload_id) = if let Some(namespace_id) = ctx.namespace_id {
		let namespace_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![namespace_id],
		})
		.await?;
		let version_id = internal_unwrap_owned!(
			internal_unwrap_owned!(namespace_res.namespaces.first()).version_id
		);

		let identity_config_res = op!([ctx] identity_config_version_get {
			version_ids: vec![version_id],
		})
		.await?;
		let identity_config =
			internal_unwrap!(internal_unwrap_owned!(identity_config_res.versions.first()).config);

		let mut rng = rand::thread_rng();
		let display_name = identity_config
			.custom_display_names
			.iter()
			.choose(&mut rng)
			.map(|c| c.display_name.clone());
		let avatar_upload_id = identity_config
			.custom_avatars
			.iter()
			.choose(&mut rng)
			.and_then(|avatar| avatar.upload_id)
			.as_ref()
			.map(common::Uuid::as_uuid);

		(display_name, avatar_upload_id)
	} else {
		(None, None)
	};

	// Attempt to create a unique handle 3 times
	let crdb = ctx.crdb("db-user").await?;
	let mut attempts = 3u32;
	let (display_name, account_number) = loop {
		if attempts == 0 {
			internal_panic!("failed all attempts to create unique user handle");
		}
		attempts -= 1;

		let display_name = gen_display_name(display_name.as_deref().unwrap_or("Guest"));

		if let Some(x) = insert_user(
			&crdb,
			user_id,
			display_name.clone(),
			avatar_upload_id,
			join_ts,
		)
		.await?
		{
			break x;
		}
	};

	msg!([ctx] user::msg::create_complete(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "user.create".into(),
				user_id: Some(user_id.into()),
				namespace_id: ctx.namespace_id,
				properties_json: Some(serde_json::to_string(&json!({
				}))?),
				..Default::default()
			},
			analytics::msg::event_create::Event {
				name: "user.profile_set".into(),
				user_id: Some(user_id.into()),
				namespace_id: ctx.namespace_id,
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}

// Handles unique constraint violations
async fn insert_user(
	crdb: &CrdbPool,
	user_id: Uuid,
	display_name: String,
	avatar_upload_id: Option<Uuid>,
	join_ts: i64,
) -> GlobalResult<Option<(String, i64)>> {
	let account_number = gen_account_number();
	tracing::info!(%display_name, %account_number, "attempt");

	let res = if let Some(avatar_upload_id) = avatar_upload_id {
		sqlx::query(indoc!(
			"
			INSERT INTO users (
				user_id,
				display_name,
				account_number,
				avatar_id,
				profile_id,
				join_ts
			)
			VALUES ($1, $2, $3, $4, $5, $6)
			"
		))
		.bind(user_id)
		.bind(&display_name)
		.bind(account_number)
		.bind(gen_avatar_id())
		.bind(avatar_upload_id)
		.bind(join_ts)
		.execute(crdb)
		.await
	} else {
		sqlx::query(indoc!(
			"
			INSERT INTO users (
				user_id,
				display_name,
				account_number,
				avatar_id,
				join_ts
			)
			VALUES ($1, $2, $3, $4, $5)
			"
		))
		.bind(user_id)
		.bind(&display_name)
		.bind(gen_account_number())
		.bind(gen_avatar_id())
		.bind(join_ts)
		.execute(crdb)
		.await
	};

	match res {
		Ok(_) => Ok(Some((display_name, account_number))),
		// https://www.postgresql.org/docs/current/errcodes-appendix.html
		Err(sqlx::Error::Database(err)) => {
			let pg_err =
				internal_unwrap_owned!(err.try_downcast_ref::<sqlx::postgres::PgDatabaseError>());

			if pg_err.code() == "23505"
				&& !pg_err
					.detail()
					.map_or(false, |d| d.contains("Key (user_id)"))
			{
				Ok(None)
			} else {
				Err(err.into())
			}
		}
		Err(err) => Err(err.into()),
	}
}

// Generates a display name with the format `{adjective:7}{space:1}{base:11}{space:1}{number:4}`
fn gen_display_name(base: impl std::fmt::Display) -> String {
	let base_str = format!("{}", base);

	let mut rand = rand::thread_rng();
	let adj = ADJECTIVES.iter().choose(&mut rand).unwrap_or(&"Unknown");

	format!(
		"{} {} {}",
		adj,
		base_str,
		std::iter::repeat_with(|| rand.gen_range(0..10))
			.map(|d| d.to_string())
			.take(4)
			.collect::<String>()
	)
}

fn gen_account_number() -> i64 {
	rand::thread_rng().gen_range(1..10000)
}

fn gen_avatar_id() -> String {
	format!("avatar-{}", rand::thread_rng().gen_range(0..7))
}
