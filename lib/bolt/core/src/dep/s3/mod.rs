use anyhow::Result;
use s3_util::aws_sdk_s3;

use crate::{config::cache, context::ProjectContext};

#[derive(Clone, serde::Deserialize)]
pub struct ServiceKey {
	pub key_id: String,
	pub key: String,
}

pub async fn fetch_service_key(ctx: &ProjectContext, path: &[&str]) -> Result<ServiceKey> {
	Ok(ServiceKey {
		key_id: ctx.read_secret(&[path, &["key_id"]].concat()).await?,
		key: ctx.read_secret(&[path, &["key"]].concat()).await?,
	})
}

pub async fn check_exists_cached(
	ctx: &ProjectContext,
	s3_client: &s3_util::Client,
	key: &str,
) -> Result<bool> {
	let entry = cache::S3FileExistsEntry {
		bucket: s3_client.bucket().into(),
		key: key.into(),
	};

	// Check if exists based on cache
	if ctx.cache(|x| x.s3_file_exists.contains(&entry)).await {
		return Ok(true);
	}

	// Get object cache
	let res = s3_client
		.head_object()
		.bucket(s3_client.bucket())
		.key(key)
		.send()
		.await;
	let object_exists = match res {
		Ok(_output) => true,
		Err(aws_sdk_s3::types::SdkError::ServiceError(err))
			if matches!(
				err.err(),
				aws_sdk_s3::error::HeadObjectError {
					kind: aws_sdk_s3::error::HeadObjectErrorKind::NotFound(_),
					..
				}
			) =>
		{
			false
		}

		Err(err) => return Err(err.into()),
	};

	// Save to cache
	if object_exists {
		ctx.cache_mut(|x| x.s3_file_exists.insert(entry)).await;
	}

	Ok(object_exists)
}
