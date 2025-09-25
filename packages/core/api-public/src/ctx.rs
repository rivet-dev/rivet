use std::{
	ops::Deref,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::Result;

#[derive(Clone)]
pub struct ApiCtx {
	inner: rivet_api_builder::ApiCtx,
	token: Option<String>,
	authentication_handled: Arc<AtomicBool>,
}

impl ApiCtx {
	pub fn new(inner: rivet_api_builder::ApiCtx, token: Option<String>) -> Self {
		ApiCtx {
			inner,
			token,
			authentication_handled: Arc::new(AtomicBool::new(false)),
		}
	}

	pub async fn auth(&self) -> Result<()> {
		let Some(auth) = &self.config().auth else {
			return Ok(());
		};

		self.authentication_handled.store(true, Ordering::Relaxed);

		if self.token.as_ref() == Some(&auth.admin_token) {
			Ok(())
		} else {
			Err(rivet_api_builder::ApiForbidden.build())
		}
	}

	pub fn skip_auth(&self) {
		self.authentication_handled.store(true, Ordering::Relaxed);
	}

	pub fn is_auth_handled(&self) -> bool {
		if self.config().auth.is_none() {
			return true;
		}

		self.authentication_handled.load(Ordering::Relaxed)
	}

	pub fn token(&self) -> Option<&str> {
		self.token.as_deref()
	}
}

impl Deref for ApiCtx {
	type Target = rivet_api_builder::ApiCtx;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl From<ApiCtx> for rivet_api_builder::ApiCtx {
	fn from(value: ApiCtx) -> rivet_api_builder::ApiCtx {
		value.inner
	}
}
