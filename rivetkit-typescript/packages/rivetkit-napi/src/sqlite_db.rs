use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::ActorContext as CoreActorContext;

use crate::napi_error;

#[napi]
pub struct SqliteDb {
	ctx: CoreActorContext,
}

impl SqliteDb {
	pub(crate) fn new(ctx: CoreActorContext) -> Self {
		Self { ctx }
	}
}

#[napi]
impl SqliteDb {
	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<Buffer> {
		self.ctx
			.db_exec(&sql)
			.await
			.map(Buffer::from)
			.map_err(napi_error)
	}

	#[napi]
	pub async fn query(&self, sql: String, params: Option<Buffer>) -> napi::Result<Buffer> {
		self.ctx
			.db_query(&sql, params.as_deref())
			.await
			.map(Buffer::from)
			.map_err(napi_error)
	}
}
