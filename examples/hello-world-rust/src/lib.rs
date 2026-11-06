use std::sync::Arc;

use async_trait::async_trait;
use rivetkit::prelude::*;
use rivetkit::{Action, BindParam, ColumnValue, Event, Handler, action};
use serde::{Deserialize, Serialize};

pub const ACTOR_NAME: &str = "counter";

pub struct Counter;

#[derive(Debug, Serialize, Deserialize)]
pub struct Increment {
	pub amount: i64,
}

impl Action for Increment {
	type Output = i64;

	const NAME: &'static str = "increment";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCount;

impl Action for GetCount {
	type Output = i64;

	const NAME: &'static str = "getCount";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewCount {
	pub count: i64,
}

impl Event for NewCount {
	const NAME: &'static str = "newCount";
}

#[async_trait]
impl Actor for Counter {
	type Input = ();
	type Actions = (Increment, GetCount);
	type Events = (NewCount,);
	type ConnParams = ();
	type Action = action::Raw;

	async fn create(ctx: &Ctx<Self>) -> Result<Self> {
		ctx.sql()
			.exec(
				"CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY,
					count INTEGER NOT NULL
				);
				INSERT OR IGNORE INTO counter (id, count) VALUES (0, 0);",
			)
			.await?;
		Ok(Self)
	}
}

impl Handler<Increment> for Counter {
	async fn handle(self: Arc<Self>, ctx: Ctx<Self>, action: Increment) -> Result<i64> {
		let result = ctx
			.sql()
			.execute(
				"UPDATE counter SET count = count + ? WHERE id = 0 RETURNING count",
				Some(vec![BindParam::Integer(action.amount)]),
			)
			.await?;
		let count = match result.rows.first().and_then(|row| row.first()) {
			Some(ColumnValue::Integer(count)) => *count,
			other => return Err(anyhow!("expected integer count, got {other:?}")),
		};
		ctx.emit(NewCount { count })?;
		Ok(count)
	}
}

impl Handler<GetCount> for Counter {
	async fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetCount) -> Result<i64> {
		let result = ctx
			.sql()
			.query("SELECT count FROM counter WHERE id = 0", None)
			.await?;
		match result.rows.first().and_then(|row| row.first()) {
			Some(ColumnValue::Integer(count)) => Ok(*count),
			other => Err(anyhow!("expected integer count, got {other:?}")),
		}
	}
}

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_actor::<Counter>(ACTOR_NAME);
	registry
}
