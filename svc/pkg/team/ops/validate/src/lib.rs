use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-validate")]
async fn handle(
	ctx: OperationContext<team::validate::Request>,
) -> GlobalResult<team::validate::Response> {
	let mut errors = Vec::new();

	if ctx.display_name.is_empty() {
		errors.push(util::err_path!["too-short"]);
	} else if ctx.display_name.len() > util::check::MAX_DISPLAY_NAME_LEN {
		errors.push(util::err_path!["too-long"]);
	}

	if util::check::display_name(&ctx.display_name) {
		let teams_res = op!([ctx] team_resolve_display_name {
			display_names: vec![ctx.display_name.clone()],
		})
		.await?;

		// Validate name id uniqueness
		if !teams_res.teams.is_empty() {
			errors.push(util::err_path!["not-unique"]);
		}

		let profanity_res = op!([ctx] profanity_check {
			strings: vec![ctx.display_name.clone()],
			censor: false,
		})
		.await?;

		if *unwrap!(profanity_res.results.first()) {
			errors.push(util::err_path!["display-name-invalid"]);
		}
	} else {
		errors.push(util::err_path!["display-name-invalid"]);
	}

	Ok(team::validate::Response {
		errors: errors
			.into_iter()
			.map(|path| team::validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}
