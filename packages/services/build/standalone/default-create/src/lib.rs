use indoc::indoc;
use proto::backend;
use rivet_operation::prelude::*;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use uuid::Uuid;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("build-default-create");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"build-default-create".into(),
		std::time::Duration::from_secs(60),
		config,
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
	);

	let rivet_config = &ctx.config().server()?.rivet;
	if rivet_config.test_builds.is_empty() {
		return Ok(());
	}

	// Build client
	let s3_client = s3_util::Client::with_bucket_and_endpoint(
		ctx.config(),
		"bucket-infra-artifacts",
		s3_util::EndpointKind::Internal,
	)
	.await?;

	for (kind, build) in &rivet_config.test_builds {
		// Check if this default build is already set
		let old_default_build = sql_fetch_optional!(
			[ctx, (String,)]
			"SELECT image_tag FROM db_build.default_builds WHERE kind = $1",
			kind,
		)
		.await?;
		if old_default_build
			.as_ref()
			.map_or(false, |(old_image_tag,)| old_image_tag == &build.tag)
		{
			tracing::info!(
				?old_default_build,
				"build already matches the given tag, skipping"
			);
			continue;
		}

		// Upload the build
		tracing::info!(tag=%build.tag, "uploading new build");
		let upload_id = upload_build(&ctx, &s3_client, build).await?;

		// Update default build
		tracing::info!(tag=%build.tag, ?upload_id, "setting default build");
		sql_execute!(
			[ctx]
			"
			UPSERT INTO db_build.default_builds (kind, image_tag, upload_id)
			VALUES ($1, $2, $3)
			",
			kind,
			&build.tag,
			upload_id,
		)
		.await?;
	}

	Ok(())
}

async fn upload_build(
	ctx: &OperationContext<()>,
	s3_client: &s3_util::Client,
	build: &rivet_config::config::rivet::TestBuild,
) -> GlobalResult<Uuid> {
	// Read object from infra artifacts bucket
	let obj = s3_client
		.get_object()
		.bucket(s3_client.bucket())
		.key(build.key.display().to_string())
		.send()
		.await?;
	let len = unwrap!(obj.body.size_hint().1, "should know size");
	let mut stream = obj.body.into_async_read();

	// Prepare upload to build bucket
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-build".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: unwrap!(unwrap!(build.key.file_name(), "should have file name").to_str()).to_string(),
				content_length: len,
				multipart: false,
				..Default::default()
			},
		],
	})
	.await?;
	let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();

	// In order by byte offset
	let mut presigned_requests = upload_prepare_res.presigned_requests.clone();
	presigned_requests.sort_by_key(|req| req.byte_offset);

	// Stream from infra artifacts bucket to build bucket
	for req in &presigned_requests {
		let part = read_bytes(&mut stream, req.content_length as usize).await?;

		let url = &req.url;
		tracing::info!(%url, part=%req.part_number, "uploading file");
		let res = reqwest::Client::new()
			.put(url)
			.header(reqwest::header::CONTENT_LENGTH, part.len() as u64)
			.body(reqwest::Body::from(part))
			.send()
			.await?;

		if !res.status().is_success() {
			let status = res.status();
			let body = res.text().await?;
			bail!("failure uploading ({status}): {body:?}");
		}
	}

	tracing::info!("successfully uploaded");

	// Complete the upload
	op!([ctx] upload_complete {
		upload_id: Some(upload_id.into()),
		bucket: Some("bucket-build".into()),
	})
	.await?;

	Ok(upload_id)
}

async fn read_bytes<R: AsyncBufRead + Unpin>(
	reader: &mut R,
	num_bytes: usize,
) -> tokio::io::Result<Vec<u8>> {
	let mut buffer = Vec::with_capacity(num_bytes);

	loop {
		let available = reader.fill_buf().await?;
		let to_read = available.len().min(num_bytes - buffer.len());

		buffer.extend_from_slice(&available[..to_read]);
		reader.consume(to_read);

		if buffer.len() >= num_bytes || to_read == 0 {
			break;
		}
	}

	Ok(buffer)
}
