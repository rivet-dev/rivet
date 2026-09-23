use std::sync::Arc;

use anyhow::{Result, bail};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use pegboard::keys::envoy::{ActorCommandKey, decode_actor_commands, read_actor_commands};
use rivet_envoy_protocol as protocol;
use tempfile::Builder;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

const ENVOY_KEY: &str = "envoy";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("pegboard-actor-command-chunking-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn start_command(input_len: usize) -> protocol::ActorCommandKeyData {
	protocol::ActorCommandKeyData::CommandStartActor(protocol::CommandStartActor {
		config: protocol::ActorConfig {
			name: "chunking-actor".to_string(),
			key: None,
			create_ts: 0,
			input: Some(vec![7; input_len]),
		},
		hibernating_requests: Vec::new(),
		preloaded_kv: None,
		sqlite_fence: None,
		sqlite_startup: None,
		waiting_requests: Vec::new(),
	})
}

fn command_key(namespace_id: Id, actor_id: Id, index: i64) -> ActorCommandKey {
	ActorCommandKey::new(namespace_id, ENVOY_KEY.to_string(), actor_id, 1, index)
}

fn write_chunked(
	tx: &universaldb::Transaction,
	key: &ActorCommandKey,
	command: protocol::ActorCommandKeyData,
) -> Result<usize> {
	let chunks = key.split(command)?;
	let chunk_count = chunks.len();
	for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
		tx.set(&tx.pack(&key.chunk(chunk_idx)), &chunk);
	}

	Ok(chunk_count)
}

fn write_unchunked(
	tx: &universaldb::Transaction,
	key: &ActorCommandKey,
	command: protocol::ActorCommandKeyData,
) -> Result<()> {
	tx.set(
		&tx.pack(key),
		&protocol::versioned::ActorCommandKeyData::wrap_latest(command)
			.serialize_with_embedded_version(protocol::PROTOCOL_VERSION)?,
	);

	Ok(())
}

async fn read_commands(db: &universaldb::Database, namespace_id: Id) -> Result<Vec<(i64, usize)>> {
	db.txn(
		"test_pegboard_actor_command_chunking_read",
		move |tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			let subspace = pegboard::keys::subspace().subspace(&ActorCommandKey::subspace(
				namespace_id,
				ENVOY_KEY.to_string(),
			));
			let entries = tx
				.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::WantAll,
						..(&subspace).into()
					},
					Serializable,
				)
				.try_collect::<Vec<_>>()
				.await?;

			decode_actor_commands(&tx, entries)?
				.into_iter()
				.map(|(key, command)| match command {
					protocol::ActorCommandKeyData::CommandStartActor(start) => {
						Ok((key.index, start.config.input.map(|x| x.len()).unwrap_or(0)))
					}
					protocol::ActorCommandKeyData::CommandStopActor(_) => {
						bail!("unexpected stop command at index {}", key.index)
					}
				})
				.collect()
		},
	)
	.await
}

#[tokio::test]
async fn decodes_chunked_and_unchunked_commands_in_order() -> Result<()> {
	let db = test_db().await?;
	let namespace_id = Id::new_v1(1);
	let actor_id = Id::new_v1(1);

	let chunk_count = db
		.txn(
			"test_pegboard_actor_command_chunking_seed",
			move |tx| async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());
				write_unchunked(
					&tx,
					&command_key(namespace_id, actor_id, 1),
					start_command(16),
				)?;
				let chunk_count = write_chunked(
					&tx,
					&command_key(namespace_id, actor_id, 2),
					start_command(50_000),
				)?;
				write_chunked(
					&tx,
					&command_key(namespace_id, actor_id, 3),
					start_command(16),
				)?;
				write_unchunked(
					&tx,
					&command_key(namespace_id, actor_id, 4),
					start_command(32),
				)?;

				Ok(chunk_count)
			},
		)
		.await?;
	assert!(chunk_count > 1, "large command should span multiple chunks");

	assert_eq!(
		read_commands(&db, namespace_id).await?,
		vec![(1, 16), (2, 50_000), (3, 16), (4, 32)],
	);

	Ok(())
}

#[tokio::test]
async fn paged_read_matches_single_read_without_splitting_chunks() -> Result<()> {
	let db = test_db().await?;
	let namespace_id = Id::new_v1(1);
	let actor_id = Id::new_v1(1);

	db.txn(
		"test_pegboard_actor_command_chunking_seed",
		move |tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			for index in 1..=12 {
				let key = command_key(namespace_id, actor_id, index);
				match index % 3 {
					0 => write_unchunked(&tx, &key, start_command(64))?,
					1 => {
						write_chunked(&tx, &key, start_command(35_000))?;
					}
					_ => {
						write_chunked(&tx, &key, start_command(64))?;
					}
				}
			}

			Ok(())
		},
	)
	.await?;

	let expected = read_commands(&db, namespace_id).await?;
	assert_eq!(expected.len(), 12);

	// A one byte budget ends every page after a single command, and a 15 KB budget crosses chunk
	// boundaries inside the large commands.
	for page_bytes in [1, 15_000, usize::MAX] {
		let paged = read_actor_commands(&db, namespace_id, ENVOY_KEY, page_bytes)
			.await?
			.into_iter()
			.map(|(key, command)| match command {
				protocol::ActorCommandKeyData::CommandStartActor(start) => {
					Ok((key.index, start.config.input.map(|x| x.len()).unwrap_or(0)))
				}
				protocol::ActorCommandKeyData::CommandStopActor(_) => {
					bail!("unexpected stop command at index {}", key.index)
				}
			})
			.collect::<Result<Vec<_>>>()?;

		assert_eq!(paged, expected, "page_bytes={page_bytes}");
	}

	Ok(())
}

#[tokio::test]
async fn ack_range_clears_chunks_and_unchunked_values_through_index() -> Result<()> {
	let db = test_db().await?;
	let namespace_id = Id::new_v1(1);
	let actor_id = Id::new_v1(1);

	db.txn(
		"test_pegboard_actor_command_chunking_seed",
		move |tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			write_unchunked(
				&tx,
				&command_key(namespace_id, actor_id, 1),
				start_command(16),
			)?;
			write_chunked(
				&tx,
				&command_key(namespace_id, actor_id, 2),
				start_command(50_000),
			)?;
			write_chunked(
				&tx,
				&command_key(namespace_id, actor_id, 3),
				start_command(16),
			)?;

			Ok(())
		},
	)
	.await?;

	// Mirrors the ack range in pegboard-envoy, clearing every command through index 2.
	db.txn(
		"test_pegboard_actor_command_chunking_ack",
		move |tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			let start = tx.pack(&ActorCommandKey::subspace_with_actor(
				namespace_id,
				ENVOY_KEY.to_string(),
				actor_id,
				1,
			));
			let (_, end) = pegboard::keys::subspace()
				.subspace(&ActorCommandKey::subspace_with_index(
					namespace_id,
					ENVOY_KEY.to_string(),
					actor_id,
					1,
					2,
				))
				.range();
			tx.clear_range(&start, &end);

			Ok(())
		},
	)
	.await?;

	assert_eq!(read_commands(&db, namespace_id).await?, vec![(3, 16)]);

	Ok(())
}
