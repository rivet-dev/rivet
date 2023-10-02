use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-profile-validate")]
async fn handle(
	ctx: OperationContext<team::profile_validate::Request>,
) -> GlobalResult<team::profile_validate::Response> {
	let mut errors = Vec::new();

	// Validate display name
	if let Some(display_name) = &ctx.display_name {
		if display_name.is_empty() {
			errors.push(util::err_path!["display-name", "too-short"]);
		} else if display_name.len() > util::check::MAX_DISPLAY_NAME_LEN {
			errors.push(util::err_path!["display-name", "too-long"]);
		}

		if !util::check::display_name(display_name) {
			errors.push(util::err_path!["display-name", "invalid"]);
		}

		// Validate display name uniqueness
		let (team_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
			"
			SELECT EXISTS (
				SELECT 1
				FROM db_team.teams
				WHERE display_name = $1
			)
		"
		))
		.bind(display_name)
		.fetch_one(&ctx.crdb().await?)
		.await?;

		if team_exists {
			errors.push(util::err_path!["display-name", "not-unique"]);
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

	Ok(team::profile_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| team::profile_validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}
