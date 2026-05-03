use anyhow::Result;
use depot::cold_tier::{
	ColdTier, ColdTierOperation, DisabledColdTier, FaultyColdTier, FilesystemColdTier,
};
#[cfg(feature = "test-faults")]
use depot::fault::{ColdTierFaultPoint, DepotFaultController, DepotFaultPoint};
use depot::metrics;
use tempfile::Builder;

#[tokio::test]
async fn filesystem_round_trip_list_and_delete() -> Result<()> {
	let root = Builder::new().prefix("sqlite-cold-tier").tempdir()?;
	let tier = FilesystemColdTier::new(root.path());

	tier.put_object("db/a/image/0001.ltx", b"image").await?;
	tier.put_object("db/a/delta/0002.ltx", b"delta").await?;
	tier.put_object("db/b/image/0001.ltx", b"other").await?;

	assert_eq!(
		Some(b"image".to_vec()),
		tier.get_object("db/a/image/0001.ltx").await?
	);

	let listed = tier.list_prefix("db/a").await?;
	let keys = listed
		.iter()
		.map(|object| (object.key.as_str(), object.size_bytes))
		.collect::<Vec<_>>();
	assert_eq!(
		vec![("db/a/delta/0002.ltx", 5), ("db/a/image/0001.ltx", 5)],
		keys
	);

	tier.delete_objects(&["db/a/image/0001.ltx".to_string()])
		.await?;
	assert_eq!(None, tier.get_object("db/a/image/0001.ltx").await?);
	assert_eq!(
		Some(b"delta".to_vec()),
		tier.get_object("db/a/delta/0002.ltx").await?
	);

	Ok(())
}

#[tokio::test]
async fn filesystem_rejects_keys_that_escape_root() -> Result<()> {
	let root = Builder::new().prefix("sqlite-cold-tier").tempdir()?;
	let tier = FilesystemColdTier::new(root.path());

	assert!(tier.put_object("../escape", b"nope").await.is_err());
	assert!(tier.put_object("/absolute", b"nope").await.is_err());
	assert!(tier.get_object("db/../escape").await.is_err());

	Ok(())
}

#[tokio::test]
async fn faulty_tier_injects_operation_failures() -> Result<()> {
	let root = Builder::new().prefix("sqlite-cold-tier").tempdir()?;
	let node_id = "faulty-tier-test-node";
	let tier = FaultyColdTier::new(FilesystemColdTier::new(root.path()), node_id);
	let put_failures_before = metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
		.with_label_values(&[node_id, "put"])
		.get();
	let get_failures_before = metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
		.with_label_values(&[node_id, "get"])
		.get();

	tier.fail_operation(ColdTierOperation::Put, true);
	assert!(
		tier.put_object("db/a/image/0001.ltx", b"image")
			.await
			.is_err()
	);
	assert_eq!(
		metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
			.with_label_values(&[node_id, "put"])
			.get(),
		put_failures_before + 1
	);

	tier.fail_operation(ColdTierOperation::Put, false);
	tier.put_object("db/a/image/0001.ltx", b"image").await?;

	tier.fail_next_operations(1);
	assert!(tier.get_object("db/a/image/0001.ltx").await.is_err());
	assert_eq!(
		metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
			.with_label_values(&[node_id, "get"])
			.get(),
		get_failures_before + 1
	);
	assert_eq!(
		Some(b"image".to_vec()),
		tier.get_object("db/a/image/0001.ltx").await?
	);

	Ok(())
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn faulty_tier_controller_supports_operation_faults() -> Result<()> {
	let root = Builder::new().prefix("sqlite-cold-tier").tempdir()?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject))
		.once()
		.fail("put failed")?;
	controller
		.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject))
		.once()
		.drop_artifact()?;
	controller
		.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::ListPrefix))
		.once()
		.delay(std::time::Duration::from_millis(1))?;
	controller
		.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::DeleteObjects))
		.once()
		.fail("delete failed")?;
	let tier = FaultyColdTier::new_with_fault_controller_for_test(
		FilesystemColdTier::new(root.path()),
		"faulty-tier-controller-node",
		controller.clone(),
	);

	assert!(
		tier.put_object("db/a/image/0001.ltx", b"image")
			.await
			.is_err()
	);
	tier.put_object("db/a/image/0001.ltx", b"image").await?;
	assert_eq!(None, tier.get_object("db/a/image/0001.ltx").await?);
	assert_eq!(
		Some(b"image".to_vec()),
		tier.get_object("db/a/image/0001.ltx").await?
	);
	let listed = tier.list_prefix("db/a").await?;
	assert_eq!(listed.len(), 1);
	assert!(
		tier.delete_objects(&["db/a/image/0001.ltx".to_string()])
			.await
			.is_err()
	);
	controller.assert_expected_fired()?;

	Ok(())
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn faulty_tier_put_drop_artifact_writes_before_error() -> Result<()> {
	let root = Builder::new().prefix("sqlite-cold-tier").tempdir()?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject))
		.once()
		.drop_artifact()?;
	let tier = FaultyColdTier::new_with_fault_controller_for_test(
		FilesystemColdTier::new(root.path()),
		"faulty-tier-put-drop-node",
		controller.clone(),
	);

	assert!(
		tier.put_object("db/a/image/0001.ltx", b"image")
			.await
			.is_err()
	);
	assert_eq!(
		Some(b"image".to_vec()),
		tier.get_object("db/a/image/0001.ltx").await?
	);
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test]
async fn disabled_tier_fails_explicitly() -> Result<()> {
	let tier = DisabledColdTier;

	assert!(
		tier.put_object("db/a/image/0001.ltx", b"image")
			.await
			.is_err()
	);
	assert!(tier.get_object("db/a/image/0001.ltx").await.is_err());
	assert!(tier.list_prefix("db/a").await.is_err());

	Ok(())
}
