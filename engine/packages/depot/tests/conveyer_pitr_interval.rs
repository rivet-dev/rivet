mod common;

use anyhow::Result;
use depot::{
	keys,
	pitr_interval::{
		read_latest_pitr_interval_coverage_at_or_before, read_pitr_interval_coverage,
		write_pitr_interval_coverage,
	},
	types::{DatabaseBranchId, PitrIntervalCoverage},
};
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

fn branch_id() -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff))
}

fn coverage(txid: u64, wall_clock_ms: i64, expires_at_ms: i64) -> PitrIntervalCoverage {
	PitrIntervalCoverage {
		txid,
		versionstamp: [txid as u8; 16],
		wall_clock_ms,
		expires_at_ms,
	}
}

async fn seed_rows(
	db: &universaldb::Database,
	branch: DatabaseBranchId,
	rows: Vec<(i64, PitrIntervalCoverage)>,
) -> Result<()> {
	db.run(move |tx| {
		let rows = rows.clone();

		async move {
			for (bucket_start_ms, coverage) in rows {
				write_pitr_interval_coverage(&tx, branch, bucket_start_ms, coverage)?;
			}

			Ok(())
		}
	})
	.await
}

#[test]
fn pitr_interval_keys_sort_by_bucket_start_ms() {
	let branch = branch_id();
	let mut keys = vec![
		keys::branch_pitr_interval_key(branch, 1_700_000_600_000),
		keys::branch_pitr_interval_key(branch, 1_700_000_000_000),
		keys::branch_pitr_interval_key(branch, 1_700_000_300_000),
	];
	keys.sort();

	assert_eq!(
		keys,
		vec![
			keys::branch_pitr_interval_key(branch, 1_700_000_000_000),
			keys::branch_pitr_interval_key(branch, 1_700_000_300_000),
			keys::branch_pitr_interval_key(branch, 1_700_000_600_000),
		]
	);
	assert!(keys[0].starts_with(&keys::branch_pitr_interval_prefix(branch)));
	assert_eq!(
		keys::decode_branch_pitr_interval_bucket(branch, &keys[1]).expect("bucket should decode"),
		1_700_000_300_000
	);
}

#[tokio::test]
async fn pitr_interval_helpers_read_exact_between_and_quiet_period_targets() -> Result<()> {
	common::test_matrix("pitr-interval-read", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch = branch_id();

			seed_rows(
				&db,
				branch,
				vec![
					(1_000, coverage(10, 1_000, 10_000)),
					(2_000, coverage(20, 2_500, 10_000)),
					(3_000, coverage(30, 3_000, 10_000)),
				],
			)
			.await?;

			let exact = db
				.run(move |tx| async move {
					read_pitr_interval_coverage(&tx, branch, 1_000, Snapshot).await
				})
				.await?
				.expect("exact bucket should exist");
			assert_eq!(exact.txid, 10);

			let between = db
				.run(move |tx| async move {
					read_latest_pitr_interval_coverage_at_or_before(&tx, branch, 2_750, Snapshot)
						.await
				})
				.await?
				.expect("between-bucket target should resolve");
			assert_eq!(between.0, 2_000);
			assert_eq!(between.1.txid, 20);

			let quiet_period = db
				.run(move |tx| async move {
					read_latest_pitr_interval_coverage_at_or_before(&tx, branch, 2_999, Snapshot)
						.await
				})
				.await?
				.expect("quiet-period target should resolve to the previous retained commit");
			assert_eq!(quiet_period.0, 2_000);
			assert_eq!(quiet_period.1.txid, 20);

			let walked_back = db
				.run(move |tx| async move {
					read_latest_pitr_interval_coverage_at_or_before(&tx, branch, 2_100, Snapshot)
						.await
				})
				.await?
				.expect("newer selected row should walk back");
			assert_eq!(walked_back.0, 1_000);
			assert_eq!(walked_back.1.txid, 10);

			Ok(())
		})
	})
	.await
}
