use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-validate")]
async fn handle(
	ctx: OperationContext<game::validate::Request>,
) -> GlobalResult<game::validate::Response> {
	let mut errors = Vec::new();

	if ctx.display_name.is_empty() {
		errors.push(util::err_path!["display-name", "too-short"]);
	} else if ctx.display_name.len() > util::check::MAX_DISPLAY_NAME_LEN {
		errors.push(util::err_path!["display-name", "too-long"]);
	}

	if !util::check::display_name(&ctx.display_name) {
		errors.push(util::err_path!["display-name", "invalid"]);
	}

	if ctx.name_id.is_empty() {
		errors.push(util::err_path!["name-id", "too-short"]);
	} else if ctx.name_id.len() > util::check::MAX_IDENT_LEN {
		errors.push(util::err_path!["name-id", "too-long"]);
	}

	if util::check::ident(&ctx.name_id) {
		let games_res = op!([ctx] game_resolve_name_id {
			name_ids: vec![ctx.name_id.clone()],
		})
		.await?;

		// Validate name id uniqueness
		if !games_res.games.is_empty() {
			errors.push(util::err_path!["name-id", "not-unique"]);
		}
	} else {
		errors.push(util::err_path!["name-id", "invalid"]);
	}

	Ok(game::validate::Response {
		errors: errors
			.into_iter()
			.map(|path| game::validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}
