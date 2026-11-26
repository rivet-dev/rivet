//! `call(target, op, header, body, token)`: everything the host asks Rust to do.
//! Every call completes its token exactly once, possibly before `call` returns.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use rivetkit_core::{
	BindParam, ColumnValue, SqliteBatchStatement, SqliteDb, SqliteTransaction, WsMessage,
};
use tokio::sync::oneshot;

use super::messages::{
	BroadcastCall, ClientActionCall, ClientConnectCall, ClientFetchCall, ClientOptions,
	ClientWebSocketCall, ConnActionCall, ConnDisconnectCall, ConnInfo, ConnList, ConnSendCall,
	ConnSubscribeCall, Created, CronJob, CronList, CronSetCall, NameCall, RegisterOptions, Removed,
	ScheduleCall, ScheduleId, ScheduleList, ScheduledEvent, ServeOptions, SqlBatchCall,
	SqlBatchResult, SqlBeginCall, SqlCall, SqlResult, SqlValue, StreamErrorCall, StreamWriteCall,
	WsCloseCall, WsSendCall, decode, encode,
};
use super::runtime::{ActorSlot, Answer, Handle, Region, Wire};
use super::{Op, client, registry};

impl Wire {
	/// Runs a host call. Errors, including an unknown op, complete the token with
	/// an error; nothing is returned to the caller.
	pub fn call(self: &Arc<Self>, target: u64, op: u32, header: &[u8], body: &[u8], token: u64) {
		if let Err(error) = self.dispatch(target, op, header, body, token) {
			self.complete(token, Err(error));
		}
	}

	fn dispatch(
		self: &Arc<Self>,
		target: u64,
		op: u32,
		header: &[u8],
		body: &[u8],
		token: u64,
	) -> Result<()> {
		let op = Op::from_u32(op).ok_or_else(|| anyhow!("unknown op {op}"))?;
		let done = |answer: Answer| {
			self.complete(token, Ok(answer));
			Ok(())
		};
		match op {
			Op::RegistryNew => done(registry::new(self)),
			Op::ClientNew => done(client::new(
				self,
				decode::<ClientOptions>(header, "client options")?,
			)?),
			Op::Register => {
				registry::register(
					self,
					target,
					decode::<RegisterOptions>(header, "register options")?,
				)?;
				done(Answer::empty())
			}
			Op::Serve => registry::serve(
				self,
				target,
				decode::<ServeOptions>(header, "serve options")?,
				token,
			),
			Op::ServeDone => registry::done(self, target, token),
			Op::Stop => {
				registry::stop(self, target)?;
				done(Answer::empty())
			}

			Op::Broadcast => {
				let call: BroadcastCall = decode(header, "broadcast")?;
				self.actor(target)?.ctx.broadcast(&call.name, body);
				done(Answer::empty())
			}
			Op::ConnSend => {
				let call: ConnSendCall = decode(header, "conn send")?;
				let slot = self.actor(target)?;
				let conn = slot
					.ctx
					.conns()
					.find(|conn| conn.id() == call.conn_id)
					.ok_or_else(|| anyhow!("connection {} is not open", call.conn_id))?;
				conn.send(&call.name, body);
				done(Answer::empty())
			}
			Op::ConnDisconnect => {
				let call: ConnDisconnectCall = decode(header, "conn disconnect")?;
				let slot = self.actor(target)?;
				self.spawn_complete(token, async move {
					slot.ctx
						.disconnect_conn_with_reason(call.conn_id, call.reason.as_deref())
						.await?;
					Ok(Answer::empty())
				});
				Ok(())
			}
			Op::ConnList => {
				let slot = self.actor(target)?;
				let conns = slot
					.ctx
					.conns()
					.map(|conn| ConnInfo {
						id: conn.id().to_owned(),
						params: conn.params(),
					})
					.collect();
				done(Answer::header(encode(&ConnList { conns })))
			}
			Op::Sleep => {
				self.actor(target)?.ctx.sleep()?;
				done(Answer::empty())
			}
			Op::Destroy => {
				self.actor(target)?.ctx.destroy()?;
				done(Answer::empty())
			}
			Op::KeepAwake => {
				let region = self.actor(target)?.ctx.keep_awake_region();
				done(self.created(target, Handle::Region(Region::KeepAwake(Box::new(region)))))
			}
			Op::WaitUntil => {
				let (tx, rx) = oneshot::channel::<()>();
				self.actor(target)?.ctx.wait_until(async move {
					let _ = rx.await;
				});
				done(self.created(target, Handle::Region(Region::WaitUntil(tx))))
			}
			Op::ActorClient => done(client::for_actor(self, target)?),

			Op::SqlExec | Op::SqlQuery | Op::SqlRun | Op::SqlExecute => {
				let call: SqlCall = decode(header, "sql call")?;
				let db = self.sql_target(target)?;
				self.spawn_complete(token, sql(db, op, call));
				Ok(())
			}
			Op::SqlBatch => {
				let call: SqlBatchCall = decode(header, "sql batch")?;
				let SqlTarget::Db(db) = self.sql_target(target)? else {
					bail!("batches run in their own transaction; call it on the actor");
				};
				self.spawn_complete(token, async move {
					let statements = call
						.statements
						.into_iter()
						.map(|statement| SqliteBatchStatement {
							sql: statement.sql,
							params: bind(statement.params),
						})
						.collect();
					let results = db.execute_batch(statements).await?;
					let mut rows = Vec::with_capacity(results.len());
					let results = results
						.into_iter()
						.map(|result| {
							rows.push(row_values(result.rows));
							SqlResult {
								columns: result.columns,
								changes: result.changes,
								last_insert_row_id: result.last_insert_row_id,
							}
						})
						.collect();
					Ok(Answer {
						header: encode(&SqlBatchResult { results }),
						body: encode(&rows),
					})
				});
				Ok(())
			}
			Op::SqlBegin => {
				let call: SqlBeginCall = decode(header, "sql begin")?;
				let slot = self.actor(target)?;
				let wire = self.clone();
				self.spawn_complete(token, async move {
					let timeout = call
						.timeout_ms
						.map(|ms| Duration::from_millis(u64::from(ms)));
					let tx = slot
						.ctx
						.sql()
						.begin_named_transaction(call.name.as_deref(), timeout)
						.await?;
					Ok(wire.created(target, Handle::Tx(Arc::new(tx))))
				});
				Ok(())
			}
			Op::SqlCommit | Op::SqlRollback => {
				let tx = match self.remove(target) {
					Some(Handle::Tx(tx)) => tx,
					_ => bail!("unknown or finished transaction handle {target}"),
				};
				self.spawn_complete(token, async move {
					if op == Op::SqlCommit {
						tx.commit().await?;
					} else {
						tx.rollback().await?;
					}
					Ok(Answer::empty())
				});
				Ok(())
			}

			Op::Schedule => {
				let call: ScheduleCall = decode(header, "schedule")?;
				let slot = self.actor(target)?;
				let args = body.to_vec();
				self.spawn_complete(token, async move {
					let id = match (call.delay_ms, call.timestamp_ms) {
						(Some(delay), None) => {
							slot.ctx
								.after(Duration::from_millis(delay), &call.action, &args)
								.await?
						}
						(None, Some(at)) => slot.ctx.at(at, &call.action, &args).await?,
						_ => bail!("schedule needs exactly one of delayMs and timestampMs"),
					};
					Ok(Answer::header(encode(&ScheduleId { id })))
				});
				Ok(())
			}
			Op::ScheduleCancel => {
				let call: ScheduleId = decode(header, "schedule cancel")?;
				let slot = self.actor(target)?;
				self.spawn_complete(token, async move {
					let removed = slot.ctx.cancel_schedule(&call.id).await?;
					Ok(Answer::header(encode(&Removed { removed })))
				});
				Ok(())
			}
			Op::ScheduleList => {
				let slot = self.actor(target)?;
				self.spawn_complete(token, async move {
					let events = slot
						.ctx
						.list_scheduled_events()
						.await?
						.into_iter()
						.map(|event| ScheduledEvent {
							id: event.id,
							action: event.action,
							args: event.args,
							run_at: event.run_at,
						})
						.collect();
					Ok(Answer::header(encode(&ScheduleList { events })))
				});
				Ok(())
			}
			Op::CronSet => {
				let call: CronSetCall = decode(header, "cron set")?;
				let slot = self.actor(target)?;
				let args = body.to_vec();
				self.spawn_complete(token, async move {
					match (call.interval_ms, &call.expression) {
						(Some(interval), None) => {
							slot.ctx
								.cron_every(
									&call.name,
									interval,
									&call.action,
									&args,
									call.max_history,
								)
								.await?
						}
						(None, Some(expression)) => {
							slot.ctx
								.cron_set(
									&call.name,
									expression,
									call.timezone.as_deref(),
									&call.action,
									&args,
									call.max_history,
								)
								.await?
						}
						_ => bail!("cron needs exactly one of intervalMs and expression"),
					}
					Ok(Answer::empty())
				});
				Ok(())
			}
			Op::CronDelete => {
				let call: NameCall = decode(header, "cron delete")?;
				let slot = self.actor(target)?;
				self.spawn_complete(token, async move {
					let removed = slot.ctx.cron_delete(&call.name).await?;
					Ok(Answer::header(encode(&Removed { removed })))
				});
				Ok(())
			}
			Op::CronList => {
				let slot = self.actor(target)?;
				self.spawn_complete(token, async move {
					let jobs = slot
						.ctx
						.cron_list()
						.await?
						.into_iter()
						.map(|job| CronJob {
							name: job.name,
							action: job.action,
							args: job.args,
							next_run_at: job.next_run_at,
							last_run_at: job.last_run_at,
							expression: job.expression,
							timezone: job.timezone,
							interval_ms: job.interval_ms,
						})
						.collect();
					Ok(Answer::header(encode(&CronList { jobs })))
				});
				Ok(())
			}

			Op::StreamWrite => {
				let call: StreamWriteCall = decode(header, "stream write")?;
				let body_handle = self.response_body(target)?;
				if call.fin {
					// Queued below, so the end still follows every earlier write.
					self.remove(target);
				}
				body_handle.write(body.to_vec(), call.fin, self.pending(token));
				Ok(())
			}
			Op::StreamError => {
				let call: StreamErrorCall = decode(header, "stream error")?;
				let body_handle = match self.remove(target) {
					Some(Handle::ResponseBody(body)) => body,
					_ => bail!("unknown or finished stream handle {target}"),
				};
				// Behind any queued writes, like a fin.
				body_handle.error(call.message, self.pending(token));
				Ok(())
			}

			Op::WsSend => {
				let call: WsSendCall = decode(header, "ws send")?;
				let message = if call.binary {
					WsMessage::Binary(body.to_vec())
				} else {
					WsMessage::Text(
						String::from_utf8(body.to_vec())
							.map_err(|_| anyhow!("websocket text frames must be valid utf-8"))?,
					)
				};
				match self.socket(target)? {
					// Core's server sockets have no send backpressure: the frame goes to the
					// envoy at once.
					Socket::Server(ws) => {
						ws.send(message);
						done(Answer::empty())
					}
					Socket::Client(socket) => {
						let frame = match message {
							WsMessage::Binary(bytes) => tungstenite::Message::Binary(bytes.into()),
							WsMessage::Text(text) => tungstenite::Message::Text(text.into()),
						};
						// Completes when queued; the host waits (in order, one call at a time
						// per thread) while a slow peer keeps the queue full.
						self.spawn_complete(token, async move {
							socket.send(frame).await?;
							Ok(Answer::empty())
						});
						Ok(())
					}
				}
			}
			Op::WsClose => {
				let call: WsCloseCall = decode(header, "ws close")?;
				match self.socket(target)? {
					Socket::Server(ws) => {
						self.spawn_complete(token, async move {
							ws.close(call.code, call.reason).await;
							Ok(Answer::empty())
						});
						Ok(())
					}
					Socket::Client(socket) => {
						let frame = call.code.map(|code| tungstenite::protocol::CloseFrame {
							code: code.into(),
							reason: call.reason.unwrap_or_default().into(),
						});
						self.spawn_complete(token, async move {
							socket.send(tungstenite::Message::Close(frame)).await?;
							Ok(Answer::empty())
						});
						Ok(())
					}
				}
			}

			Op::ClientAction => {
				let call: ClientActionCall = decode(header, "client action")?;
				client::action(self, target, call, body, token)
			}
			Op::ClientConnect => {
				let call: ClientConnectCall = decode(header, "client connect")?;
				client::connect(self, target, call, token)
			}
			Op::ClientFetch => {
				let call: ClientFetchCall = decode(header, "client fetch")?;
				client::fetch(self, target, call, body, token)
			}
			Op::ClientWebSocket => {
				let call: ClientWebSocketCall = decode(header, "client websocket")?;
				client::web_socket(self, target, call, token)
			}
			Op::ConnAction => {
				let call: ConnActionCall = decode(header, "conn action")?;
				client::conn_action(self, target, call.action, body, token)
			}
			Op::ConnSubscribe => {
				let call: ConnSubscribeCall = decode(header, "conn subscribe")?;
				client::subscribe(self, target, call.event, token)
			}
			Op::ConnUnsubscribe => {
				let call: ConnSubscribeCall = decode(header, "conn unsubscribe")?;
				client::unsubscribe(self, target, call.event, token)
			}
		}
	}

	fn created(&self, owner: u64, handle: Handle) -> Answer {
		Answer::header(encode(&Created {
			handle: self.insert(owner, handle),
		}))
	}

	fn sql_target(&self, id: u64) -> Result<SqlTarget> {
		self.get(id, "actor or transaction", |handle| match handle {
			Handle::Actor(slot) => Some(SqlTarget::Db(Box::new(db(slot)))),
			Handle::Tx(tx) => Some(SqlTarget::Tx(tx.clone())),
			_ => None,
		})
	}

	fn response_body(&self, id: u64) -> Result<Arc<super::body::OrderedBody>> {
		self.get(id, "stream", |handle| match handle {
			Handle::ResponseBody(body) => Some(body.clone()),
			_ => None,
		})
	}

	fn socket(&self, id: u64) -> Result<Socket> {
		self.get(id, "websocket", |handle| match handle {
			Handle::WebSocket(ws) => Some(Socket::Server(ws.clone())),
			Handle::ClientSocket(socket) => Some(Socket::Client(socket.clone())),
			_ => None,
		})
	}
}

enum Socket {
	Server(rivetkit_core::websocket::WebSocket),
	Client(Arc<client::SocketSlot>),
}

enum SqlTarget {
	Db(Box<SqliteDb>),
	Tx(Arc<SqliteTransaction>),
}

fn db(slot: &ActorSlot) -> SqliteDb {
	slot.ctx.sql().clone()
}

fn bind(params: Option<Vec<SqlValue>>) -> Option<Vec<BindParam>> {
	params.map(|params| {
		params
			.into_iter()
			.map(|value| match value {
				SqlValue::Null => BindParam::Null,
				SqlValue::Integer(value) => BindParam::Integer(value),
				SqlValue::Float(value) => BindParam::Float(value),
				SqlValue::Text(value) => BindParam::Text(value),
				SqlValue::Blob(value) => BindParam::Blob(value),
			})
			.collect()
	})
}

fn row_values(rows: Vec<Vec<ColumnValue>>) -> Vec<Vec<SqlValue>> {
	rows.into_iter()
		.map(|row| {
			row.into_iter()
				.map(|value| match value {
					ColumnValue::Null => SqlValue::Null,
					ColumnValue::Integer(value) => SqlValue::Integer(value),
					ColumnValue::Float(value) => SqlValue::Float(value),
					ColumnValue::Text(value) => SqlValue::Text(value),
					ColumnValue::Blob(value) => SqlValue::Blob(value),
				})
				.collect()
		})
		.collect()
}

fn rows_answer(
	columns: Vec<String>,
	rows: Vec<Vec<ColumnValue>>,
	changes: i64,
	last: Option<i64>,
) -> Answer {
	Answer {
		header: encode(&SqlResult {
			columns,
			changes,
			last_insert_row_id: last,
		}),
		body: encode(&row_values(rows)),
	}
}

async fn sql(target: SqlTarget, op: Op, call: SqlCall) -> Result<Answer> {
	let params = bind(call.params);
	match (target, op) {
		(SqlTarget::Db(db), Op::SqlExec) => {
			let result = db.exec(call.sql).await?;
			Ok(rows_answer(result.columns, result.rows, 0, None))
		}
		(SqlTarget::Tx(tx), Op::SqlExec) => {
			let result = tx.exec(call.sql).await?;
			Ok(rows_answer(result.columns, result.rows, 0, None))
		}
		(SqlTarget::Db(db), Op::SqlQuery) => {
			let result = db.query(call.sql, params).await?;
			Ok(rows_answer(result.columns, result.rows, 0, None))
		}
		(SqlTarget::Db(db), Op::SqlRun) => {
			let result = db.run(call.sql, params).await?;
			Ok(Answer::header(encode(&SqlResult {
				columns: Vec::new(),
				changes: result.changes,
				last_insert_row_id: None,
			})))
		}
		(SqlTarget::Db(db), Op::SqlExecute) => {
			let result = db.execute(call.sql, params).await?;
			Ok(rows_answer(
				result.columns,
				result.rows,
				result.changes,
				result.last_insert_row_id,
			))
		}
		// A transaction only has `exec` and `execute`; query/run map onto execute.
		(SqlTarget::Tx(tx), Op::SqlQuery | Op::SqlRun | Op::SqlExecute) => {
			let result = tx.execute(call.sql, params).await?;
			Ok(rows_answer(
				result.columns,
				result.rows,
				result.changes,
				result.last_insert_row_id,
			))
		}
		(_, op) => bail!("{op:?} is not a sql op"),
	}
}
