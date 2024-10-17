use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rand::prelude::*;
use rivet_operation::prelude::*;
use tokio::time::Instant;

const PARALLEL_WORKERS: usize = 1;

pub async fn start() -> GlobalResult<()> {
	// TODO: Handle ctrl-c

	let pools = rivet_pools::from_env().await?;

	let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 15));
	loop {
		interval.tick().await;

		run_from_env(util::timestamp::now()).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(_ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env().await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-mm-sustain");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-mm-sustain".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
	);

	// Region
	let region_res = op!([ctx] faker_region {}).await?;
	let region_id = unwrap!(region_res.region_id.as_ref()).as_uuid();

	// Game
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await?;

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyEcho as i32,
	})
	.await?;
	let build_id = build_res.build_id.unwrap().as_uuid();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![
				// lobby_group_config(build_id, region_id, "basic-4d1"),
				// lobby_group_config(build_id, region_id, "basic-2d1"),
				// lobby_group_config(build_id, region_id, "basic-1d1"),
				lobby_group_config(build_id, region_id, "basic-1d2"),
				lobby_group_config(build_id, region_id, "basic-1d4"),
				lobby_group_config(build_id, region_id, "basic-1d8"),
				lobby_group_config(build_id, region_id, "basic-1d16"),
			],
		}),
		..Default::default()
	})
	.await?;

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![unwrap!(game_version_res.version_id)],
	})
	.await?;
	let lobby_groups = &unwrap!(unwrap!(version_get_res.versions.first())
		.config_meta
		.as_ref())
	.lobby_groups;
	let lobby_group_ids = lobby_groups
		.iter()
		.map(|x| x.lobby_group_id.unwrap().as_uuid())
		.collect::<Vec<_>>();

	let ns_create_res = op!([ctx] faker_game_namespace {
		game_id: game_res.game_id,
		version_id: game_version_res.version_id,
		override_name_id: "prod".to_owned(),
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = ns_create_res.namespace_id.unwrap().as_uuid();

	let mut handles = Vec::new();

	for i in 0..PARALLEL_WORKERS {
		handles.push(tokio::spawn(run_lobby_worker(
			ctx.clone(),
			i,
			namespace_id,
			region_id,
			lobby_group_ids.clone(),
		)));
	}

	for handle in handles {
		handle.await?;
	}

	Ok(())
}

fn lobby_group_config(
	build_id: Uuid,
	region_id: Uuid,
	tier: &str,
) -> backend::matchmaker::LobbyGroup {
	backend::matchmaker::LobbyGroup {
		name_id: util::faker::ident(),

		regions: vec![backend::matchmaker::lobby_group::Region {
			region_id: Some(region_id.into()),
			tier_name_id: tier.to_owned(),
			idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
				min_idle_lobbies: 0,
				// Don't auto-destroy lobbies from tests
				max_idle_lobbies: 32,
			}),
		}],
		max_players_normal: 8,
		max_players_direct: 10,
		max_players_party: 12,
		listable: true,
		taggable: false,
		allow_dynamic_max_players: false,

		runtime: Some(
			backend::matchmaker::lobby_runtime::Docker {
				build_id: Some(build_id.into()),
				args: Vec::new(),
				env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
					key: "HELLO".into(),
					value: "world".into(),
				}],
				network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
				ports: vec![
					backend::matchmaker::lobby_runtime::Port {
						label: "test-http".into(),
						target_port: Some(8001),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http
							as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					},
					backend::matchmaker::lobby_runtime::Port {
						label: "test-tcp".into(),
						target_port: Some(8002),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp
							as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					},
					backend::matchmaker::lobby_runtime::Port {
						label: "test-udp".into(),
						target_port: Some(8003),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp
							as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					},
				],
			}
			.into(),
		),

		actions: None,
	}
}

async fn run_lobby_worker(
	ctx: OperationContext<()>,
	_worker_idx: usize,
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_ids: Vec<Uuid>,
) {
	loop {
		// Choose random lobby group id
		let lobby_group_id = {
			let mut rng = thread_rng();
			*lobby_group_ids.choose(&mut rng).unwrap()
		};

		let start = Instant::now();
		let lobby_id = Uuid::new_v4();
		match run_lobby_lifecycle(&ctx, lobby_id, namespace_id, region_id, lobby_group_id).await {
			Ok(_) => {
				tracing::info!(duration = %start.elapsed().as_secs_f64(), "lobby lifecycle success")
			}
			Err(err) => {
				tracing::error!(duration = %start.elapsed().as_secs_f64(), ?err, "lobby lifecycle fail")
			}
		}

		// Shut down lobby
		tracing::info!(?lobby_id, "shutting down lobby");
		msg!([ctx] mm::msg::lobby_stop(lobby_id) -> mm::msg::lobby_cleanup_complete(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await
		.unwrap();
	}
}

async fn run_lobby_lifecycle(
	ctx: &OperationContext<()>,
	lobby_id: Uuid,
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
) -> GlobalResult<()> {
	// Create lobby
	tracing::info!(?lobby_id, "creating lobby");
	let res = msg!([ctx] @notrace mm::msg::lobby_create(lobby_id) -> Result<mm::msg::lobby_ready_complete, mm::msg::lobby_create_fail> {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(namespace_id.into()),
		lobby_group_id: Some(lobby_group_id.into()),
		region_id: Some(region_id.into()),
		create_ray_id: None,
		preemptively_created: false,
		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
		tags: HashMap::new(),
		dynamic_max_players: None,
		parameters: util::env::test_id_param(),
	})
	.await?;

	if let Err(err) = res {
		bail!(format!("{err:?}"));
	}

	// Test HTTP connectivity
	let (hostname, _) = get_lobby_addr(ctx, lobby_id, "test-http").await?;
	tracing::info!("testing http to {}", hostname);

	// Echo body
	let random_body = Uuid::new_v4().to_string();
	let client = reqwest::Client::new();
	let res = client
		.post(format!("http://{hostname}"))
		.body(random_body.clone())
		.send()
		.await?
		.error_for_status()?;
	let res_text = res.text().await?;
	ensure_eq!(random_body, res_text, "echoed wrong response");

	// Used to pause on when a gateway timeout is encountered
	// let random_body = Uuid::new_v4().to_string();
	// let client = reqwest::Client::new();
	// let res = client
	// 	.post(format!("http://{hostname}"))
	// 	.body(random_body.clone())
	// 	.send()
	// 	.await?;
	// if res.status() == reqwest::StatusCode::GATEWAY_TIMEOUT {
	// 	let lobby_res = op!([ctx] mm_lobby_get {
	// 		lobby_ids: vec![lobby_id.into()],
	// 	})
	// 	.await?;
	// 	let lobby = unwrap!(lobby_res.lobbies.first());

	// 	let run_res = op!([ctx] job_run::ops::get {
	// 		run_ids: vec![unwrap!(lobby.run_id)],
	// 	})
	// 	.await?;
	// 	let run = unwrap!(run_res.runs.first());
	// 	let run_meta = unwrap_ref!(run.run_meta);
	// 	let Some(backend::job::run_meta::Kind::Nomad(nomad)) = run_meta.kind.as_ref() else {
	// 		unreachable!()
	// 	};

	// 	let url = format!(
	// 		"http://localhost:4646/ui/allocations/{}",
	// 		unwrap_ref!(nomad.alloc_id)
	// 	);

	// 	tracing::error!(?lobby_id, alloc_url = %url, "found gateway timeout, waiting forever");

	// 	std::future::pending::<()>().await;
	// }
	//
	// let res_text = res.text().await?;
	// ensure_eq!(random_body, res_text, "echoed wrong response");

	Ok(())
}

/// Fetches the address to get the lobby from.
async fn get_lobby_addr(
	ctx: &OperationContext<()>,
	lobby_id: Uuid,
	port: &str,
) -> GlobalResult<(String, u16)> {
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		include_stopped: true,
	})
	.await?;
	let lobby = unwrap!(lobby_res.lobbies.first());

	ensure_with!(lobby.stop_ts.is_none(), MATCHMAKER_LOBBY_STOPPED);

	let run_id = unwrap!(lobby.run_id);

	let run_res = op!([ctx] job_run::ops::get { run_ids: vec![run_id] }).await?;
	let run = unwrap!(run_res.runs.first());

	let port = unwrap!(run
		.proxied_ports
		.iter()
		.find(|x| x.target_nomad_port_label == Some(util_mm::format_nomad_port_label(port))));

	Ok((
		unwrap!(port.ingress_hostnames.first()).clone(),
		port.ingress_port as u16,
	))
}
