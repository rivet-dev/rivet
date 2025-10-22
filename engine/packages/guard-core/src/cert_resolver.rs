use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::{ServerConfig, sign::CertifiedKey};
use std::sync::Arc;

/// Type signature for a function that resolves a TLS certificate based on the server name
pub type CertResolverFn = Arc<
	dyn Fn(&str) -> Result<Arc<CertifiedKey>, Box<dyn std::error::Error + Send + Sync>>
		+ Send
		+ Sync,
>;

/// Custom certificate resolver implementation that delegates to a resolver function
pub(crate) struct CertResolver {
	resolver_fn: CertResolverFn,
}

impl std::fmt::Debug for CertResolver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CertResolver")
			.field("resolver_fn", &"<function>")
			.finish()
	}
}

impl CertResolver {
	pub fn new(resolver_fn: CertResolverFn) -> Self {
		Self { resolver_fn }
	}
}

impl ResolvesServerCert for CertResolver {
	fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
		// Extract the server name if available
		if let Some(server_name) = client_hello.server_name() {
			tracing::debug!("SNI server name requested: {}", server_name);

			// Call the resolver function with the server name directly
			let resolver_fn = &self.resolver_fn;
			match (resolver_fn)(server_name) {
				Ok(cert) => {
					tracing::debug!("Resolved certificate for {}", server_name);
					return Some(cert);
				}
				Err(e) => {
					// Log the error but don't fall back to a default certificate
					tracing::debug!("Error resolving certificate for {}: {}", server_name, e);
					return None;
				}
			}
		} else {
			tracing::debug!("No SNI server name provided");
			return None;
		}
	}
}

pub fn create_tls_config(resolver_fn: CertResolverFn) -> ServerConfig {
	ServerConfig::builder()
		.with_no_client_auth()
		.with_cert_resolver(Arc::new(CertResolver::new(resolver_fn)))
}
