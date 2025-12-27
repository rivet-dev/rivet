use std::path::PathBuf;

use anyhow::{Context, Result};
use rustls::ClientConfig;

/// Helper function to build TLS configuration with optional custom certificates
///
/// # Arguments
///
/// * `ssl_root_cert_path` - Optional path to root CA certificate for server verification
/// * `ssl_client_cert_path` - Optional path to client certificate for mutual TLS
/// * `ssl_client_key_path` - Optional path to client private key for mutual TLS
///
/// # Returns
///
/// A configured `ClientConfig` for use with tokio-postgres-rustls
pub fn build_tls_config(
	ssl_root_cert_path: Option<&PathBuf>,
	ssl_client_cert_path: Option<&PathBuf>,
	ssl_client_key_path: Option<&PathBuf>,
) -> Result<ClientConfig> {
	let mut root_store = rustls::RootCertStore::empty();

	// Add custom root certificate if provided
	if let Some(root_cert_path) = ssl_root_cert_path {
		tracing::debug!(?root_cert_path, "loading custom root certificate");
		let cert_data = std::fs::read(root_cert_path).with_context(|| {
			format!("failed to read root certificate from {:?}", root_cert_path)
		})?;

		let certs = rustls_pemfile::certs(&mut cert_data.as_slice())
			.collect::<std::result::Result<Vec<_>, _>>()
			.context("failed to parse root certificate")?;

		for cert in certs {
			root_store
				.add(cert)
				.context("failed to add root certificate to store")?;
		}
	} else {
		// Use default webpki root certificates
		root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
	}

	// Build TLS config with client certificates if provided
	let tls_config = if let (Some(client_cert_path), Some(client_key_path)) =
		(ssl_client_cert_path, ssl_client_key_path)
	{
		tracing::debug!(
			?client_cert_path,
			?client_key_path,
			"loading client certificates"
		);

		// Load client certificate
		let cert_data = std::fs::read(client_cert_path).with_context(|| {
			format!(
				"failed to read client certificate from {:?}",
				client_cert_path
			)
		})?;
		let certs = rustls_pemfile::certs(&mut cert_data.as_slice())
			.collect::<std::result::Result<Vec<_>, _>>()
			.context("failed to parse client certificate")?;

		// Load client key
		let key_data = std::fs::read(client_key_path)
			.with_context(|| format!("failed to read client key from {:?}", client_key_path))?;
		let key = rustls_pemfile::private_key(&mut key_data.as_slice())
			.context("failed to parse client key")?
			.context("no private key found in client key file")?;

		ClientConfig::builder()
			.with_root_certificates(root_store)
			.with_client_auth_cert(certs, key)
			.context("failed to configure client authentication")?
	} else {
		ClientConfig::builder()
			.with_root_certificates(root_store)
			.with_no_client_auth()
	};

	Ok(tls_config)
}
