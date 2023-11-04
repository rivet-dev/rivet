use indoc::indoc;
use proto::backend;
use rivet_operation::prelude::*;

#[tracing::instrument]
pub async fn run_from_env() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("upload-provider-fill").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("upload-provider-fill");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"upload-provider-fill".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	let crdb_pool = ctx.crdb().await?;

	let Ok(backfill_provider) = std::env::var("S3_BACKFILL_PROVIDER") else {
		tracing::warn!("no backfill provider env var, will have to manually re-run to backfill");
		return Ok(());
	};

	let provider = s3_util::Provider::from_str(&backfill_provider)?;
	let proto_provider = match provider {
		s3_util::Provider::Minio => backend::upload::Provider::Minio,
		s3_util::Provider::Backblaze => backend::upload::Provider::Backblaze,
		s3_util::Provider::Aws => backend::upload::Provider::Aws,
	};

	sql_execute!(
		[ctx]
		"
		UPDATE db_upload.uploads
		SET provider = $1
		WHERE provider IS NULL 
		",
		proto_provider as i32 as i64,
	)
	.await?;

	Ok(())
}
