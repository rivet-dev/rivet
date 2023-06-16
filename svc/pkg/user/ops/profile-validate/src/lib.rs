use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-profile-validate")]
async fn handle(
	ctx: OperationContext<user::profile_validate::Request>,
) -> GlobalResult<user::profile_validate::Response> {
	let mut errors = Vec::new();

	// Validate display name
	if let Some(display_name) = &ctx.display_name {
		if display_name.is_empty() {
			errors.push(util::err_path!["display-name", "too-short"]);
		} else if display_name.len() > util::check::MAX_DISPLAY_NAME_LEN {
			errors.push(util::err_path!["display-name", "too-long"]);
		}

		if util::check::display_name(display_name) {
			let profanity_res = op!([ctx] profanity_check {
				strings: vec![display_name.clone()],
				censor: false,
			})
			.await?;

			if *internal_unwrap_owned!(profanity_res.results.first()) {
				errors.push(util::err_path!["display-name", "invalid"]);
			}
		} else {
			errors.push(util::err_path!["display-name", "invalid"]);
		}
	}

	// Validate account number
	if let Some(account_number) = &ctx.account_number {
		if *account_number < 1 || *account_number > 9999 {
			errors.push(util::err_path!["account-number-invalid"]);
		}
	}

	// Validate biography
	if let Some(bio) = &ctx.bio {
		if bio.len() > util::check::MAX_BIOGRAPHY_LEN {
			errors.push(util::err_path!["bio", "too-long"]);
		}

		if !util::check::biography(bio) {
			errors.push(util::err_path!["bio", "invalid"]);
		}
	}

	// Only validate handle uniqueness if at least one of the two handle components is given
	if ctx.display_name.is_some() || ctx.account_number.is_some() {
		// If either the display name or account number are missing, fetch them from the given user
		let (display_name, account_number) =
			if ctx.display_name.is_none() || ctx.account_number.is_none() {
				let user_id = internal_unwrap!(ctx.user_id);

				let users_res = op!([ctx] user_get {
					user_ids: vec![*user_id],
				})
				.await?;

				let user = users_res.users.first();
				let user = internal_unwrap!(user, "user not found");

				(
					ctx.display_name
						.clone()
						.unwrap_or(user.display_name.clone()),
					ctx.account_number.unwrap_or(user.account_number),
				)
			} else {
				(
					internal_unwrap!(ctx.display_name).clone(),
					*internal_unwrap!(ctx.account_number),
				)
			};

		// Find user by handle
		let (user_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
			"
			SELECT EXISTS (
				SELECT 1
				FROM users
				WHERE display_name = $1 and account_number = $2
			)
			"
		))
		.bind(display_name)
		.bind(account_number as i64)
		.fetch_one(&ctx.crdb("db-user").await?)
		.await?;

		// Validate handle uniqueness
		if user_exists {
			errors.push(util::err_path!["handle-not-unique"]);
		}
	}

	Ok(user::profile_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| user::profile_validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}
