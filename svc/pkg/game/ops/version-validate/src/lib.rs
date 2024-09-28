use std::{collections::HashSet, convert::TryInto};

use proto::{
	backend::{
		self,
		matchmaker::{
			self,
			lobby_runtime::{
				NetworkMode as LobbyRuntimeNetworkMode, ProxyKind as LobbyRuntimeProxyKind,
				ProxyProtocol as LobbyRuntimeProxyProtocol,
			},
		},
		pkg::*,
	},
	common,
};
use rivet_operation::prelude::*;

// See version-info-game-mode.ts
const MAX_LOBBY_SIZE: u32 = 256;
const MAX_MAX_PLAYERS: u32 = 256;
const MAX_MIN_IDLE_LOBBY_COUNT: u32 = 16;
const MAX_MAX_IDLE_LOBBY_COUNT: u32 = 32;
const MAX_CUSTOM_DISPLAY_NAME_LEN: usize = 11;

#[derive(Default)]
struct MiddlewareCounter {
	custom_headers: usize,
}

#[derive(PartialEq, Eq, Hash)]
enum PortRangeProtocol {
	Tcp,
	Udp,
}

struct PortRange {
	proto: PortRangeProtocol,
	min: u32,
	max: u32,
}

#[operation(name = "game-version-validate")]
async fn handle(
	ctx: OperationContext<game::version_validate::Request>,
) -> GlobalResult<game::version_validate::Response> {
	let mut errors = Vec::new();

	let game_id = unwrap_ref!(ctx.game_id);
	let proto_config = unwrap_ref!(ctx.config);

	// Display name validation
	if ctx.display_name.is_empty() {
		errors.push(util::err_path!["display-name", "too-short"]);
	} else if ctx.display_name.len() > util::check::MAX_DISPLAY_NAME_LONG_LEN {
		errors.push(util::err_path!["display-name", "too-long"]);
	}

	if !util::check::display_name_long(&ctx.display_name) {
		errors.push(util::err_path!["display-name", "invalid"]);
	}

	// Get game config
	let game_res = op!([ctx] mm_config_game_get {
		game_ids: vec![*game_id],
	})
	.await?;
	let mm_game_config = unwrap_ref!(unwrap!(game_res.games.first()).config);

	// Validate display name uniqueness
	{
		let version_list_res = op!([ctx] game_version_list {
			game_ids: vec![*game_id],
		})
		.await?;

		let version_list = unwrap!(version_list_res.games.first(), "version list not found");

		let versions_res = op!([ctx] game_version_get {
			version_ids: version_list.version_ids.clone(),
		})
		.await?;

		// Check for version name uniqueness
		if versions_res
			.versions
			.iter()
			.any(|ver| ver.display_name == ctx.display_name)
		{
			errors.push(util::err_path!["display-name", "not-unique"]);
		}
	}

	// CDN config validation
	if let Some(cdn) = &proto_config.cdn {
		// TODO: Validate site id belongs to the given game
		if cdn.site_id.is_none() {
			errors.push(util::err_path!["config", "cdn", "no-site-id"]);
		}

		if cdn.routes.len() > 32 {
			errors.push(util::err_path!["config", "cdn", "routes-meta", "too-many"]);
		}

		let mut unique_globs = HashSet::<util::glob::Glob>::new();
		for (route_index, route) in cdn.routes.iter().take(32).enumerate() {
			let glob = TryInto::<util::glob::Glob>::try_into(unwrap!(route.glob.clone()))?;

			if glob.tokens.is_empty() {
				errors.push(util::err_path![
					"config",
					"cdn",
					"routes",
					route_index,
					"glob",
					"invalid",
				]);
			} else if unique_globs.contains(&glob) {
				errors.push(util::err_path![
					"config",
					"cdn",
					"routes",
					route_index,
					"glob",
					"not-unique",
				]);
			} else {
				unique_globs.insert(glob);
			}

			if route.priority > 100 {
				errors.push(util::err_path![
					"config",
					"cdn",
					"routes",
					route_index,
					"priority",
					"too-high",
				]);
			} else if route.priority < 0 {
				errors.push(util::err_path![
					"config",
					"cdn",
					"routes",
					route_index,
					"priority",
					"too-low",
				]);
			}

			if route.middlewares.len() > 32 {
				errors.push(util::err_path![
					"config",
					"cdn",
					"routes",
					route_index,
					"middlewares-meta",
					"too-many",
				]);
			}

			let mut middleware_counter = MiddlewareCounter::default();
			for (middleware_index, middleware) in route.middlewares.iter().take(32).enumerate() {
				match unwrap_ref!(middleware.kind) {
					backend::cdn::middleware::Kind::CustomHeaders(custom_headers) => {
						if middleware_counter.custom_headers > 1 {
							errors.push(util::err_path![
								"config",
								"cdn",
								"routes",
								route_index,
								"middlewares",
								middleware_index,
								"not-unique",
							]);
						} else {
							middleware_counter.custom_headers += 1;
						}

						// TODO: check for 0 headers?
						if custom_headers.headers.len() > 32 {
							errors.push(util::err_path![
								"config",
								"cdn",
								"routes",
								route_index,
								"middlewares",
								middleware_index,
								"custom-headers",
								"headers-meta",
								"too-many",
							]);
						}

						for (header_index, header) in
							custom_headers.headers.iter().take(32).enumerate()
						{
							if header.name.is_empty() {
								errors.push(util::err_path![
									"config",
									"cdn",
									"routes",
									route_index,
									"middlewares",
									middleware_index,
									"custom-headers",
									"headers",
									header_index,
									"name",
									"invalid",
								]);
							} else if header.name.len() > 512 {
								errors.push(util::err_path![
									"config",
									"cdn",
									"routes",
									route_index,
									"middlewares",
									middleware_index,
									"custom-headers",
									"headers",
									header_index,
									"name",
									"too-long",
								]);
							}

							if header.value.len() > 1024 {
								errors.push(util::err_path![
									"config",
									"cdn",
									"routes",
									route_index,
									"middlewares",
									middleware_index,
									"custom-headers",
									"headers",
									header_index,
									"value",
									"too-long",
								]);
							}
						}
					}
				}
			}
		}
	}

	// Matchmaker config validation
	if let Some(matchmaker) = &proto_config.matchmaker {
		// Validate captcha config
		if let Some(captcha_config) = &matchmaker.captcha {
			if captcha_config.requests_before_reverify > 600 {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"captcha",
					"requests-before-reverify",
					"too-high",
				]);
			}

			if captcha_config.verification_ttl > util::duration::hours(12) {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"captcha",
					"verification-ttl",
					"too-high",
				]);
			}

			if captcha_config.turnstile.is_none() && captcha_config.hcaptcha.is_none() {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"captcha-meta",
					"must-have-one",
				]);
			}

			if let Some(hcaptcha) = &captcha_config.hcaptcha {
				if let Some(site_key) = &hcaptcha.site_key {
					if site_key.is_empty() {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"captcha",
							"hcaptcha",
							"site-key",
							"too-short",
						]);
					} else if site_key.len() > 36 {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"captcha",
							"hcaptcha",
							"site-key",
							"too-long",
						]);
					}
				} else {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"hcaptcha",
						"site-key",
						"required",
					]);
				}

				if let Some(secret_key) = &hcaptcha.secret_key {
					if secret_key.is_empty() {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"captcha",
							"hcaptcha",
							"secret-key",
							"too-short",
						]);
					} else if secret_key.len() > 42 {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"captcha",
							"hcaptcha",
							"secret-key",
							"too-long",
						]);
					}
				} else {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"hcaptcha",
						"secret-key",
						"required",
					]);
				}
			}

			if let Some(turnstile) = &captcha_config.turnstile {
				if turnstile.site_key.is_empty() {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"turnstile",
						"site-key",
						"too-short",
					]);
				} else if turnstile.site_key.len() > 30 {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"turnstile",
						"site-key",
						"too-long",
					]);
				}

				if turnstile.secret_key.is_empty() {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"turnstile",
						"secret-key",
						"too-short",
					]);
				} else if turnstile.secret_key.len() > 40 {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"captcha",
						"turnstile",
						"secret-key",
						"too-long",
					]);
				}
			}
		}

		// Fetch all regions
		let regions_res = op!([ctx] region_get {
			region_ids: matchmaker.lobby_groups
				.iter()
				.flat_map(|lobby_group| lobby_group.regions.iter())
				.filter_map(|region| region.region_id)
				.collect::<Vec<_>>()
		})
		.await?;

		// Fetch all tiers
		let tiers_res = op!([ctx] tier_list {
			region_ids: regions_res.regions
				.iter()
				.filter_map(|region| region.region_id)
				.collect::<Vec<_>>()
		})
		.await?;

		let tiers = tiers_res
			.regions
			.iter()
			.flat_map(|region| region.tiers.clone())
			.collect::<Vec<_>>();

		if matchmaker.lobby_groups.is_empty() {
			errors.push(util::err_path![
				"config",
				"matchmaker",
				"game-modes-meta",
				"too-few",
			]);
		} else if matchmaker.lobby_groups.len() > 32 {
			errors.push(util::err_path![
				"config",
				"matchmaker",
				"game-modes-meta",
				"too-many",
			]);
		}

		let mut unique_lobby_names = HashSet::new();
		for (lobby_index, lobby_group) in matchmaker.lobby_groups.iter().take(32).enumerate() {
			let lobby_group_label = format!("*{}*", lobby_group.name_id);

			if util::check::ident_lenient(&lobby_group.name_id) {
				if unique_lobby_names.contains(lobby_group.name_id.trim()) {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"game-modes",
						lobby_index,
						"name-not-unique",
					]);
				} else {
					unique_lobby_names.insert(lobby_group.name_id.trim().to_owned());
				}
			} else {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"name-id-invalid",
				]);
			}

			// Validate max players
			if lobby_group.max_players_normal == 0 {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-normal",
					"too-low",
				]);
			} else if lobby_group.max_players_normal > MAX_MAX_PLAYERS {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-normal",
					"too-high",
				]);
			}

			if lobby_group.max_players_direct == 0 {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-direct",
					"too-low",
				]);
			} else if lobby_group.max_players_direct > MAX_MAX_PLAYERS {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-direct",
					"too-high",
				]);
			}

			if lobby_group.max_players_party == 0 {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-party",
					"too-low",
				]);
			} else if lobby_group.max_players_party > MAX_MAX_PLAYERS {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"player-counts",
					"max-players-party",
					"too-high",
				]);
			}

			// Validate region ids
			for (region_index, region) in lobby_group.regions.iter().take(64).enumerate() {
				let region_config = regions_res
					.regions
					.iter()
					.find(|r| r.region_id == region.region_id);

				if let Some(region_config) = region_config {
					let region_label = format!("*{}*", region_config.name_id);

					// Validate tier name id
					if !util::check::ident(&region.tier_name_id)
						|| !tiers
							.iter()
							.any(|tier| tier.tier_name_id == region.tier_name_id)
					{
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"regions",
							region_label,
							"tier-name-id-invalid",
						]);
					}

					// Validate idle lobbies
					if let Some(idle_lobbies) = &region.idle_lobbies {
						if idle_lobbies.min_idle_lobbies > idle_lobbies.max_idle_lobbies {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"regions",
								region_label,
								"idle-lobbies",
								"min-gt-max",
							]);
						}
						if idle_lobbies.min_idle_lobbies > MAX_MIN_IDLE_LOBBY_COUNT {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"regions",
								region_label,
								"idle-lobbies",
								"min-too-high",
							]);
						}
						if idle_lobbies.max_idle_lobbies > MAX_MAX_IDLE_LOBBY_COUNT {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"regions",
								region_label,
								"idle-lobbies",
								"max-too-high",
							]);
						}
					}
				} else {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"game-modes",
						lobby_group_label,
						"regions",
						region_index,
						"invalid",
					]);
				}
			}

			if let Some(matchmaker::LobbyRuntime {
				runtime: Some(matchmaker::lobby_runtime::Runtime::Docker(docker_config)),
			}) = &lobby_group.runtime
			{
				// TODO: Validate build id belongs to the given game
				// Validate build id
				if docker_config.build_id.is_none() {
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"game-modes",
						lobby_group_label,
						"no-build",
					]);
				}

				// Validate args
				{
					let mut unique_args = HashSet::new();

					if docker_config.args.len() > 64 {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"args-too-many",
						]);
					}

					for (arg_index, arg) in docker_config.args.iter().take(64).enumerate() {
						if !arg.is_empty() {
							if arg.len() > 512 {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"args",
									arg_index,
									"too-long",
								]);
							} else {
								// Validate uniqueness
								if unique_args.contains(arg) {
									errors.push(util::err_path![
										"config",
										"matchmaker",
										"game-modes",
										lobby_group_label,
										"args",
										arg_index,
										"not-unique",
									]);
								} else {
									unique_args.insert(arg.clone());
								}
							}
						} else {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"args",
								arg_index,
								"invalid",
							]);
						}
					}
				}

				// Validate env vars
				{
					let mut unique_env_vars = HashSet::new();

					if docker_config.env_vars.len() > 64 {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"env-vars-too-many",
						]);
					}

					for env_var in docker_config.env_vars.iter().take(64) {
						let env_var_label = format!("*{}*", env_var.key);
						let mut valid = true;

						// Validate env var key
						if env_var.key.is_empty() {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"env-vars",
								env_var_label,
								"key",
								"invalid"
							]);

							valid = false;
						} else if env_var.key.len() > 64 {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"env-vars",
								env_var_label,
								"key",
								"too-long"
							]);

							valid = false;
						}

						// Validate env var value
						if env_var.value.is_empty() {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"env-vars",
								env_var_label,
								"value",
								"invalid"
							]);

							valid = false;
						} else if env_var.value.len() > 512 {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"env-vars",
								env_var_label,
								"value",
								"too-long"
							]);

							valid = false;
						}

						// Validate uniqueness
						if valid {
							if unique_env_vars.contains(&env_var.key) {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"env-vars",
									env_var_label,
									"key",
									"not-unique",
								]);
							} else {
								unique_env_vars.insert(env_var.key.clone());
							}
						}

						// Validate not provided by default
						let conflicts_default = util_mm::consts::DEFAULT_ENV_KEYS
							.iter()
							.any(|x| *x == env_var.key);
						let conflicts_port = docker_config.ports.iter().any(|port| {
							if port.target_port.is_some() {
								env_var.key == format!("PORT_{}", port.label.replace('-', "_"))
							} else if port.port_range.is_some() {
								env_var.key == format!("PORT_RANGE_{}_MIN", port.label)
									|| env_var.key == format!("PORT_RANGE_{}_MAX", port.label)
							} else {
								false
							}
						});
						if conflicts_default || conflicts_port {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"env-vars",
								env_var_label,
								"key",
								"conflicts-with-default",
							]);
						}
					}
				}

				let network_mode = unwrap!(LobbyRuntimeNetworkMode::from_i32(
					docker_config.network_mode
				));
				// Validate ports
				if !mm_game_config.host_networking_enabled
					&& matches!(network_mode, LobbyRuntimeNetworkMode::Host)
				{
					errors.push(util::err_path![
						"config",
						"matchmaker",
						"game-modes",
						lobby_group_label,
						"host-networking-disabled",
					]);
				} else {
					let mut unique_port_labels = HashSet::<String>::new();
					let mut unique_ports = HashSet::<(u32, i32)>::new();
					let mut ranges = Vec::<PortRange>::new();

					if docker_config.ports.len() > 16 {
						errors.push(util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"ports-too-many",
						]);
					}

					for port in docker_config.ports.iter().take(16) {
						let port_label = format!("*{}*", port.label);

						if util::check::ident(&port.label) {
							if unique_port_labels.contains(&port.label) {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"label-not-unique",
								]);
							} else {
								unique_port_labels.insert(port.label.clone());
							}
						} else {
							errors.push(util::err_path![
								"config",
								"matchmaker",
								"game-modes",
								lobby_group_label,
								"ports",
								port_label,
								"label-invalid",
							]);
						}

						let proxy_protocol =
							unwrap!(LobbyRuntimeProxyProtocol::from_i32(port.proxy_protocol));
						let proxy_kind = unwrap!(LobbyRuntimeProxyKind::from_i32(port.proxy_kind));

						// Validate ports unique
						if let Some(target_port) = port.target_port {
							if unique_ports.contains(&(target_port, port.proxy_protocol)) {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"port-protocol-not-unique",
								]);
							} else {
								unique_ports.insert((target_port, port.proxy_protocol));
							}
						}

						match (
							network_mode,
							proxy_kind,
							proxy_protocol,
							port.target_port,
							&port.port_range,
						) {
							// === No Proxy + Range ===
							(
								LobbyRuntimeNetworkMode::Host,
								LobbyRuntimeProxyKind::None,
								LobbyRuntimeProxyProtocol::Tcp | LobbyRuntimeProxyProtocol::Udp,
								None,
								Some(port_range),
							) => {
								let this_range = PortRange {
									proto: match proxy_protocol {
										LobbyRuntimeProxyProtocol::Tcp => PortRangeProtocol::Tcp,
										LobbyRuntimeProxyProtocol::Udp => PortRangeProtocol::Udp,
										_ => unreachable!(),
									},
									min: port_range.min,
									max: port_range.max,
								};

								// Validate port range
								if port_range.min > port_range.max {
									errors.push(util::err_path![
										"config",
										"matchmaker",
										"game-modes",
										lobby_group_label,
										"ports",
										port_label,
										"range-min-gt-max",
									]);
								}

								// Validate ranges
								if port_range.min < util_mm::consts::MIN_HOST_PORT as u32
									|| port_range.max > util_mm::consts::MAX_HOST_PORT as u32
								{
									errors.push(util::err_path![
										"config",
										"matchmaker",
										"game-modes",
										lobby_group_label,
										"ports",
										port_label,
										"port-out-of-range",
									]);
								} else if ranges.iter().any(|other_range| {
									this_range.proto == other_range.proto
										&& this_range.max >= other_range.min
										&& this_range.min <= other_range.max
								}) {
									errors.push(util::err_path![
										"config",
										"matchmaker",
										"game-modes",
										lobby_group_label,
										"ports",
										port_label,
										"ranges-overlap",
									]);
								}

								ranges.push(this_range);
							}

							// === Game Guard ===
							(
								_,
								LobbyRuntimeProxyKind::GameGuard,
								LobbyRuntimeProxyProtocol::Http
								| LobbyRuntimeProxyProtocol::Https
								| LobbyRuntimeProxyProtocol::Tcp
								| LobbyRuntimeProxyProtocol::TcpTls
								| LobbyRuntimeProxyProtocol::Udp,
								Some(_target_port),
								None,
							) => {
								// Valid
							}

							// === Error cases ===
							(_, _, _, Some(_), Some(_)) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"duplicate-port-and-port-range",
								]);
							}
							(_, _, _, None, None) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"missing-port-and-port-range",
								]);
							}
							(LobbyRuntimeNetworkMode::Host, _, _, Some(_), None) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"unsupported-target-port",
								]);
							}
							(_, LobbyRuntimeProxyKind::GameGuard, _, None, Some(_)) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"unsupported-port-range",
								]);
							}
							(
								LobbyRuntimeNetworkMode::Bridge,
								LobbyRuntimeProxyKind::None,
								_,
								_,
								_,
							) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"needs-host-network",
								]);
							}
							(
								LobbyRuntimeNetworkMode::Host,
								_,
								LobbyRuntimeProxyProtocol::Http
								| LobbyRuntimeProxyProtocol::Https
								| LobbyRuntimeProxyProtocol::TcpTls,
								_,
								_,
							) => {
								errors.push(util::err_path![
									"config",
									"matchmaker",
									"game-modes",
									lobby_group_label,
									"ports",
									port_label,
									"needs-game-guard",
								]);
							}
						}
					}
				}
			} else {
				errors.push(util::err_path![
					"config",
					"matchmaker",
					"game-modes",
					lobby_group_label,
					"no-runtime",
				]);
			}

			if let Some(actions) = &lobby_group.actions {
				// Validate find config
				if let Some(matchmaker::FindConfig {
					verification: Some(verification),
					..
				}) = &actions.find
				{
					let validation_res = op!([ctx] external_request_validate {
						config: Some(backend::net::ExternalRequestConfig {
							url: verification.url.clone(),
							headers: verification.headers.clone(),
							..Default::default()
						}),
					})
					.await?;

					// Append errors from external request validation
					errors.extend(validation_res.errors.iter().map(|err| {
						util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"find-config",
							"verification-config"
						]
						.into_iter()
						.chain(err.path.clone())
						.collect::<Vec<_>>()
					}));
				}

				// Validate join config
				if let Some(matchmaker::JoinConfig {
					verification: Some(verification),
					..
				}) = &actions.join
				{
					let validation_res = op!([ctx] external_request_validate {
						config: Some(backend::net::ExternalRequestConfig {
							url: verification.url.clone(),
							headers: verification.headers.clone(),
							..Default::default()
						}),
					})
					.await?;

					// Append errors from external request validation
					errors.extend(validation_res.errors.iter().map(|err| {
						util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"join-config",
							"verification-config"
						]
						.into_iter()
						.chain(err.path.clone())
						.collect::<Vec<_>>()
					}));
				}

				// Validate create config
				if let Some(matchmaker::CreateConfig {
					verification: Some(verification),
					..
				}) = &actions.create
				{
					let validation_res = op!([ctx] external_request_validate {
						config: Some(backend::net::ExternalRequestConfig {
							url: verification.url.clone(),
							headers: verification.headers.clone(),
							..Default::default()
						}),
					})
					.await?;

					// Append errors from external request validation
					errors.extend(validation_res.errors.iter().map(|err| {
						util::err_path![
							"config",
							"matchmaker",
							"game-modes",
							lobby_group_label,
							"join-config",
							"verification-config"
						]
						.into_iter()
						.chain(err.path.clone())
						.collect::<Vec<_>>()
					}));
				}
			}
		}
	} else {
		errors.push(util::err_path!["config", "matchmaker", "missing",]);
	}

	// KV config validation
	// if let Some(kv) = &proto_config.kv {

	// }

	// TODO: Add this back with the new API models
	// Identity config validation
	// if let Some(identity) = &proto_config.identity {
	// 	if identity.custom_display_names.is_empty() {
	// 		errors.push(util::err_path![
	// 			"config",
	// 			"identity",
	// 			"custom-display-names-meta",
	// 			"too-few",
	// 		]);
	// 	} else if identity.custom_display_names.len() > 10 {
	// 		errors.push(util::err_path![
	// 			"config",
	// 			"identity",
	// 			"custom-display-names-meta",
	// 			"too-many",
	// 		]);
	// 	}

	// 	let mut unique_custom_display_names = HashSet::<String>::new();
	// 	for (c_index, custom_display_name) in
	// 		identity.custom_display_names.iter().take(32).enumerate()
	// 	{
	// 		// Display name validation
	// 		if custom_display_name.display_name.is_empty() {
	// 			errors.push(util::err_path![
	// 				"config",
	// 				"identity",
	// 				"custom-display-names",
	// 				c_index,
	// 				"display-name",
	// 				"too-short"
	// 			]);
	// 		} else if custom_display_name.display_name.len() > MAX_CUSTOM_DISPLAY_NAME_LEN {
	// 			errors.push(util::err_path![
	// 				"config",
	// 				"identity",
	// 				"custom-display-names",
	// 				c_index,
	// 				"display-name",
	// 				"too-long"
	// 			]);
	// 		}

	// 		if util::check::display_name(&custom_display_name.display_name) {
	// 			let profanity_res = op!([ctx] profanity_check {
	// 				strings: vec![custom_display_name.display_name.clone()],
	// 				censor: false,
	// 			})
	// 			.await?;

	// 			if *unwrap!(profanity_res.results.first()) {
	// 				errors.push(util::err_path![
	// 					"config",
	// 					"identity",
	// 					"custom-display-names",
	// 					c_index,
	// 					"display-name",
	// 					"invalid"
	// 				]);
	// 			}

	// 			if unique_custom_display_names.contains(&custom_display_name.display_name) {
	// 				errors.push(util::err_path![
	// 					"config",
	// 					"identity",
	// 					"custom-display-names",
	// 					c_index,
	// 					"display-name",
	// 					"not-unique"
	// 				]);
	// 			} else {
	// 				unique_custom_display_names.insert(custom_display_name.display_name.clone());
	// 			}
	// 		} else {
	// 			errors.push(util::err_path![
	// 				"config",
	// 				"identity",
	// 				"custom-display-names",
	// 				c_index,
	// 				"display-name",
	// 				"invalid"
	// 			]);
	// 		}
	// 	}

	// 	if identity.custom_avatars.len() > 10 {
	// 		errors.push(util::err_path![
	// 			"config",
	// 			"identity",
	// 			"custom-avatars-meta",
	// 			"too-many",
	// 		]);
	// 	}

	// TODO: Validate upload ids belong to the given game
	// for (avatar_index, custom_avatar) in identity.custom_avatars.iter().take(32).enumerate() {
	// 	let upload_id = unwrap_ref!(custom_avatar.upload_id);
	// }
	// }

	Ok(game::version_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| common::ValidationError { path })
			.collect::<Vec<_>>(),
	})
}
