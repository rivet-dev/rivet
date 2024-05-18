use proto::{backend::pkg::*, common};
use rivet_operation::prelude::*;

#[operation(name = "game-namespace-validate")]
async fn handle(
	ctx: OperationContext<game::namespace_validate::Request>,
) -> GlobalResult<game::namespace_validate::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let mut errors = Vec::new();

	if ctx.display_name.is_empty() {
		errors.push(util::err_path!["display-name", "too-short"]);
	} else if ctx.display_name.len() > util::check::MAX_DISPLAY_NAME_LONG_LEN {
		errors.push(util::err_path!["display-name", "too-long"]);
	}

	if !util::check::display_name_long(&ctx.display_name) {
		errors.push(util::err_path!["display-name", "invalid"]);
	}

	if ctx.name_id.is_empty() {
		errors.push(util::err_path!["name-id", "too-short"]);
	} else if ctx.name_id.len() > util::check::MAX_IDENT_LEN {
		errors.push(util::err_path!["name-id", "too-long"]);
	}

	if util::check::ident(&ctx.name_id) {
		let namespaces_res = op!([ctx] game_namespace_resolve_name_id {
			game_id: Some(game_id.into()),
			name_ids: vec![ctx.name_id.clone()],
		})
		.await?;

		// Validate name id uniqueness
		if !namespaces_res.namespaces.is_empty() {
			errors.push(util::err_path!["name-id", "not-unique"]);
		}
	} else {
		errors.push(util::err_path!["name-id", "invalid"]);
	}

	Ok(game::namespace_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| common::ValidationError { path })
			.collect::<Vec<_>>(),
	})
}
