use aws_sdk_s3::error::{CreateBucketError, CreateBucketErrorKind};

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
pub async fn provision(
	config: rivet_config::Config,
	buckets: &[S3Bucket],
) -> Result<(), ProvisionError> {
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
			Ok(_) => tracing::info!(bucket = ?bucket_name, "bucket created"),
			Err(err) => {
				if let aws_sdk_s3::types::SdkError::ServiceError(service_err) = &err {
					if let CreateBucketError {
						kind: CreateBucketErrorKind::BucketAlreadyOwnedByYou(_),
						..
					} = service_err.err()
					{
						tracing::info!(bucket = ?bucket_name, "bucket already exists");
						continue;
					}
				}
				return Err(ProvisionError::CreateBucketError(format!("{err:?}")));
			}
		}
	}

	tracing::info!("finished provisioning storage buckets");

	Ok(())
}
