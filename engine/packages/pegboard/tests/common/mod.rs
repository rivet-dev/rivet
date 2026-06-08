use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::keys;
use std::io::Write;
use universaldb::utils::IsolationLevel::Serializable;
use xxhash_rust::xxh3::xxh3_128_with_seed;

pub const POOL_NAME: &str = "test-pool";
pub const VERSION: u32 = 7;
pub const HASH_NOW: i64 = 1_000_000;
pub const HASH_ELIGIBLE_THRESHOLD: i64 = 10_000;

#[derive(Debug, Clone)]
pub struct HashEnvoyRegistration {
	pub envoy_key: String,
	pub hash_positions: Vec<[u8; 16]>,
	pub slots: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct HashAllocationInput {
	pub pivots: Vec<[u8; 16]>,
	pub rng_seed: u64,
}

#[derive(Debug, Clone)]
pub struct EnvoyFixture {
	pub namespace_id: Id,
	pub envoy_key: String,
	pub pool_name: String,
	pub version: u32,
	pub create_ts: i64,
	pub last_ping_ts: i64,
	pub virtual_nodes: Option<u8>,
}

#[derive(Debug)]
pub struct EnvoyKeyState {
	pub pool_name: bool,
	pub version: bool,
	pub create_ts: bool,
	pub last_ping_ts: bool,
	pub expired_ts: bool,
	pub virtual_nodes: bool,
	pub load_balancer_idx: bool,
	pub active_envoy: bool,
	pub active_envoy_by_name: bool,
	pub hash_entries: usize,
}

pub fn hash_pos(value: u128) -> [u8; 16] {
	value.to_be_bytes()
}

pub fn deterministic_envoy_key(index: usize) -> String {
	format!("envoy-{index:03}")
}

pub fn deterministic_hash_pos(
	envoy_index: usize,
	virtual_node_index: usize,
	envoy_count: usize,
	virtual_node_count: usize,
) -> [u8; 16] {
	let total_positions = envoy_count * virtual_node_count;
	let position_index = virtual_node_index * envoy_count + envoy_index;
	((position_index as u128) * (u128::MAX / total_positions as u128)).to_be_bytes()
}

pub fn deterministic_hash_positions(
	envoy_index: usize,
	envoy_count: usize,
	virtual_node_count: usize,
) -> Vec<[u8; 16]> {
	(0..virtual_node_count)
		.map(|virtual_node_index| {
			deterministic_hash_pos(
				envoy_index,
				virtual_node_index,
				envoy_count,
				virtual_node_count,
			)
		})
		.collect()
}

pub fn count_deterministic_envoy_allocations(
	allocations: Vec<Option<String>>,
	envoy_count: usize,
) -> Result<Vec<usize>> {
	let mut counts = vec![0usize; envoy_count];

	for allocation in allocations {
		let envoy_key = allocation.context("expected hash allocator to choose an envoy")?;
		let index = envoy_key
			.strip_prefix("envoy-")
			.context("expected deterministic envoy key prefix")?
			.parse::<usize>()?;
		let count = counts
			.get_mut(index)
			.context("expected deterministic envoy index in range")?;
		*count += 1;
	}

	Ok(counts)
}

pub fn fresh_ping_ts() -> i64 {
	HASH_NOW
}

pub fn stale_ping_ts() -> i64 {
	HASH_NOW - HASH_ELIGIBLE_THRESHOLD - 1
}

pub fn unique_pool_name(prefix: &str) -> String {
	format!("{prefix}-{}", Uuid::new_v4())
}

pub async fn setup_deps() -> Result<rivet_test_deps::TestDeps> {
	let _ = tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_target(false)
		.try_init();

	rivet_test_deps::TestDeps::new().await
}

pub async fn load_hash_config(hash_fields: &str) -> Result<rivet_config::Config> {
	let dir = tempfile::tempdir()?;
	let path = dir.path().join("config.json5");
	let mut file = std::fs::File::create(&path)?;
	write!(
		file,
		r#"{{
			pegboard: {{
				envoy_load_balancer: {{
					hash: {{
						{hash_fields}
					}}
				}}
			}}
		}}"#
	)?;
	drop(file);

	rivet_config::Config::load(&[path]).await
}

pub async fn write_envoy(
	test_deps: &rivet_test_deps::TestDeps,
	last_ping_ts: i64,
	virtual_nodes: Option<u8>,
) -> Result<EnvoyFixture> {
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let envoy_key = Uuid::new_v4().to_string();
	let pool_name = POOL_NAME.to_string();
	let create_ts = util::timestamp::now();
	let version = VERSION;

	let fixture = EnvoyFixture {
		namespace_id,
		envoy_key,
		pool_name,
		version,
		create_ts,
		last_ping_ts,
		virtual_nodes,
	};

	write_envoy_fixture(test_deps, &fixture).await?;

	Ok(fixture)
}

pub async fn write_conn_init_registration(
	test_deps: &rivet_test_deps::TestDeps,
	virtual_nodes: u8,
) -> Result<EnvoyFixture> {
	write_envoy(test_deps, util::timestamp::now(), Some(virtual_nodes)).await
}

pub async fn write_hash_envoy(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
	envoy_key: &str,
	last_ping_ts: i64,
	hash_positions: Vec<[u8; 16]>,
	slots: Option<i64>,
) -> Result<()> {
	let pool_name = pool_name.to_string();
	let envoy_key = envoy_key.to_string();
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let pool_name = pool_name.clone();
			let envoy_key = envoy_key.clone();
			let hash_positions = hash_positions.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				let create_ts = HASH_NOW;

				tx.write(
					&keys::envoy::PoolNameKey::new(namespace_id, envoy_key.clone()),
					pool_name.clone(),
				)?;
				tx.write(
					&keys::envoy::VersionKey::new(namespace_id, envoy_key.clone()),
					VERSION,
				)?;
				tx.write(
					&keys::envoy::CreateTsKey::new(namespace_id, envoy_key.clone()),
					create_ts,
				)?;
				tx.write(
					&keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.clone()),
					last_ping_ts,
				)?;
				tx.write(
					&keys::envoy::VirtualNodesKey::new(namespace_id, envoy_key.clone()),
					hash_positions.len() as u8,
				)?;
				tx.write(
					&keys::ns::EnvoyLoadBalancerIdxKey::new(
						namespace_id,
						pool_name.clone(),
						VERSION,
						last_ping_ts,
						envoy_key.clone(),
					),
					(),
				)?;
				tx.write(
					&keys::ns::ActiveEnvoyKey::new(namespace_id, create_ts, envoy_key.clone()),
					(),
				)?;
				tx.write(
					&keys::ns::ActiveEnvoyByNameKey::new(
						namespace_id,
						pool_name.clone(),
						create_ts,
						envoy_key.clone(),
					),
					(),
				)?;

				if let Some(slots) = slots {
					tx.write(
						&keys::envoy::SlotsKey::new(namespace_id, envoy_key.clone()),
						slots,
					)?;
				}

				for hash_pos in hash_positions {
					tx.write(
						&keys::ns::EnvoyHashIdxKey::new(
							namespace_id,
							pool_name.clone(),
							VERSION,
							hash_pos,
							envoy_key.clone(),
						),
						(),
					)?;
				}

				Ok(())
			}
		})
		.await
}

pub async fn write_hash_envoys(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
	envoys: Vec<HashEnvoyRegistration>,
) -> Result<()> {
	let pool_name = pool_name.to_string();
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let pool_name = pool_name.clone();
			let envoys = envoys.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());

				for (idx, envoy) in envoys.into_iter().enumerate() {
					let create_ts = HASH_NOW + idx as i64;

					tx.write(
						&keys::envoy::PoolNameKey::new(namespace_id, envoy.envoy_key.clone()),
						pool_name.clone(),
					)?;
					tx.write(
						&keys::envoy::VersionKey::new(namespace_id, envoy.envoy_key.clone()),
						VERSION,
					)?;
					tx.write(
						&keys::envoy::CreateTsKey::new(namespace_id, envoy.envoy_key.clone()),
						create_ts,
					)?;
					tx.write(
						&keys::envoy::LastPingTsKey::new(namespace_id, envoy.envoy_key.clone()),
						fresh_ping_ts(),
					)?;
					tx.write(
						&keys::envoy::VirtualNodesKey::new(namespace_id, envoy.envoy_key.clone()),
						envoy.hash_positions.len() as u8,
					)?;
					tx.write(
						&keys::ns::EnvoyLoadBalancerIdxKey::new(
							namespace_id,
							pool_name.clone(),
							VERSION,
							fresh_ping_ts(),
							envoy.envoy_key.clone(),
						),
						(),
					)?;
					tx.write(
						&keys::ns::ActiveEnvoyKey::new(
							namespace_id,
							create_ts,
							envoy.envoy_key.clone(),
						),
						(),
					)?;
					tx.write(
						&keys::ns::ActiveEnvoyByNameKey::new(
							namespace_id,
							pool_name.clone(),
							create_ts,
							envoy.envoy_key.clone(),
						),
						(),
					)?;

					if let Some(slots) = envoy.slots {
						tx.write(
							&keys::envoy::SlotsKey::new(namespace_id, envoy.envoy_key.clone()),
							slots,
						)?;
					}

					for hash_pos in envoy.hash_positions {
						tx.write(
							&keys::ns::EnvoyHashIdxKey::new(
								namespace_id,
								pool_name.clone(),
								VERSION,
								hash_pos,
								envoy.envoy_key.clone(),
							),
							(),
						)?;
					}
				}

				Ok(())
			}
		})
		.await
}

pub async fn allocate_hash(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
	samples: u8,
	max_scan: u32,
	pivots: Vec<[u8; 16]>,
	rng_seed: u64,
) -> Result<(
	Option<String>,
	pegboard::workflows::actor2::HashAllocatorReadStats,
)> {
	// slot_jitter = 0 keeps the seeded RNG path deterministic so the existing
	// tests assert exact envoy choices.
	allocate_hash_with_jitter(
		test_deps,
		namespace_id,
		pool_name,
		samples,
		max_scan,
		0,
		pivots,
		rng_seed,
	)
	.await
}

pub async fn allocate_hash_with_jitter(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
	samples: u8,
	max_scan: u32,
	slot_jitter: u8,
	pivots: Vec<[u8; 16]>,
	rng_seed: u64,
) -> Result<(
	Option<String>,
	pegboard::workflows::actor2::HashAllocatorReadStats,
)> {
	let pools = test_deps.pools().clone();
	let pool_name = pool_name.to_string();
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let pools = pools.clone();
			let pool_name = pool_name.clone();
			let pivots = pivots.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				pegboard::workflows::actor2::allocate_hash_for_tests(
					namespace_id,
					&pool_name,
					&tx,
					&pools,
					HASH_NOW,
					HASH_ELIGIBLE_THRESHOLD,
					samples,
					max_scan,
					slot_jitter,
					true,
					pivots,
					rng_seed,
				)
				.await
			}
		})
		.await
}

pub async fn allocate_hash_batch(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
	samples: u8,
	max_scan: u32,
	allocations: Vec<HashAllocationInput>,
) -> Result<Vec<Option<String>>> {
	let pools = test_deps.pools().clone();
	let pool_name = pool_name.to_string();
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let pools = pools.clone();
			let pool_name = pool_name.clone();
			let allocations = allocations.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				let mut results = Vec::with_capacity(allocations.len());

				for allocation in allocations {
					let (result, _) = pegboard::workflows::actor2::allocate_hash_for_tests(
						namespace_id,
						&pool_name,
						&tx,
						&pools,
						HASH_NOW,
						HASH_ELIGIBLE_THRESHOLD,
						samples,
						max_scan,
						0,
						true,
						allocation.pivots,
						allocation.rng_seed,
					)
					.await?;
					results.push(result);
				}

				Ok(results)
			}
		})
		.await
}

pub async fn read_virtual_nodes(
	test_deps: &rivet_test_deps::TestDeps,
	fixture: &EnvoyFixture,
) -> Result<Option<u8>> {
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let fixture = fixture.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.read_opt(
					&keys::envoy::VirtualNodesKey::new(
						fixture.namespace_id,
						fixture.envoy_key.clone(),
					),
					Serializable,
				)
				.await
			}
		})
		.await
}

pub async fn mark_expired(
	test_deps: &rivet_test_deps::TestDeps,
	fixture: &EnvoyFixture,
) -> Result<()> {
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let fixture = fixture.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(
					&keys::envoy::ExpiredTsKey::new(fixture.namespace_id, fixture.envoy_key),
					util::timestamp::now(),
				)?;
				Ok(())
			}
		})
		.await
}

pub async fn expire(
	test_deps: &rivet_test_deps::TestDeps,
	fixture: &EnvoyFixture,
	skip_if_fresh: bool,
) -> Result<pegboard::ops::envoy::expire::Output> {
	pegboard::ops::envoy::expire::expire_with_pools(
		test_deps.config(),
		test_deps.pools(),
		&pegboard::ops::envoy::expire::Input {
			namespace_id: fixture.namespace_id,
			envoy_key: fixture.envoy_key.clone(),
			skip_if_fresh,
		},
	)
	.await
}

pub async fn read_key_state(
	test_deps: &rivet_test_deps::TestDeps,
	fixture: &EnvoyFixture,
	hash_positions_to_check: u8,
) -> Result<EnvoyKeyState> {
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let fixture = fixture.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				let mut hash_entries = 0;

				for i in 0..hash_positions_to_check {
					let exists = tx
						.exists(
							&keys::ns::EnvoyHashIdxKey::new(
								fixture.namespace_id,
								fixture.pool_name.clone(),
								fixture.version,
								xxh3_128_with_seed(fixture.envoy_key.as_bytes(), i as u64)
									.to_be_bytes(),
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?;
					if exists {
						hash_entries += 1;
					}
				}

				Ok(EnvoyKeyState {
					pool_name: tx
						.exists(
							&keys::envoy::PoolNameKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					version: tx
						.exists(
							&keys::envoy::VersionKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					create_ts: tx
						.exists(
							&keys::envoy::CreateTsKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					last_ping_ts: tx
						.exists(
							&keys::envoy::LastPingTsKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					expired_ts: tx
						.exists(
							&keys::envoy::ExpiredTsKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					virtual_nodes: tx
						.exists(
							&keys::envoy::VirtualNodesKey::new(
								fixture.namespace_id,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					load_balancer_idx: tx
						.exists(
							&keys::ns::EnvoyLoadBalancerIdxKey::new(
								fixture.namespace_id,
								fixture.pool_name.clone(),
								fixture.version,
								fixture.last_ping_ts,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					active_envoy: tx
						.exists(
							&keys::ns::ActiveEnvoyKey::new(
								fixture.namespace_id,
								fixture.create_ts,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					active_envoy_by_name: tx
						.exists(
							&keys::ns::ActiveEnvoyByNameKey::new(
								fixture.namespace_id,
								fixture.pool_name.clone(),
								fixture.create_ts,
								fixture.envoy_key.clone(),
							),
							Serializable,
						)
						.await?,
					hash_entries,
				})
			}
		})
		.await
}

pub fn assert_registration_keys_present(state: &EnvoyKeyState, expected_hash_entries: usize) {
	assert!(state.pool_name, "PoolNameKey should exist");
	assert!(state.version, "VersionKey should exist");
	assert!(state.create_ts, "CreateTsKey should exist");
	assert!(state.last_ping_ts, "LastPingTsKey should exist");
	assert!(
		state.load_balancer_idx,
		"EnvoyLoadBalancerIdxKey should exist"
	);
	assert!(state.active_envoy, "ActiveEnvoyKey should exist");
	assert!(
		state.active_envoy_by_name,
		"ActiveEnvoyByNameKey should exist"
	);
	assert_eq!(state.hash_entries, expected_hash_entries);
}

async fn write_envoy_fixture(
	test_deps: &rivet_test_deps::TestDeps,
	fixture: &EnvoyFixture,
) -> Result<()> {
	test_deps
		.pools()
		.udb()?
		.txn("test_pegboardcommon_mod", |tx| {
			let fixture = fixture.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());

				tx.write(
					&keys::envoy::PoolNameKey::new(fixture.namespace_id, fixture.envoy_key.clone()),
					fixture.pool_name.clone(),
				)?;
				tx.write(
					&keys::envoy::VersionKey::new(fixture.namespace_id, fixture.envoy_key.clone()),
					fixture.version,
				)?;
				tx.write(
					&keys::envoy::CreateTsKey::new(fixture.namespace_id, fixture.envoy_key.clone()),
					fixture.create_ts,
				)?;
				tx.write(
					&keys::envoy::LastPingTsKey::new(
						fixture.namespace_id,
						fixture.envoy_key.clone(),
					),
					fixture.last_ping_ts,
				)?;
				tx.write(
					&keys::ns::EnvoyLoadBalancerIdxKey::new(
						fixture.namespace_id,
						fixture.pool_name.clone(),
						fixture.version,
						fixture.last_ping_ts,
						fixture.envoy_key.clone(),
					),
					(),
				)?;
				tx.write(
					&keys::ns::ActiveEnvoyKey::new(
						fixture.namespace_id,
						fixture.create_ts,
						fixture.envoy_key.clone(),
					),
					(),
				)?;
				tx.write(
					&keys::ns::ActiveEnvoyByNameKey::new(
						fixture.namespace_id,
						fixture.pool_name.clone(),
						fixture.create_ts,
						fixture.envoy_key.clone(),
					),
					(),
				)?;

				if let Some(virtual_nodes) = fixture.virtual_nodes {
					tx.write(
						&keys::envoy::VirtualNodesKey::new(
							fixture.namespace_id,
							fixture.envoy_key.clone(),
						),
						virtual_nodes,
					)?;

					for i in 0..virtual_nodes {
						tx.write(
							&keys::ns::EnvoyHashIdxKey::new(
								fixture.namespace_id,
								fixture.pool_name.clone(),
								fixture.version,
								xxh3_128_with_seed(fixture.envoy_key.as_bytes(), i as u64)
									.to_be_bytes(),
								fixture.envoy_key.clone(),
							),
							(),
						)?;
					}
				}

				Ok(())
			}
		})
		.await
}
