use napi_derive::napi;
use rivetkit_core::ActorContext as CoreActorContext;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::{
	ExecuteResult, JsBindParam, JsNativeDatabase, QueryResult,
	open_database_with_runtime_config,
};

#[napi]
pub struct SqliteDb {
	ctx: CoreActorContext,
	database: Mutex<Option<Arc<JsNativeDatabase>>>,
}

impl SqliteDb {
	pub(crate) fn new(ctx: CoreActorContext) -> Self {
		Self {
			ctx,
			database: Mutex::new(None),
		}
	}

	async fn database(&self) -> napi::Result<Arc<JsNativeDatabase>> {
		let mut guard = self.database.lock().await;
		if let Some(database) = guard.as_ref() {
			return Ok(Arc::clone(database));
		}

		let database = Arc::new(
			open_database_with_runtime_config(
				self.ctx
					.sql()
					.runtime_config()
					.map_err(crate::napi_anyhow_error)?,
				Vec::new(),
			)
			.await?,
		);
		*guard = Some(Arc::clone(&database));
		Ok(database)
	}
}

#[napi]
impl SqliteDb {
	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<QueryResult> {
		let database = self.database().await?;
		database.exec(sql).await
	}

	#[napi]
	pub async fn run(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<ExecuteResult> {
		let database = self.database().await?;
		database.run(sql, params).await
	}

	#[napi]
	pub async fn query(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<QueryResult> {
		let database = self.database().await?;
		database.query(sql, params).await
	}

	#[napi]
	pub async fn close(&self) -> napi::Result<()> {
		let database = self.database.lock().await.take();
		if let Some(database) = database {
			database.close().await?;
		}
		Ok(())
	}
}
