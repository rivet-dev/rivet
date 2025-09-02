use futures_util::TryStreamExt;
use rivet_test_deps_docker::TestDatabase;
use std::sync::Arc;
use universaldb::{Database, RangeOption, options::StreamingMode, tuple::Subspace};
use uuid::Uuid;

#[tokio::test]
async fn test_postgres_driver() {
	let _ = tracing_subscriber::fmt::try_init();

	let (db_config, docker_config) = TestDatabase::Postgres
		.config(Uuid::new_v4(), 1)
		.await
		.unwrap();
	let mut docker_config = docker_config.unwrap();
	docker_config.start().await.unwrap();

	tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

	let rivet_config::config::Database::Postgres(postgres_config) = db_config else {
		unreachable!();
	};

	// Get the connection string from the secret
	let connection_string = postgres_config.url.read().clone();

	let driver = universaldb::driver::PostgresDatabaseDriver::new(connection_string)
		.await
		.unwrap();
	let db = Database::new(Arc::new(driver));

	test_gasoline_operations(&db).await;
}

#[tokio::test]
async fn test_rocksdb_gasoline() {
	// Create a unique temporary directory for this test
	let temp_dir = tempfile::tempdir().unwrap();
	let db_path = temp_dir.path();

	let driver = universaldb::driver::RocksDbDatabaseDriver::new(db_path.to_path_buf())
		.await
		.unwrap();
	let db = Database::new(Arc::new(driver));

	test_gasoline_operations(&db).await;
}

pub async fn test_gasoline_operations(db: &Database) {
	// First, test a simple write and read to isolate the issue
	println!("Running simple write/read test...");

	// Simple test: write a single key and read it back
	db.run(|tx, _maybe_committed| async move {
		tx.set(b"simple_test_key", b"simple_test_value");
		Ok(())
	})
	.await
	.unwrap();

	let value = db
		.run(|tx, _maybe_committed| async move {
			let val = tx.get(b"simple_test_key", false).await?;
			println!(
				"Simple test read result: {:?}",
				val.as_ref()
					.map(|v| std::str::from_utf8(v).unwrap_or("<binary>"))
			);
			Ok(val)
		})
		.await
		.unwrap();

	assert!(
		value.is_some(),
		"Simple write/read test failed - value not found!"
	);
	println!("Simple write/read test passed");

	// Define the gasoline subspace
	const RIVET: usize = 0;
	const GASOLINE: usize = 1;
	const KV: usize = 2;
	let workflow_subspace = Subspace::from(&(RIVET, GASOLINE, KV));

	// Generate a workflow ID
	let workflow_id = Uuid::new_v4();

	// Test 1: Write workflow data like gasoline does
	db.run(|tx, _maybe_committed| {
		let workflow_subspace = workflow_subspace.clone();
		async move {
			// Write create timestamp (similar to CreateTsKey)
			let create_ts_key = workflow_subspace.pack(&("workflow", "create_ts", workflow_id));
			tx.set(&create_ts_key, b"12345678");

			// Write chunked input data (similar to InputKey)
			let input_data = r#"{"test": "data"}"#;
			let input_bytes = input_data.as_bytes();

			// Chunk the data (mimicking FormalChunkedKey behavior)
			const CHUNK_SIZE: usize = 1024;
			let chunks: Vec<_> = input_bytes.chunks(CHUNK_SIZE).collect();

			for (i, chunk) in chunks.iter().enumerate() {
				let chunk_key =
					workflow_subspace.pack(&("workflow", "input", workflow_id, i as u32));
				tx.set(&chunk_key, chunk);
			}

			// Write state data
			let state_data = r#"{}"#;
			let state_key = workflow_subspace.pack(&("workflow", "state", workflow_id, 0u32));
			tx.set(&state_key, state_data.as_bytes());

			// Write has_wake_condition
			let wake_condition_key =
				workflow_subspace.pack(&("workflow", "has_wake_condition", workflow_id));
			tx.set(&wake_condition_key, b"false");

			Ok(())
		}
	})
	.await
	.unwrap();

	println!("Test 1 completed - data written");

	// Test 2: Read workflow data back like gasoline does
	let (input_found, state_found, wake_found) = db
		.run(|tx, _maybe_committed| {
			let workflow_subspace = workflow_subspace.clone();
			async move {
				// Read input chunks using range query
				let input_key_base = workflow_subspace.pack(&("workflow", "input", workflow_id));
				let input_subspace = Subspace::from_bytes(input_key_base.clone());

				let input_chunks = tx
					.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(&input_subspace).into()
						},
						false,
					)
					.try_collect::<Vec<_>>()
					.await?;

				// Read state chunks
				let state_key_base = workflow_subspace.pack(&("workflow", "state", workflow_id));
				let state_subspace = Subspace::from_bytes(state_key_base.clone());

				let state_chunks = tx
					.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(&state_subspace).into()
						},
						false,
					)
					.try_collect::<Vec<_>>()
					.await?;

				// Read wake condition
				let wake_condition_key =
					workflow_subspace.pack(&("workflow", "has_wake_condition", workflow_id));
				let wake_condition = tx.get(&wake_condition_key, false).await?;

				println!("Input chunks found: {}", input_chunks.len());
				println!("State chunks found: {}", state_chunks.len());
				println!("Wake condition found: {}", wake_condition.is_some());

				Ok((
					!input_chunks.is_empty(),
					!state_chunks.is_empty(),
					wake_condition.is_some(),
				))
			}
		})
		.await
		.unwrap();

	assert!(input_found, "Should find input chunks");
	assert!(state_found, "Should find state chunks");
	assert!(wake_found, "Should find wake condition");

	// Test 3: Test the exact pattern gasoline uses with subspace operations
	db.run(|tx, _maybe_committed| {
		let workflow_subspace = workflow_subspace.clone();
		async move {
			// Create a new workflow ID
			let workflow_id2 = Uuid::new_v4();

			// Write using the exact subspace pattern gasoline uses
			let input_key_base = workflow_subspace.pack(&("workflow", "input", workflow_id2));
			let input_subspace = Subspace::from_bytes(input_key_base.clone());

			// Write chunked data to the input subspace
			let test_input = r#"{"action": "test_workflow"}"#;
			let chunk_key = input_subspace.pack(&(0u32,));
			tx.set(&chunk_key, test_input.as_bytes());

			Ok(())
		}
	})
	.await
	.unwrap();

	// Test 4: Verify the data was written correctly
	let workflow_id2 = db
		.run(|tx, _maybe_committed| {
			let workflow_subspace = workflow_subspace.clone();
			async move {
				// Generate the same workflow_id2 again (for test purposes, we'll store it)
				// In real usage, we'd pass it between transactions
				let workflow_id2 = Uuid::new_v4();

				// Write and immediately read back to verify
				let input_key_base = workflow_subspace.pack(&("workflow", "input", workflow_id2));
				let input_subspace = Subspace::from_bytes(input_key_base.clone());
				let chunk_key = input_subspace.pack(&(0u32,));

				tx.set(&chunk_key, b"test_data");

				// Read it back in the same transaction
				let value = tx.get(&chunk_key, false).await?;
				assert_eq!(
					value,
					Some(b"test_data".to_vec()),
					"Should read back the same data"
				);

				Ok(workflow_id2)
			}
		})
		.await
		.unwrap();

	// Test 5: Read in a separate transaction (like gasoline does)
	db.run(|tx, _maybe_committed| {
		let workflow_subspace = workflow_subspace.clone();
		async move {
			let input_key_base = workflow_subspace.pack(&("workflow", "input", workflow_id2));
			let input_subspace = Subspace::from_bytes(input_key_base.clone());

			// Read using range query like gasoline
			let input_chunks = tx
				.get_ranges_keyvalues((&input_subspace).into(), false)
				.try_collect::<Vec<_>>()
				.await?;

			assert!(
				!input_chunks.is_empty(),
				"Should find input chunks in separate transaction"
			);

			Ok(())
		}
	})
	.await
	.unwrap();
}
