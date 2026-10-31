//! Service entry point for Epoxy protocol discovery. It probes every peer once at
//! startup; later requests probe a peer on first use and after a version error.

use std::sync::Arc;

use crate::transport::HttpTransport;
use anyhow::Result;

#[tracing::instrument(skip_all)]
pub async fn start(transport: Arc<HttpTransport>) -> Result<()> {
	transport.refresh_versions().await
}
