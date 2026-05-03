mod common;

use anyhow::Result;
use depot::{
	error::SqliteStorageError,
	keys,
	policy::{
		clear_database_pitr_policy_override, clear_database_shard_cache_policy_override,
		get_bucket_pitr_policy, get_bucket_shard_cache_policy, get_database_pitr_policy_override,
		get_database_shard_cache_policy_override, get_effective_pitr_policy,
		get_effective_shard_cache_policy, set_bucket_pitr_policy, set_bucket_shard_cache_policy,
		set_database_pitr_policy_override, set_database_shard_cache_policy_override,
	},
	types::{
		DEFAULT_PITR_INTERVAL_MS, DEFAULT_PITR_RETENTION_MS, DEFAULT_SHARD_CACHE_RETENTION_MS,
		PitrPolicy, SQLITE_STORAGE_META_VERSION, ShardCachePolicy, decode_pitr_policy,
		decode_shard_cache_policy, encode_pitr_policy, encode_shard_cache_policy,
	},
};
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

fn bucket_id() -> depot::types::BucketId {
	depot::types::BucketId::from_uuid(Uuid::from_u128(0x1020_3040_5060_7080_90a0_b0c0_d0e0_f000))
}

fn assert_embedded_version(encoded: &[u8]) {
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_STORAGE_META_VERSION
	);
}

fn has_policy_error(
	err: &anyhow::Error,
	policy: &'static str,
	field: &'static str,
	value: i64,
) -> bool {
	err.chain().any(|cause| {
		matches!(
			cause.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::InvalidPolicyValue {
				policy: actual_policy,
				field: actual_field,
				value: actual_value,
			}) if *actual_policy == policy && *actual_field == field && *actual_value == value
		)
	})
}

#[test]
fn policy_payloads_round_trip_with_embedded_version() {
	let pitr = PitrPolicy {
		interval_ms: 60_000,
		retention_ms: 86_400_000,
	};
	let shard_cache = ShardCachePolicy {
		retention_ms: 3_600_000,
	};

	let encoded_pitr = encode_pitr_policy(pitr).expect("pitr policy should encode");
	assert_embedded_version(&encoded_pitr);
	assert_eq!(
		decode_pitr_policy(&encoded_pitr).expect("pitr policy should decode"),
		pitr
	);

	let encoded_shard_cache =
		encode_shard_cache_policy(shard_cache).expect("shard cache policy should encode");
	assert_embedded_version(&encoded_shard_cache);
	assert_eq!(
		decode_shard_cache_policy(&encoded_shard_cache).expect("shard cache policy should decode"),
		shard_cache
	);
}

#[tokio::test]
async fn policy_lookup_falls_back_from_database_to_bucket_to_defaults() -> Result<()> {
	common::test_matrix("conveyer-policy-fallback", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let bucket_id = bucket_id();
			let database_id = ctx.database_id.clone();
			let bucket_pitr = PitrPolicy {
				interval_ms: 120_000,
				retention_ms: 2_400_000,
			};
			let database_pitr = PitrPolicy {
				interval_ms: 300_000,
				retention_ms: 9_600_000,
			};
			let bucket_shard_cache = ShardCachePolicy {
				retention_ms: 4_800_000,
			};
			let database_shard_cache = ShardCachePolicy {
				retention_ms: 19_200_000,
			};

			assert_eq!(
				get_bucket_pitr_policy(&db, bucket_id).await?,
				PitrPolicy {
					interval_ms: DEFAULT_PITR_INTERVAL_MS,
					retention_ms: DEFAULT_PITR_RETENTION_MS,
				}
			);
			assert_eq!(
				get_effective_pitr_policy(&db, bucket_id, &database_id).await?,
				PitrPolicy::default()
			);
			assert_eq!(
				get_effective_shard_cache_policy(&db, bucket_id, &database_id).await?,
				ShardCachePolicy {
					retention_ms: DEFAULT_SHARD_CACHE_RETENTION_MS,
				}
			);
			assert_eq!(
				get_bucket_shard_cache_policy(&db, bucket_id).await?,
				ShardCachePolicy::default()
			);

			set_bucket_pitr_policy(&db, bucket_id, bucket_pitr).await?;
			set_bucket_shard_cache_policy(&db, bucket_id, bucket_shard_cache).await?;
			assert_eq!(
				get_effective_pitr_policy(&db, bucket_id, &database_id).await?,
				bucket_pitr
			);
			assert_eq!(
				get_effective_shard_cache_policy(&db, bucket_id, &database_id).await?,
				bucket_shard_cache
			);

			set_database_pitr_policy_override(&db, bucket_id, &database_id, database_pitr).await?;
			set_database_shard_cache_policy_override(
				&db,
				bucket_id,
				&database_id,
				database_shard_cache,
			)
			.await?;
			assert_eq!(
				get_database_pitr_policy_override(&db, bucket_id, &database_id).await?,
				Some(database_pitr)
			);
			assert_eq!(
				get_database_shard_cache_policy_override(&db, bucket_id, &database_id).await?,
				Some(database_shard_cache)
			);
			assert_eq!(
				get_effective_pitr_policy(&db, bucket_id, &database_id).await?,
				database_pitr
			);
			assert_eq!(
				get_effective_shard_cache_policy(&db, bucket_id, &database_id).await?,
				database_shard_cache
			);

			clear_database_pitr_policy_override(&db, bucket_id, &database_id).await?;
			clear_database_shard_cache_policy_override(&db, bucket_id, &database_id).await?;
			assert_eq!(
				get_database_pitr_policy_override(&db, bucket_id, &database_id).await?,
				None
			);
			assert_eq!(
				get_database_shard_cache_policy_override(&db, bucket_id, &database_id).await?,
				None
			);
			assert_eq!(
				get_effective_pitr_policy(&db, bucket_id, &database_id).await?,
				bucket_pitr
			);
			assert_eq!(
				get_effective_shard_cache_policy(&db, bucket_id, &database_id).await?,
				bucket_shard_cache
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn policy_writes_use_expected_storage_keys() -> Result<()> {
	common::test_matrix("conveyer-policy-storage", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let bucket_id = bucket_id();
			let database_id = ctx.database_id.clone();
			let pitr = PitrPolicy {
				interval_ms: 60_000,
				retention_ms: 600_000,
			};
			let shard_cache = ShardCachePolicy {
				retention_ms: 1_200_000,
			};

			set_bucket_pitr_policy(&db, bucket_id, pitr).await?;
			set_database_shard_cache_policy_override(&db, bucket_id, &database_id, shard_cache)
				.await?;

			let bucket_policy = db
				.run(move |tx| async move {
					Ok(tx
						.informal()
						.get(&keys::bucket_policy_pitr_key(bucket_id), Snapshot)
						.await?)
				})
				.await?
				.expect("bucket pitr policy should be stored");
			assert_eq!(decode_pitr_policy(&bucket_policy)?, pitr);

			let database_policy = db
				.run(move |tx| {
					let database_id = database_id.clone();
					async move {
						Ok(tx
							.informal()
							.get(
								&keys::database_shard_cache_policy_key(bucket_id, &database_id),
								Snapshot,
							)
							.await?)
					}
				})
				.await?
				.expect("database shard cache policy should be stored");
			assert_eq!(decode_shard_cache_policy(&database_policy)?, shard_cache);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn invalid_policy_values_return_typed_errors() -> Result<()> {
	common::test_matrix("conveyer-policy-invalid", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let bucket_id = bucket_id();
			let database_id = ctx.database_id.clone();

			let err = set_bucket_pitr_policy(
				&db,
				bucket_id,
				PitrPolicy {
					interval_ms: 0,
					retention_ms: 1,
				},
			)
			.await
			.expect_err("zero pitr interval should fail");
			assert!(has_policy_error(&err, "pitr", "interval_ms", 0));

			let err = set_database_pitr_policy_override(
				&db,
				bucket_id,
				&database_id,
				PitrPolicy {
					interval_ms: 1,
					retention_ms: -1,
				},
			)
			.await
			.expect_err("negative pitr retention should fail");
			assert!(has_policy_error(&err, "pitr", "retention_ms", -1));

			let err =
				set_bucket_shard_cache_policy(&db, bucket_id, ShardCachePolicy { retention_ms: 0 })
					.await
					.expect_err("zero shard cache retention should fail");
			assert!(has_policy_error(&err, "shard_cache", "retention_ms", 0));

			Ok(())
		})
	})
	.await
}
