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
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	let join_ts = ctx.ts();

	// Get customizations from game version
	let (display_name, avatar_upload_id) = if let Some(namespace_id) = ctx.namespace_id {
		let namespace_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![namespace_id],
		})
		.await?;

		let version_id = unwrap!(unwrap!(namespace_res.namespaces.first()).version_id);

		let identity_config_res = op!([ctx] identity_config_version_get {
			version_ids: vec![version_id],
		})
		.await?;
		let identity_config = unwrap_ref!(unwrap!(identity_config_res.versions.first()).config);

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
	let mut attempts = 3u32;
	let (_display_name, _account_number) = loop {
		if attempts == 0 {
			bail!("failed all attempts to create unique user handle");
		}
		attempts -= 1;

		let display_name = gen_display_name(display_name.as_deref().unwrap_or("Guest"));

		if let Some(x) = insert_user(
			ctx,
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
				event_id: Some(Uuid::new_v4().into()),
				name: "user.create".into(),
				user_id: Some(user_id.into()),
				namespace_id: ctx.namespace_id,
				properties_json: Some(serde_json::to_string(&json!({
				}))?),
				..Default::default()
			},
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
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
	ctx: &OperationContext<user::msg::create::Message>,
	user_id: Uuid,
	display_name: String,
	avatar_upload_id: Option<Uuid>,
	join_ts: i64,
) -> GlobalResult<Option<(String, i64)>> {
	let account_number = gen_account_number();
	tracing::info!(%display_name, %account_number, "attempt");

	let res = if let Some(avatar_upload_id) = avatar_upload_id {
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_user.users (
				user_id,
				display_name,
				account_number,
				avatar_id,
				profile_id,
				join_ts
			)
			VALUES ($1, $2, $3, $4, $5, $6)
			ON CONFLICT (display_name, account_number) DO NOTHING
			",
			user_id,
			&display_name,
			account_number,
			gen_avatar_id(),
			avatar_upload_id,
			join_ts,
		)
		.await?
	} else {
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_user.users (
				user_id,
				display_name,
				account_number,
				avatar_id,
				join_ts
			)
			VALUES ($1, $2, $3, $4, $5)
			ON CONFLICT (display_name, account_number) DO NOTHING
			",
			user_id,
			&display_name,
			gen_account_number(),
			gen_avatar_id(),
			join_ts,
		)
		.await?
	};

	if res.rows_affected() == 1 {
		Ok(Some((display_name, account_number)))
	} else {
		Ok(None)
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
