use proto::{backend::pkg::*, common};
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
	} else {
		errors.push(util::err_path!["display-name-invalid"]);
	}

	Ok(team::validate::Response {
		errors: errors
			.into_iter()
			.map(|path| common::ValidationError { path })
			.collect::<Vec<_>>(),
	})
}
