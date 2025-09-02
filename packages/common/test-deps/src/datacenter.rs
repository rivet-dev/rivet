use anyhow::*;
use uuid::Uuid;

use crate::TestDeps;
use rivet_test_deps_docker::{TestDatabase, TestPubSub};

/// Datacenters for the same test ID & datacenter label will preserve the same storage if stopped
/// and started again. This is helpful for testing restarting datacenters with new configs.
pub async fn setup_single_datacenter(
	test_id: Uuid,
	dc_label: u16,
	datacenters: Vec<rivet_config::config::topology::Datacenter>,
	api_peer_port: u16,
	guard_port: u16,
) -> Result<TestDeps> {
	let test_database = TestDatabase::from_env();
	let test_pubsub = TestPubSub::from_env();

	let dc = datacenters
		.iter()
		.find(|x| x.datacenter_label == dc_label)
		.expect("invalid dc label")
		.clone();

	tracing::info!(
		dc = dc.datacenter_label,
		?test_database,
		?test_pubsub,
		"setting up test dependencies with configuration"
	);

	// Setup database
	let mut container_names = Vec::new();
	let (db_config, mut db_docker_config) =
		test_database.config(test_id, dc.datacenter_label).await?;
	if let Some(docker_config) = &mut db_docker_config {
		let was_started = docker_config.start().await?;
		container_names.push(docker_config.container_name.clone());

		// If Postgres was just started, wait for it to be ready
		if was_started && test_database == TestDatabase::Postgres {
			tracing::info!(
				dc = dc.datacenter_label,
				port = docker_config.port_mapping.0,
				"waiting for Postgres to be ready"
			);
			TestDatabase::wait_for_postgres_ready(docker_config.port_mapping.0, 10).await?;
		}
	}

	// Setup pubsub
	let (pubsub_config, mut pubsub_docker_config) =
		test_pubsub.config(test_id, dc.datacenter_label).await?;
	if let Some(docker_config) = &mut pubsub_docker_config {
		docker_config.start().await?;
		container_names.push(docker_config.container_name.clone());
	}

	tracing::info!(
		dc = dc.datacenter_label,
		"containers started, waiting for services to be ready"
	);

	// Pick ports for other services
	// TODO: Race condition with picking before binding
	let api_public_port = portpicker::pick_unused_port().context("api_public_port")?;
	let pegboard_port = portpicker::pick_unused_port().context("pegboard_port")?;

	tracing::info!(
		dc = dc.datacenter_label,
		api_public_port,
		api_peer_port,
		pegboard_port,
		guard_port,
		"using ports for test services"
	);

	// Setup config
	let mut root = rivet_config::config::Root::default();
	root.database = Some(db_config);
	root.pubsub = Some(pubsub_config);
	root.api_public = Some(rivet_config::config::ApiPublic {
		port: Some(api_public_port),
		..Default::default()
	});
	root.api_peer = Some(rivet_config::config::ApiPeer {
		port: Some(api_peer_port),
		..Default::default()
	});
	root.pegboard = Some(rivet_config::config::Pegboard {
		port: Some(pegboard_port),
		..Default::default()
	});

	root.topology = Some(rivet_config::config::topology::Topology {
		datacenter_label: dc.datacenter_label,
		datacenters,
	});

	root.guard = Some(rivet_config::config::guard::Guard {
		host: None,
		port: Some(guard_port),
		https: None,
	});

	tracing::info!(
		dc = dc.datacenter_label,
		"creating test configuration and pools"
	);
	let config = rivet_config::Config::from_root(root);
	let pools = rivet_pools::Pools::test(config.clone()).await?;

	tracing::info!(dc = dc.datacenter_label, "test dependencies setup complete");
	tracing::info!(dc = dc.datacenter_label, config = ?*config, "test dependencies config");
	Ok(TestDeps {
		pools,
		config,
		container_names,
		api_public_port,
		api_peer_port,
		guard_port,
		pegboard_port,
		stop_docker_containers_on_drop: true,
	})
}
