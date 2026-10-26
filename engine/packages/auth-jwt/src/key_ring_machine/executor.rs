//! Single-dispatch object-store and secure-key effects, gated by fresh coordinator authority.
use super::{KeyRingStateMachine, effect::Effect, event::Event, snapshot::Snapshot, types::*};
use object_store::{ObjectStore, PutMode, UpdateVersion};
use parking_lot::Mutex;
use rivet_env::{Env, TaskHandle, TaskScope};
use rivet_keys::keys::KeyAddress;
use rivet_object_store_util::conditional_write::WriteResult;
use rivet_singleton_coordinator::authority::Authority;
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use std::sync::Arc;
use tokio::sync::watch;

pub(crate) struct RingExecutor {
	pub store: Arc<dyn ObjectStore>,
	pub authority: Authority,
	pub snapshot: watch::Receiver<Snapshot>,
	pub tasks: TaskScope,
	pub active: Mutex<Option<(OpId, TaskHandle)>>,
	pub replies: Arc<crate::rotation::Replies>,
}
impl RingExecutor {
	pub async fn shutdown(&self) {
		self.tasks.shutdown().await;
	}
}
fn permitted(
	authority: &Authority,
	snapshot: &watch::Receiver<Snapshot>,
	env: &dyn Env,
	op: OpId,
) -> bool {
	authority.permits()
		&& env.clock().boot_ms().ok().is_some_and(|now| {
			snapshot
				.borrow()
				.pending
				.is_some_and(|call| call.op == op && now < call.deadline_boot_ms)
		})
}
impl Executor<KeyRingStateMachine> for RingExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<KeyRingStateMachine>) {
		if let Effect::EmergencyRotateResponse { call, response } = effect {
			self.replies.complete(
				call,
				if self.authority.permits() {
					Some(response)
				} else {
					None
				},
			);
			return;
		}
		let op = match effect {
			Effect::CancelOperation { op } => {
				let mut active = self.active.lock();
				if active.as_ref().is_some_and(|(id, _)| *id == op)
					&& let Some((_, task)) = active.take()
				{
					task.abort();
				}
				return;
			}
			Effect::EmergencyRotateResponse { .. } => unreachable!("delivered synchronously"),
			Effect::ReadRingRequest { op }
			| Effect::GenerateKeyRequest { op }
			| Effect::WriteRingRequest { op, .. } => op,
		};
		let env = ctx.env().clone();
		if !permitted(&self.authority, &self.snapshot, env.as_ref(), op) {
			return;
		}
		let snapshot = self.snapshot.clone();
		let authority = self.authority.clone();
		let store = self.store.clone();
		let inbox = ctx.inbox().clone();
		let task = ctx.spawn_in(&self.tasks, async move {
			if !permitted(&authority, &snapshot, env.as_ref(), op) {
				return;
			}
			let response = match effect {
				Effect::ReadRingRequest { .. } => Event::ReadRingResponse {
					op,
					result: crate::store::read(store.as_ref()).await,
				},
				Effect::GenerateKeyRequest { .. } => Event::GenerateKeyResponse {
					op,
					result: material().map_err(|error| error.to_string()),
				},
				Effect::WriteRingRequest { request, .. } => {
					let mode = match request.expected_etag {
						Some(etag) => PutMode::Update(UpdateVersion {
							e_tag: Some(etag),
							version: None,
						}),
						None => PutMode::Create,
					};
					let result = match store
						.put_opts(
							&crate::store::SigningKeyRingKey::new().pack(),
							request.bytes.into(),
							mode.into(),
						)
						.await
					{
						Ok(result) => match result.e_tag.filter(|tag| !tag.is_empty()) {
							Some(etag) => WriteResult::Confirmed { etag },
							None => WriteResult::Unknown,
						},
						Err(_) => WriteResult::Unknown,
					};
					Event::WriteRingResponse { op, result }
				}
				Effect::EmergencyRotateResponse { .. } => unreachable!("delivered synchronously"),
				Effect::CancelOperation { .. } => unreachable!("handled synchronously"),
			};
			if permitted(&authority, &snapshot, env.as_ref(), op) {
				let _ = inbox.send(response).await;
			}
		});
		if let Some((_, previous)) = self.active.lock().replace((op, task)) {
			previous.abort();
		}
	}
}
/// Production credential generation is an explicit boundary, never operational simulation RNG.
fn material() -> anyhow::Result<Material> {
	let mut material = Material {
		kid: [0; 16],
		seed: [0; 32],
	};
	rivet_env::backends::production::secure_random(&mut material.kid)?;
	rivet_env::backends::production::secure_random(&mut material.seed)?;
	Ok(material)
}
