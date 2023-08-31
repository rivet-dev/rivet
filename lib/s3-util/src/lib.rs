pub use aws_sdk_s3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("env var: {0}")]
	VarError(#[from] std::env::VarError),
	#[error("invalid uri: {0}")]
	InvalidEndpoint(#[from] aws_smithy_http::endpoint::error::InvalidEndpointError),
	#[error("lookup host: {0}")]
	LookupHost(std::io::Error),
	#[error("unresolved host")]
	UnresolvedHost,
	#[error("unknown provider: {0}")]
	UnknownProvider(String),
}

/// How to access the S3 service.
pub enum EndpointKind {
	/// Used for making calls within the cluster. Requires the Nebula network & Consul DNS.
	///
	/// This should be used for all API calls.
	Internal,

	/// Used for making calls within the cluster, but without access to Consul DNS. This will
	/// resolve the Consul DNS IP address.
	///
	/// Should be used sparingly.
	InternalResolved,

	/// Used for making calls from outside of the cluster.
	///
	/// This should be used for all public presigned requests.
	External,
}

#[derive(Debug)]
pub enum Provider {
	Minio,
	Backblaze,
	Aws,
}

impl Provider {
	pub fn default() -> Result<Self, Error> {
		Self::from_str(&std::env::var("S3_DEFAULT_PROVIDER")?)
	}

	pub fn from_str(s: &str) -> Result<Self, Error> {
		match s {
			"MINIO" => Ok(Provider::Minio),
			"BACKBLAZE" => Ok(Provider::Backblaze),
			"AWS" => Ok(Provider::Aws),
			_ => Err(Error::UnknownProvider(s.to_string())),
		}
	}
}

impl std::fmt::Display for Provider {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Provider::Minio => write!(f, "MINIO"),
			Provider::Backblaze => write!(f, "BACKBLAZE"),
			Provider::Aws => write!(f, "AWS"),
		}
	}
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
	) -> Result<Self, Error> {
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

	pub async fn from_env(svc_name: &str) -> Result<Self, Error> {
		Self::from_env_opt(svc_name, Provider::default()?, EndpointKind::Internal).await
	}

	pub async fn from_env_with_provider(svc_name: &str, provider: Provider) -> Result<Self, Error> {
		Self::from_env_opt(svc_name, provider, EndpointKind::Internal).await
	}

	pub async fn from_env_opt(
		svc_name: &str,
		provider: Provider,
		endpoint_kind: EndpointKind,
	) -> Result<Self, Error> {
		let svc_screaming = svc_name.to_uppercase().replace("-", "_");

		let bucket = std::env::var(format!("S3_{}_BUCKET_{}", provider, svc_screaming))?;
		let region = std::env::var(format!("S3_{}_REGION_{}", provider, svc_screaming))?;
		let access_key_id =
			std::env::var(format!("S3_{}_ACCESS_KEY_ID_{}", provider, svc_screaming))?;
		let secret_access_key = std::env::var(format!(
			"S3_{}_SECRET_ACCESS_KEY_{}",
			provider, svc_screaming
		))?;

		let endpoint = match endpoint_kind {
			EndpointKind::Internal => std::env::var(format!(
				"S3_{}_ENDPOINT_INTERNAL_{}",
				provider, svc_screaming
			))?,
			EndpointKind::InternalResolved => {
				let mut endpoint = std::env::var(format!(
					"S3_{}_ENDPOINT_INTERNAL_{}",
					provider, svc_screaming
				))?;

				// HACK: Resolve Minio Consul address to schedule the job with. We
				// do this since the job servers don't have Consul clients
				// running on them to resolve Consul address to.
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
						.map_err(Error::LookupHost)?;
					let Some(host) = hosts.next() else {
						return Err(Error::UnresolvedHost)
					};

					// Substitute endpoint with IP
					endpoint = endpoint.replace(MINIO_K8S_HOST, &host.to_string());
				}

				endpoint
			}
			EndpointKind::External => std::env::var(format!(
				"S3_{}_ENDPOINT_EXTERNAL_{}",
				provider, svc_screaming
			))?,
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
