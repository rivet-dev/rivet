#[derive(Debug, thiserror::Error)]
pub enum ClientError {
	#[error("env var: {0}")]
	VarError(#[from] std::env::VarError),
	#[error("invalid uri: {0}")]
	InvalidEndpoint(#[from] aws_smithy_http::endpoint::error::InvalidEndpointError),
	#[error("lookup host: {0}")]
	LookupHost(std::io::Error),
	#[error("unresolved host")]
	UnresolvedHost,
}

/// How to access the S3 service.
pub enum EndpointKind {
	/// Used for making calls within the core cluster using private DNS.
	///
	/// This should be used for all API calls.
	Internal,

	/// Used for making calls within the cluster, but without access to the internal DNS server. This will
	/// resolve the IP address on the machine building the presigned request.
	///
	/// Should be used sparingly, incredibly hacky.
	InternalResolved,

	/// Used for making calls from outside of the cluster.
	///
	/// This should be used for all public presigned requests.
	External,
}

#[derive(Clone)]
pub struct Client {
	bucket: String,
	client: aws_sdk_s3::Client,
}

impl std::ops::Deref for Client {
	type Target = aws_sdk_s3::Client;

	fn deref(&self) -> &aws_sdk_s3::Client {
		&self.client
	}
}

impl Client {
	pub fn new(
		bucket: &str,
		endpoint: &str,
		region: &str,
		access_key_id: &str,
		secret_access_key: &str,
	) -> Result<Self, ClientError> {
		dbg!(&bucket);
		dbg!(&endpoint);
		dbg!(&region);
		dbg!(&access_key_id);
		dbg!(&secret_access_key);
		let config = aws_sdk_s3::Config::builder()
			.region(aws_sdk_s3::Region::new(region.to_owned()))
			.endpoint_resolver(aws_sdk_s3::Endpoint::immutable(endpoint)?)
			.credentials_provider(aws_sdk_s3::Credentials::new(
				access_key_id,
				secret_access_key,
				None,
				None,
				"Static",
			))
			// .sleep_impl(Arc::new(aws_smithy_async::rt::sleep::TokioSleep::new()))
			.build();
		let client = aws_sdk_s3::Client::from_conf(config);

		Ok(Client {
			bucket: bucket.to_owned(),
			client,
		})
	}

	pub async fn from_env(svc_name: &str) -> Result<Self, ClientError> {
		Self::from_env_opt(svc_name, EndpointKind::Internal).await
	}

	pub async fn from_env_opt(
		svc_name: &str,
		endpoint_kind: EndpointKind,
	) -> Result<Self, ClientError> {
		let bucket = namespaced_bucket_name(&svc_name);
		let region = std::env::var("S3_REGION")?;
		let access_key_id = std::env::var("S3_ACCESS_KEY_ID")?;
		let secret_access_key = std::env::var("S3_SECRET_ACCESS_KEY")?;

		let endpoint = match endpoint_kind {
			EndpointKind::Internal => std::env::var("S3_ENDPOINT_INTERNAL")?,
			EndpointKind::InternalResolved => {
				let mut endpoint = std::env::var("S3_ENDPOINT_INTERNAL")?;

				// HACK: Resolve Minio DNS address to schedule the job with. We
				// do this since the job servers don't have the internal DNS servers
				// to resolve the Minio endpoint.
				//
				// This has issues if there's a race condition with changing the
				// Minio address.
				//
				// We can't resolve the presigned URL, since the host's presigned
				// host is part of the signature.
				const MINIO_K8S_HOST: &str = "minio.minio.svc.cluster.local:9200";
				if endpoint.contains(MINIO_K8S_HOST) {
					tracing::info!(host = %MINIO_K8S_HOST, "looking up dns");

					// Resolve IP
					let mut hosts = tokio::net::lookup_host(MINIO_K8S_HOST)
						.await
						.map_err(ClientError::LookupHost)?;
					let Some(host) = hosts.next() else {
						return Err(ClientError::UnresolvedHost);
					};

					// Substitute endpoint with IP
					endpoint = endpoint.replace(MINIO_K8S_HOST, &host.to_string());
				}

				endpoint
			}
			EndpointKind::External => std::env::var("S3_ENDPOINT_EXTERNAL")?,
		};

		Self::new(
			&bucket,
			&endpoint,
			&region,
			&access_key_id,
			&secret_access_key,
		)
	}

	pub fn bucket(&self) -> &str {
		&self.bucket
	}
}

pub fn s3_region() -> Result<String, ClientError> {
	std::env::var("S3_REGION").map_err(Into::into)
}

pub fn s3_credentials() -> Result<(String, String), ClientError> {
	let access_key_id = std::env::var("S3_ACCESS_KEY_ID")?;
	let secret_access_key = std::env::var("S3_SECRET_ACCESS_KEY")?;

	Ok((access_key_id, secret_access_key))
}

pub fn s3_endpoint_external() -> Result<String, ClientError> {
	std::env::var("S3_ENDPOINT_EXTERNAL").map_err(Into::into)
}

pub fn namespaced_bucket_name(name: &str) -> String {
	format!("{}-{}", rivet_util::env::namespace(), name)
}
