use rstest::rstest;
use slatedb::config::SstBlockSize;
use slatedb::object_store::memory::InMemory;
use slatedb::object_store::ObjectStore;
use slatedb::Db;
use std::sync::Arc;

#[tokio::test]
async fn test_configurable_block_size() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

    // Test with custom block size of 8KiB
    let db = Db::builder("/tmp/test_custom_block_size", object_store.clone())
        .with_sst_block_size(SstBlockSize::Block8Kib)
        .build()
        .await;

    assert!(db.is_ok());

    // Write some data and verify the DB works correctly
    let db = db.expect("Failed to create DB");
    db.put(b"key1", b"value1")
        .await
        .expect("Failed to put key1");
    db.put(b"key2", b"value2")
        .await
        .expect("Failed to put key2");
    db.flush().await.expect("Failed to flush");

    let value1 = db.get(b"key1").await.expect("Failed to get key1");
    assert_eq!(&value1.expect("Failed to get key1")[..], b"value1");

    let value2 = db.get(b"key2").await.expect("Failed to get key2");
    assert_eq!(&value2.expect("Failed to get key2")[..], b"value2");
}

#[tokio::test]
async fn test_default_block_size() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

    // Test with no block size specified (should use default 4KiB)
    let db = Db::builder("/tmp/test_default_block_size", object_store.clone())
        .build()
        .await;

    assert!(db.is_ok());
}

#[tokio::test]
async fn test_edge_case_block_sizes() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

    // Test with minimum valid block size (1KiB)
    let db = Db::builder("/tmp/test_min_block_size", object_store.clone())
        .with_sst_block_size(SstBlockSize::Block1Kib)
        .build()
        .await;

    assert!(db.is_ok());

    // Test with maximum valid block size (64KiB)
    let db = Db::builder("/tmp/test_max_block_size", object_store.clone())
        .with_sst_block_size(SstBlockSize::Block64Kib)
        .build()
        .await;

    assert!(db.is_ok());
}

#[tokio::test]
async fn test_block_size_actually_used() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

    // Test with a small block size to force multiple blocks
    let db = Db::builder("/tmp/test_block_size_used", object_store.clone())
        .with_sst_block_size(SstBlockSize::Block1Kib) // 1KiB blocks
        .build()
        .await
        .expect("Failed to create DB");

    // Write enough data to force multiple blocks
    // Each key-value pair is roughly 16 + value_size bytes
    let value = vec![b'x'; 100]; // 100 byte values

    for i in 0..100 {
        let key = format!("key_{:04}", i);
        db.put(key.as_bytes(), &value)
            .await
            .expect("Failed to put key");
    }

    // Force flush to create SST
    db.flush().await.expect("Failed to flush");

    // Verify all data can be read back
    for i in 0..100 {
        let key = format!("key_{:04}", i);
        let result = db.get(key.as_bytes()).await.expect("Failed to get key");
        assert_eq!(
            result.expect("Failed to get key").as_ref(),
            value.as_slice()
        );
    }
}

#[rstest]
#[case::block_1kb(SstBlockSize::Block1Kib, 1024)]
#[case::block_2kb(SstBlockSize::Block2Kib, 2048)]
#[case::block_4kb(SstBlockSize::Block4Kib, 4096)]
#[case::block_8kb(SstBlockSize::Block8Kib, 8192)]
#[case::block_16kb(SstBlockSize::Block16Kib, 16384)]
#[case::block_32kb(SstBlockSize::Block32Kib, 32768)]
#[case::block_64kb(SstBlockSize::Block64Kib, 65536)]
#[tokio::test]
async fn test_all_block_sizes(
    #[case] block_size_enum: SstBlockSize,
    #[case] expected_bytes: usize,
) {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

    // Verify the enum returns the correct byte value
    assert_eq!(block_size_enum.as_bytes(), expected_bytes);

    // Verify we can create a DB with each size
    let db = Db::builder(
        format!("/tmp/test_block_size_{}", expected_bytes),
        object_store.clone(),
    )
    .with_sst_block_size(block_size_enum)
    .build()
    .await;

    assert!(db.is_ok());
}
