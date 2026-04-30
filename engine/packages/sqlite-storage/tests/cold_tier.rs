use anyhow::Result;
use sqlite_storage::cold_tier::{
	ColdTier, ColdTierOperation, FaultyColdTier, FilesystemColdTier,
};
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
	let tier = FaultyColdTier::new(FilesystemColdTier::new(root.path()));

	tier.fail_operation(ColdTierOperation::Put, true);
	assert!(tier.put_object("db/a/image/0001.ltx", b"image").await.is_err());

	tier.fail_operation(ColdTierOperation::Put, false);
	tier.put_object("db/a/image/0001.ltx", b"image").await?;

	tier.fail_next_operations(1);
	assert!(tier.get_object("db/a/image/0001.ltx").await.is_err());
	assert_eq!(
		Some(b"image".to_vec()),
		tier.get_object("db/a/image/0001.ltx").await?
	);

	Ok(())
}
