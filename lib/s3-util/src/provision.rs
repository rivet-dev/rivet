use aws_sdk_s3::error::{CreateBucketError, CreateBucketErrorKind};

use crate::{registry, Client, ClientError};

#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
	#[error("client error: {0}")]
	ClientError(#[from] ClientError),
	#[error("failed to create bucket: {0}")]
	CreateBucketError(String),
	#[error("missing env var: {0}")]
	MissingEnvVar(String),
}

/// Provisions all required S3 buckets.
pub async fn provision() -> Result<(), ProvisionError> {
	// Build client
	let region = crate::s3_region()?;
	let (access_key_id, secret_access_key) = crate::s3_credentials()?;
	let endpoint = crate::s3_endpoint_external()?;
	let client = Client::new("", &endpoint, &region, &access_key_id, &secret_access_key)?;

	// Provision buckets
	for bucket in registry::BUCKETS {
		let bucket_name = crate::namespaced_bucket_name(&bucket.name);

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
