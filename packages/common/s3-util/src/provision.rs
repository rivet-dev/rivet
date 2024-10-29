use aws_sdk_s3::operation::create_bucket::CreateBucketError;

use crate::{Client, ClientError, S3Bucket};

#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
	#[error("client error: {0}")]
	ClientError(#[from] ClientError),
	#[error("failed to create bucket: {0}")]
	CreateBucketError(String),
	#[error("missing env var: {0}")]
	MissingEnvVar(String),
	#[error("{0}")]
	Global(global_error::GlobalError),
}

/// Provisions all required S3 buckets.
#[tracing::instrument(skip_all)]
pub async fn provision(
	config: rivet_config::Config,
	buckets: &[S3Bucket],
) -> Result<(), ProvisionError> {
	tracing::info!(buckets = ?buckets.len(), "provisioning s3 buckets");

	// Build client
	let s3_config = &config.server().map_err(ProvisionError::Global)?.s3;
	let client = Client::new(
		"",
		&s3_config.endpoint_external.to_string(),
		&s3_config.region,
		&*s3_config.access_key_id.read(),
		&*s3_config.secret_access_key.read(),
	)?;

	// Provision buckets
	for bucket in buckets {
		let bucket_name = crate::namespaced_bucket_name(&config, &bucket.name)?;

		match client.create_bucket().bucket(&bucket_name).send().await {
			Ok(_) => tracing::debug!(bucket = ?bucket_name, "bucket created"),
			Err(err) => {
				if let aws_sdk_s3::error::SdkError::ServiceError(service_err) = &err {
					// Minio responds with BucketAlreadyExists
					// SeaweedFS responds with BucketAlreadyExists
					if let CreateBucketError::BucketAlreadyOwnedByYou(_)
					| CreateBucketError::BucketAlreadyExists(_) = service_err.err()
					{
						tracing::debug!(bucket = ?bucket_name, "bucket already exists");
						continue;
					}
				}
				return Err(ProvisionError::CreateBucketError(format!("{err:?}")));
			}
		}
	}

	tracing::debug!("finished provisioning storage buckets");

	Ok(())
}
