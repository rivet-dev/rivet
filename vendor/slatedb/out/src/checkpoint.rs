use crate::config::{CheckpointOptions, CheckpointScope};
use crate::db::Db;
use crate::memtable_flusher::FlushTarget;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[non_exhaustive]
#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct Checkpoint {
    pub id: Uuid,
    pub manifest_id: u64,
    pub expire_time: Option<DateTime<Utc>>,
    pub create_time: DateTime<Utc>,
    pub name: Option<String>,
}

impl Checkpoint {
    pub(crate) fn validate_retained_identity(&self) -> Result<(), crate::error::SlateDBError> {
        if self.expire_time.is_some() || self.name.is_some() {
            return Err(crate::error::SlateDBError::InvalidConfiguration(
                "checkpoint identity already exists with incompatible retention or name".into(),
            ));
        }
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CheckpointCreateResult {
    /// The id of the created checkpoint.
    pub id: Uuid,
    /// The manifest id referenced by the created checkpoint.
    pub manifest_id: u64,
}

impl Db {
    /// Creates a checkpoint of an opened db using the provided options. Returns the ID of the created
    /// checkpoint and the id of the referenced manifest.
    pub async fn create_checkpoint(
        &self,
        scope: CheckpointScope,
        options: &CheckpointOptions,
    ) -> Result<CheckpointCreateResult, crate::Error> {
        options.validate()?;
        let target = match scope {
            CheckpointScope::All => {
                self.inner.request_batch_writer_flush(true).await?;
                FlushTarget::All
            }
            CheckpointScope::Durable => FlushTarget::CurrentDurable,
        };

        self.inner
            .memtable_flusher()
            .create_checkpoint(target, options.clone())
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::admin::AdminBuilder;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::checkpoint::Checkpoint;
    use crate::checkpoint::CheckpointCreateResult;
    use crate::config::{CheckpointOptions, CheckpointScope, Settings};
    use crate::db::Db;
    use crate::db_state::{SsTableId, SsTableView};
    use crate::format::sst::SsTableFormat;
    use crate::iter::RowEntryIterator;
    use crate::manifest::store::ManifestStore;
    use crate::manifest::Manifest;
    use crate::proptest_util::{rng, sample};
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils;
    use bytes::Bytes;
    use chrono::TimeDelta;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::clock::SystemClock;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_should_create_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        // open and close the db to init the manifest and trigger another write
        let db = Db::open(path.clone(), object_store.clone()).await.unwrap();
        db.close().await.unwrap();
        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let before_checkpoint = manifest_store.read_latest_manifest().await.unwrap();

        let CheckpointCreateResult {
            id: checkpoint_id,
            manifest_id: checkpoint_manifest_id,
        } = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(manifest.id, checkpoint_manifest_id);
        let checkpoints = &manifest.manifest.core.checkpoints;
        assert_eq!(
            before_checkpoint.manifest.core.checkpoints.len() + 1,
            checkpoints.len()
        );
        let checkpoint = checkpoints.iter().find(|c| c.id == checkpoint_id).unwrap();
        assert_eq!(checkpoint.manifest_id, manifest.id);
        assert_eq!(checkpoint.expire_time, None);
    }

    #[tokio::test]
    async fn test_should_create_checkpoint_with_expiry() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        // open and close the db to init the manifest and trigger another write
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        db.close().await.unwrap();
        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let checkpoint_time = DefaultSystemClock::default().now();

        let CheckpointCreateResult {
            id: checkpoint_id,
            manifest_id: _,
        } = admin
            .create_detached_checkpoint(&CheckpointOptions {
                lifetime: Some(Duration::from_secs(3600)),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let checkpoints = &manifest.manifest.core.checkpoints;
        let checkpoint = checkpoints.iter().find(|c| c.id == checkpoint_id).unwrap();
        assert!(checkpoint.expire_time.is_some());
        let expire_time = checkpoint.expire_time.unwrap();
        let expected = checkpoint_time + Duration::from_secs(3600);
        // check that expire time is close to the expected value (account for delay/time adjustment)
        if expire_time >= expected {
            assert!(expire_time.signed_duration_since(expected) < TimeDelta::seconds(5))
        } else {
            assert!(expected.signed_duration_since(expire_time) < TimeDelta::seconds(5))
        }
    }

    #[tokio::test]
    async fn test_should_create_checkpoint_from_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        db.close().await.unwrap();
        let CheckpointCreateResult {
            id: source_checkpoint_id,
            manifest_id: source_checkpoint_manifest_id,
        } = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .unwrap();

        let CheckpointCreateResult {
            id: _,
            manifest_id: checkpoint_manifest_id,
        } = admin
            .create_detached_checkpoint(&CheckpointOptions {
                source: Some(source_checkpoint_id),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        assert_eq!(checkpoint_manifest_id, source_checkpoint_manifest_id);
    }

    #[tokio::test]
    async fn test_should_fail_create_checkpoint_from_missing_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let admin = AdminBuilder::new(path, object_store.clone()).build();
        // open and close the db to init the manifest and trigger another write
        let _ = Db::builder(path, object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();

        let source_checkpoint_id = uuid::Uuid::new_v4();
        let result = admin
            .create_detached_checkpoint(&CheckpointOptions {
                source: Some(source_checkpoint_id),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap_err();

        assert_eq!(
            result.to_string(),
            format!(
                "Data error: checkpoint missing. checkpoint_id=`{}`",
                source_checkpoint_id
            )
        );
    }

    #[tokio::test]
    async fn test_should_fail_create_checkpoint_no_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let admin = AdminBuilder::new(path, object_store.clone()).build();
        let result = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .unwrap_err();

        assert_eq!(
            result.to_string(),
            "Data error: failed to find latest transactional object (e.g. manifest) version"
        );
    }

    #[tokio::test]
    async fn test_should_refresh_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let _ = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        let CheckpointCreateResult { id, manifest_id: _ } = admin
            .create_detached_checkpoint(&CheckpointOptions {
                lifetime: Some(Duration::from_secs(100)),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();
        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let checkpoint = manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .find(|c| c.id == id)
            .unwrap();
        let expire_time = checkpoint.expire_time.unwrap();

        admin
            .refresh_checkpoint(id, Some(Duration::from_secs(1000)))
            .await
            .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let found: Vec<&Checkpoint> = manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .filter(|c| c.id == id)
            .collect();
        assert_eq!(1, found.len());
        let refreshed_expire_time = found.first().unwrap().expire_time.unwrap();
        assert!(refreshed_expire_time > expire_time);
    }

    #[tokio::test]
    async fn test_should_fail_refresh_checkpoint_if_checkpoint_missing() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let _ = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();

        let result = admin
            .refresh_checkpoint(uuid::Uuid::new_v4(), Some(Duration::from_secs(1000)))
            .await
            .unwrap_err();

        assert_eq!(result.to_string(), "Data error: invalid DB state error");
    }

    #[tokio::test]
    async fn test_should_delete_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let _ = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings::default())
            .build()
            .await
            .unwrap();
        let CheckpointCreateResult { id, manifest_id: _ } = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .unwrap();

        admin.delete_checkpoint(id).await.unwrap();

        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert!(!manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .any(|c| c.id == id));
    }

    #[tokio::test]
    async fn test_checkpoint_scope_with_force_flush() {
        let db_options = Settings {
            flush_interval: Some(Duration::from_millis(5000)),
            ..Settings::default()
        };
        test_checkpoint_scope_all(db_options, |manifest| {
            manifest.core.tree.l0.front().unwrap().clone()
        })
        .await;
    }

    #[tokio::test]
    async fn test_checkpoint_scope_all_flushes_current_memtable_into_latest_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_checkpoint_scope_all_flushes_current_memtable");
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(Settings {
                flush_interval: Some(Duration::from_millis(5000)),
                ..Settings::default()
            })
            .build()
            .await
            .unwrap();

        db.put(b"k1", b"v1").await.unwrap();
        db.put(b"k2", b"v2").await.unwrap();

        let checkpoint = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();

        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let latest_manifest = manifest_store.read_latest_manifest().await.unwrap();

        assert_eq!(latest_manifest.id, checkpoint.manifest_id);
        assert_eq!(latest_manifest.manifest.core.tree.l0.len(), 1);
        assert!(
            latest_manifest.manifest.core.last_l0_seq >= 2,
            "expected checkpoint flush to advance last_l0_seq, got {}",
            latest_manifest.manifest.core.last_l0_seq
        );

        assert_flushed_entry(
            Arc::clone(&object_store),
            path,
            &latest_manifest
                .manifest
                .core
                .tree
                .l0
                .front()
                .unwrap()
                .sst
                .id,
            (&Bytes::from_static(b"k2"), &Bytes::from_static(b"v2")),
        )
        .await;

        db.close().await.unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_checkpoint_scope_with_force_flush_wal_disabled() {
        let db_options = Settings {
            flush_interval: Some(Duration::from_millis(5000)),
            wal_enabled: false,
            ..Settings::default()
        };
        test_checkpoint_scope_all(db_options, |manifest| {
            manifest.core.tree.l0.front().unwrap().clone()
        })
        .await;
    }

    async fn test_checkpoint_scope_all<F: FnOnce(Manifest) -> SsTableView>(
        db_options: Settings,
        last_flushed_table: F,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(db_options)
            .build()
            .await
            .unwrap();

        let mut rng = rng::new_test_rng(None);
        let table = sample::table(&mut rng, 1000, 10);
        test_utils::seed_database(&db, &table, false).await.unwrap();

        let checkpoint = db
            .create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
            .await
            .unwrap();

        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let manifest = manifest_store
            .read_manifest(checkpoint.manifest_id)
            .await
            .unwrap();

        let last_written_kv = table.last_key_value().unwrap();
        let last_flushed_table_id = last_flushed_table(manifest);
        assert_flushed_entry(
            Arc::clone(&object_store),
            path,
            &last_flushed_table_id.sst.id,
            last_written_kv,
        )
        .await;
    }

    async fn assert_flushed_entry(
        object_store: Arc<dyn ObjectStore>,
        path: Path,
        table_id: &SsTableId,
        kv: (&Bytes, &Bytes),
    ) {
        let table_store = Arc::new(TableStore::new(
            Arc::clone(&object_store),
            SsTableFormat::default(),
            path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let sst_handle = SsTableView::identity(
            table_store
                .open_sst(table_id, Some(Bytes::new()))
                .await
                .unwrap(),
        );

        let mut sst_iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            kv.0,
            Arc::clone(&table_store),
            SstIteratorOptions::default(),
            None,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        let sst_entry = sst_iter.next().await.unwrap().unwrap();
        let val = match sst_entry.value {
            crate::types::ValueDeletable::Value(v) => v,
            _ => panic!("Expected a Value"),
        };
        assert_eq!(*kv.1, val)
    }

    #[tokio::test]
    async fn test_should_create_checkpoint_with_name() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let db = Db::open(path.clone(), object_store.clone()).await.unwrap();
        db.close().await.unwrap();
        let manifest_store = ManifestStore::new(&path, object_store.clone());

        let checkpoint_name = "my_checkpoint".to_string();
        let CheckpointCreateResult {
            id: checkpoint_id,
            manifest_id: _,
        } = admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: Some(checkpoint_name.clone()),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let checkpoint = manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .find(|c| c.id == checkpoint_id)
            .unwrap();
        assert_eq!(checkpoint.name, Some(checkpoint_name));
    }

    #[tokio::test]
    async fn test_should_allow_multiple_checkpoints_with_no_name() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let db = Db::open(path.clone(), object_store.clone()).await.unwrap();
        db.close().await.unwrap();
        let manifest_store = ManifestStore::new(&path, object_store.clone());

        // Create multiple checkpoints without names
        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: None,
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: None,
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let unnamed_checkpoints: Vec<_> = manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .filter(|c| c.name.is_none())
            .collect();
        assert!(unnamed_checkpoints.len() >= 2);
    }

    #[tokio::test]
    async fn test_should_list_checkpoints_filtered_by_name() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();
        let db = Db::open(path.clone(), object_store.clone()).await.unwrap();
        db.close().await.unwrap();

        // Create checkpoints with different names
        let name1 = "checkpoint_1".to_string();
        let name2 = "checkpoint_2".to_string();
        let name3 = "".to_string();

        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: Some(name1.clone()),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: Some(name2.clone()),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: None,
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        admin
            .create_detached_checkpoint(&CheckpointOptions {
                name: Some(name3.clone()),
                ..CheckpointOptions::default()
            })
            .await
            .unwrap();

        // List all checkpoints
        let all_checkpoints = admin.list_checkpoints(None).await.unwrap();
        assert!(all_checkpoints.len() >= 4);

        // List checkpoints filtered by empty name
        let filtered_checkpoints = admin.list_checkpoints(Some("")).await.unwrap();
        assert_eq!(filtered_checkpoints.len(), 2);
        assert!(filtered_checkpoints
            .iter()
            .all(|cp| cp.name.is_none() || cp.name.as_deref() == Some("")));

        // List checkpoints filtered by name1
        let filtered_checkpoints = admin.list_checkpoints(Some(&name1)).await.unwrap();
        assert_eq!(filtered_checkpoints.len(), 1);
        assert_eq!(filtered_checkpoints[0].name, Some(name1.clone()));

        // List checkpoints filtered by name2
        let filtered_checkpoints = admin.list_checkpoints(Some(&name2)).await.unwrap();
        assert_eq!(filtered_checkpoints.len(), 1);
        assert_eq!(filtered_checkpoints[0].name, Some(name2.clone()));

        // List checkpoints filtered by non-existent name
        let filtered_checkpoints = admin.list_checkpoints(Some("non_existent")).await.unwrap();
        assert_eq!(filtered_checkpoints.len(), 0);
    }
}
