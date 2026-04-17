use anyhow::Result;
use rivetkit_core::CoreRegistry;

#[derive(Debug, Default)]
pub struct Registry {
	inner: CoreRegistry,
}

impl Registry {
	pub fn new() -> Self {
		Self::default()
	}

	pub async fn serve(self) -> Result<()> {
		self.inner.serve().await
	}
}
