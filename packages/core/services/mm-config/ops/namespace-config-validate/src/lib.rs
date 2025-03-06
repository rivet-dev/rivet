use proto::{backend::pkg::*, common};
use rivet_operation::prelude::*;

#[operation(name = "mm-config-namespace-config-validate")]
async fn handle(
	ctx: OperationContext<mm_config::namespace_config_validate::Request>,
) -> GlobalResult<mm_config::namespace_config_validate::Response> {
	let mut errors = Vec::new();

	if ctx.lobby_count_max < 1 {
		errors.push(util::err_path!["lobby-count", "too-low"]);
	} else if ctx.lobby_count_max >= 32_768 {
		errors.push(util::err_path!["lobby-count", "too-high"]);
	}

	// TODO: Remove when namespace inputs for the below are implemented
	if ctx.max_players_per_client < 1 {
		errors.push(util::err_path!["max-players", "too-low"]);
	}

	if ctx.max_players_per_client < 1 {
		errors.push(util::err_path!["max-players", "client", "too-low"]);
	}
	if ctx.max_players_per_client_vpn < 1 {
		errors.push(util::err_path!["max-players", "vpn", "too-low"]);
	}
	if ctx.max_players_per_client_proxy < 1 {
		errors.push(util::err_path!["max-players", "proxy", "too-low"]);
	}
	if ctx.max_players_per_client_tor < 1 {
		errors.push(util::err_path!["max-players", "tor", "too-low"]);
	}
	if ctx.max_players_per_client_hosting < 1 {
		errors.push(util::err_path!["max-players", "hosting", "too-low"]);
	}

	Ok(mm_config::namespace_config_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| common::ValidationError { path })
			.collect::<Vec<_>>(),
	})
}
