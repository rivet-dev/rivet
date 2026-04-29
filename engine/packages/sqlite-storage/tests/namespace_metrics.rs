mod support;

use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use namespace::{
	keys::metric::{Metric, MetricKey},
	types::SqliteNamespaceConfig,
};
use rivet_metrics::REGISTRY;
use sqlite_storage::{
	compactor::{CompactorConfig, SqliteCompactPayload, metrics, worker},
	keys::meta_checkpoints_key,
	types::{decode_checkpoints, encode_checkpoints},
};
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};

const ACTOR_ID: &str = "namespace-metrics-actor";
const ACTOR_NAME: &str = "namespace-metrics";

async fn seed_namespace_config(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	config: SqliteNamespaceConfig,
) -> Result<()> {
	db.run(move |tx| {
		let config = config.clone();
		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			tx.write(&namespace::keys::sqlite_config_key(namespace_id), config)?;
			Ok(())
		}
	})
	.await
}

async fn read_metric(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	metric: Metric,
) -> Result<i64> {
	db.run(move |tx| {
		let metric = metric.clone();
		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			Ok(tx
				.read_opt(&MetricKey::new(namespace_id, metric), Snapshot)
				.await?
				.unwrap_or(0))
		}
	})
	.await
}

async fn write_metric(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	metric: Metric,
	value: i64,
) -> Result<()> {
	db.run(move |tx| {
		let metric = metric.clone();
		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			namespace::keys::metric::inc(&tx, namespace_id, metric, value);
			Ok(())
		}
	})
	.await
}

async fn set_checkpoint_refcount(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	refcount: u32,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let checkpoints_key = meta_checkpoints_key(&actor_id);
			let checkpoints_bytes = tx
				.informal()
				.get(&checkpoints_key, Serializable)
				.await?
				.expect("checkpoint index should exist");
			let mut checkpoints = decode_checkpoints(&checkpoints_bytes)?;
			for entry in &mut checkpoints.entries {
				entry.refcount = refcount;
			}
			tx.informal()
				.set(&checkpoints_key, &encode_checkpoints(checkpoints)?);
			Ok(())
		}
	})
	.await
}

fn pitr_config() -> CompactorConfig {
	CompactorConfig {
		pitr_enabled: true,
		..CompactorConfig::default()
	}
}

async fn run_payload(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	actor_id: &str,
	actor_name: &str,
) -> Result<()> {
	run_payload_with_config(db, namespace_id, actor_id, actor_name, pitr_config()).await
}

async fn run_payload_with_config(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	actor_id: &str,
	actor_name: &str,
	compactor_config: CompactorConfig,
) -> Result<()> {
	worker::test_hooks::handle_payload_once(
		db,
		SqliteCompactPayload {
			actor_id: actor_id.to_string(),
			namespace_id: Some(namespace_id),
			actor_name: Some(actor_name.to_string()),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		compactor_config,
		CancellationToken::new(),
	)
	.await
}

#[tokio::test]
async fn metric_key_variants_compile() -> Result<()> {
	let db = support::test_db("sqlite-namespace-metric-keys-").await?;
	let namespace_id = Id::new_v1(4001);
	let actor_name = "actor-a";
	let variants = vec![
		Metric::SqliteStorageLiveUsed(actor_name.to_string()),
		Metric::SqliteStoragePitrUsed(actor_name.to_string()),
		Metric::SqliteCheckpointCount(actor_name.to_string()),
		Metric::SqliteCheckpointPinned(actor_name.to_string()),
	];

	for (idx, metric) in variants.into_iter().enumerate() {
		let value = i64::try_from(idx + 1)?;
		write_metric(Arc::clone(&db), namespace_id, metric.clone(), value).await?;
		assert_eq!(read_metric(Arc::clone(&db), namespace_id, metric).await?, value);
	}

	Ok(())
}

#[tokio::test]
async fn compactor_emits_pitr_metric_keys() -> Result<()> {
	let db = support::test_db("sqlite-namespace-pitr-metrics-").await?;
	let namespace_id = Id::new_v1(4002);
	seed_namespace_config(Arc::clone(&db), namespace_id, support::namespace_config()).await?;
	support::commit_pages(Arc::clone(&db), ACTOR_ID, vec![(3, 0x33), (5, 0x55)], 128, 1)
		.await?;

	run_payload(Arc::clone(&db), namespace_id, ACTOR_ID, ACTOR_NAME).await?;

	let pitr_used = read_metric(
		Arc::clone(&db),
		namespace_id,
		Metric::SqliteStoragePitrUsed(ACTOR_NAME.to_string()),
	)
	.await?;
	let checkpoint_count = read_metric(
		Arc::clone(&db),
		namespace_id,
		Metric::SqliteCheckpointCount(ACTOR_NAME.to_string()),
	)
	.await?;

	assert!(pitr_used > 0);
	assert_eq!(checkpoint_count, 1);
	Ok(())
}

#[tokio::test]
async fn pinned_count_correct_during_fork() -> Result<()> {
	let db = support::test_db("sqlite-namespace-pinned-metrics-").await?;
	let namespace_id = Id::new_v1(4003);
	seed_namespace_config(Arc::clone(&db), namespace_id, support::namespace_config()).await?;
	support::commit_pages(Arc::clone(&db), ACTOR_ID, vec![(8, 0x88)], 128, 1).await?;
	run_payload(Arc::clone(&db), namespace_id, ACTOR_ID, ACTOR_NAME).await?;

	set_checkpoint_refcount(Arc::clone(&db), ACTOR_ID, 1).await?;
	run_payload_with_config(
		Arc::clone(&db),
		namespace_id,
		ACTOR_ID,
		ACTOR_NAME,
		CompactorConfig::default(),
	)
	.await?;
	assert_eq!(
		read_metric(
			Arc::clone(&db),
			namespace_id,
			Metric::SqliteCheckpointPinned(ACTOR_NAME.to_string()),
		)
		.await?,
		1,
	);

	set_checkpoint_refcount(Arc::clone(&db), ACTOR_ID, 0).await?;
	run_payload_with_config(
		Arc::clone(&db),
		namespace_id,
		ACTOR_ID,
		ACTOR_NAME,
		CompactorConfig::default(),
	)
	.await?;
	assert_eq!(
		read_metric(
			Arc::clone(&db),
			namespace_id,
			Metric::SqliteCheckpointPinned(ACTOR_NAME.to_string()),
		)
		.await?,
		0,
	);

	Ok(())
}

#[tokio::test]
async fn namespace_aggregate_gauge_sums_actors() -> Result<()> {
	let db = support::test_db("sqlite-namespace-gauge-sums-").await?;
	let namespace_id = Id::new_v1(4004);
	let namespace_label = namespace_id.to_string();
	write_metric(
		Arc::clone(&db),
		namespace_id,
		Metric::SqliteStoragePitrUsed("a".to_string()),
		1_000_000,
	)
	.await?;
	write_metric(
		Arc::clone(&db),
		namespace_id,
		Metric::SqliteStoragePitrUsed("b".to_string()),
		2_000_000,
	)
	.await?;
	write_metric(
		Arc::clone(&db),
		namespace_id,
		Metric::SqliteStoragePitrUsed("c".to_string()),
		3_000_000,
	)
	.await?;

	worker::test_hooks::refresh_namespace_metrics_once(Arc::clone(&db), namespace_id).await?;

	assert_eq!(
		metrics::SQLITE_STORAGE_PITR_USED_BYTES_NAMESPACE_SUM
			.with_label_values(&[namespace_label.as_str()])
			.get(),
		6_000_000,
	);
	Ok(())
}

#[test]
fn no_per_actor_prometheus_emit() {
	let _ = &*metrics::SQLITE_STORAGE_LIVE_USED_BYTES_NAMESPACE_SUM;
	let _ = &*metrics::SQLITE_STORAGE_PITR_USED_BYTES_NAMESPACE_SUM;
	let _ = &*metrics::SQLITE_CHECKPOINT_COUNT_NAMESPACE_SUM;
	let _ = &*metrics::SQLITE_CHECKPOINT_PINNED_NAMESPACE_SUM;

	let names = REGISTRY
		.gather()
		.into_iter()
		.map(|family| family.name().to_string())
		.collect::<Vec<_>>();

	assert!(!names.iter().any(|name| name == "sqlite_checkpoint_count"));
	assert!(!names
		.iter()
		.any(|name| name == "sqlite_retention_delta_kept_bytes"));
	assert!(!names
		.iter()
		.any(|name| name == "sqlite_retention_checkpoint_kept_bytes"));
}
