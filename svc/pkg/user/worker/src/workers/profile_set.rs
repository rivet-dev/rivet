use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-profile-set")]
async fn worker(ctx: OperationContext<user::msg::profile_set::Message>) -> Result<(), GlobalError> {
	let body = ctx.body();
	let user::msg::profile_set::Message {
		user_id,
		display_name,
		account_number,
		bio,
	} = body;
	let user_id = internal_unwrap!(user_id);

	let mut query_components = Vec::new();

	// Check if each component exists
	if display_name.is_some() {
		query_components.push(format!("display_name = ${}", query_components.len() + 2));
	}
	if account_number.is_some() {
		query_components.push(format!("account_number = ${}", query_components.len() + 2));
	}
	if bio.is_some() {
		query_components.push(format!("bio = ${}", query_components.len() + 2));
	}

	internal_assert!(!query_components.is_empty());

	// Validate profile
	let validation_res = op!([ctx] user_profile_validate {
		user_id: body.user_id,
		display_name: display_name.clone(),
		account_number: *account_number,
		bio: bio.clone()
	})
	.await?;
	if !validation_res.errors.is_empty() {
		tracing::warn!(errors = ?validation_res.errors, "validation errors");

		let readable_errors = validation_res
			.errors
			.iter()
			.map(|err| err.path.join("."))
			.collect::<Vec<_>>()
			.join(", ");
		panic_with!(VALIDATION_ERROR, error = readable_errors);
	}

	// Build query
	let built_query = query_components.join(",");
	let query_string = format!("UPDATE users SET {} WHERE user_id = $1", built_query);

	let query = sqlx::query(&query_string).bind(**user_id);

	// Bind display name
	let query = if let Some(display_name) = display_name {
		query.bind(display_name)
	} else {
		query
	};

	// Bind account number
	let query = if let Some(account_number) = account_number {
		query.bind(*account_number as i64)
	} else {
		query
	};

	// Bind bio
	let query = if let Some(bio) = bio {
		query.bind(util::format::biography(bio))
	} else {
		query
	};

	query.execute(&ctx.crdb("db-user").await?).await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: Some(*user_id),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "user.profile_set".into(),
				user_id: Some(*user_id),
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}
