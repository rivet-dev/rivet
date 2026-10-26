//! Executes independent bounded transport requests and returns correlated outcomes to the runner.

use super::{Payloads, VolumeClientTransport};
use crate::volume_client_machine::VolumeClientMachine;
use crate::volume_client_machine::{
	effect::VolumeClientEffect as Effect,
	event::{RpcError, VolumeClientEvent as Input},
	types::*,
};
use parking_lot::Mutex;
use rivet_env::TaskScope;
use rivet_state_machine_ephemeral::EffectContext;
use std::{
	future::Future,
	sync::{Arc, Weak},
	time::Duration,
};
use vbare::Encode;

pub(crate) struct RequestExecutor {
	pub payloads: Arc<Mutex<Payloads>>,
	pub tasks: Weak<TaskScope>,
	pub transport: Arc<dyn VolumeClientTransport>,
	pub may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	pub config: VolumeClientConfig,
	pub on_revoked: Option<Input>,
}

impl RequestExecutor {
	/// Materializes append bytes while processing the effect; network completion only stores fetched pages.
	pub(crate) fn request(&self, request: Effect, ctx: EffectContext<VolumeClientMachine>) {
		let Some(tasks) = self.tasks.upgrade() else {
			return;
		};

		// Capture correlation and materialize bytes before the core can release payloads.
		let failure = unavailable(&request);
		let timeout = match request {
			Effect::VolumeOpenRequest { .. }
			| Effect::VolumeCloseRequest { .. }
			| Effect::ClaimRequest { .. } => self.config.claim_timeout_ms,
			_ => self.config.append_timeout_ms,
		};
		let is_append = matches!(request, Effect::AppendRequest { .. });
		let prepared = if is_append {
			Some(self.payloads.lock().prepare_append(&request))
		} else {
			None
		};
		// A creating open sends its initial writes' bodies, read before the core can release them.
		let initial_bodies = match &request {
			Effect::VolumeOpenRequest { initial, .. } if !initial.is_empty() => {
				Some(self.payloads.lock().initial_writes(initial))
			}
			_ => None,
		};

		// Each peer owns an independent future; no network call holds the payload map.
		let transport = self.transport.clone();
		let authority = self.may_dispatch.clone();
		let on_revoked = self.on_revoked.clone();
		let payloads = self.payloads.clone();
		let inbox = ctx.inbox().clone();
		let clock = ctx.env().clock();
		let mut task = Box::pin(ctx.track(async move {
			if !authority() {
				if let Some(event) = on_revoked.clone() {
					let _ = inbox.send(event).await;
				}
				return;
			}

			let operation = async {
				if let Some(prepared) = prepared {
					return Ok(prepared?.execute(transport.as_ref()).await);
				}
				if let Effect::ReadVolumeLogEntriesRequest {
					op_id,
					worker_id,
					volume_log_key,
					start,
					max_entries,
					max_bytes,
				} = request
				{
					let read = transport
						.read(
							worker_id.clone(),
							volume_log_key,
							start,
							max_entries,
							max_bytes,
						)
						.await;
					if !authority() {
						return Ok(failure.clone());
					}

					// Enforce the reservation before inserting remotely supplied payloads.
					let result = match read {
						Ok((state, mut entries)) => {
							let bytes = entries.iter().try_fold(0usize, |sum, entry| {
								sum.checked_add(entry.encoded_len())
									.ok_or_else(|| anyhow::anyhow!("read size overflow"))
							})?;
							if bytes > max_bytes as usize {
								tracing::warn!(
									bytes,
									max_bytes,
									"oversized volume log read response"
								);
								Err(RpcError::Unavailable)
							} else {
								// The wire carries only a byte limit; retain the core's requested prefix.
								entries.truncate(max_entries as usize);
								Ok((state, payloads.lock().retain_entries(entries)?))
							}
						}
						Err(error) => Err(error),
					};
					Ok(Input::ReadVolumeLogEntriesResponse {
						op_id,
						worker_id,
						result,
					})
				} else if let (
					Some(bodies),
					Effect::VolumeOpenRequest {
						op_id,
						volume_key,
						client_incarnation,
						create_if_absent,
						deadline_boot_ms,
						read,
						..
					},
				) = (initial_bodies, &request)
				{
					Ok(Input::VolumeOpenResponse {
						op_id: *op_id,
						result: transport
							.open(
								volume_key.clone(),
								*client_incarnation,
								*create_if_absent,
								bodies?,
								read.clone(),
								*deadline_boot_ms,
							)
							.await,
					})
				} else {
					// These requests own no payload handles; avoid holding the shared map during I/O.
					Payloads::default()
						.execute(request, transport.as_ref())
						.await
				}
			};

			// Expiry keeps the original operation ID so stale completion rules still apply.
			let event =
				match rivet_env::timeout(clock.as_ref(), Duration::from_millis(timeout), operation)
					.await
				{
					Ok(Ok(event)) => event,
					Ok(Err(error)) => {
						tracing::error!(%error, "volume transport executor failed");
						Input::Fence
					}
					Err(_) => failure,
				};

			// Recheck the same admitted authority before handing the reply to the core.
			if !authority() {
				release_page(&payloads, &event);
				if let Some(event) = on_revoked.clone() {
					let _ = inbox.send(event).await;
				}
			} else if let Err(error) = inbox.send(event).await {
				release_page(&payloads, &error.0);
			}
		}));

		// A replica's appends must reach the transport in effect order: the transport admits
		// an append to its worker's ordered lane before its future first yields. Spawned tasks
		// start in no particular order, so the first poll runs here, while this effect is
		// processed in order, and the task finishes from where that poll stopped.
		if is_append {
			let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
			if task.as_mut().poll(&mut cx).is_ready() {
				return;
			}
		}
		tasks.spawn(task);
	}
}

/// Preserve correlation on a transport deadline; the core alone chooses retry or failure.
fn unavailable(effect: &Effect) -> Input {
	match effect {
		Effect::VolumeOpenRequest { op_id, .. } => Input::VolumeOpenResponse {
			op_id: *op_id,
			result: Err(RpcError::Unavailable),
		},
		Effect::VolumeCloseRequest { op_id, .. } => Input::VolumeCloseResponse {
			op_id: *op_id,
			result: Err(RpcError::Unavailable),
		},
		Effect::ClaimRequest {
			op_id, worker_id, ..
		} => Input::ClaimResponse {
			op_id: *op_id,
			worker_id: worker_id.clone(),
			result: Err(RpcError::Unavailable),
		},
		Effect::AppendRequest {
			op_id, worker_id, ..
		} => Input::AppendResponse {
			op_id: *op_id,
			worker_id: worker_id.clone(),
			result: Err(RpcError::Unavailable),
		},
		Effect::AdvanceCommittedRequest {
			op_id, worker_id, ..
		} => Input::AdvanceCommittedResponse {
			op_id: *op_id,
			worker_id: worker_id.clone(),
			result: Err(RpcError::Unavailable),
		},
		Effect::TruncateRequest {
			op_id, worker_id, ..
		}
		| Effect::MarkInSyncRequest {
			op_id, worker_id, ..
		} => Input::MatchResponse {
			op_id: *op_id,
			worker_id: worker_id.clone(),
			result: Err(RpcError::Unavailable),
		},
		Effect::ReadVolumeLogEntriesRequest {
			op_id, worker_id, ..
		} => Input::ReadVolumeLogEntriesResponse {
			op_id: *op_id,
			worker_id: worker_id.clone(),
			result: Err(RpcError::Unavailable),
		},
		Effect::OpenResponse { .. }
		| Effect::CloseResponse { .. }
		| Effect::RecoveryResponse { .. }
		| Effect::WriteResponse { .. }
		| Effect::WaitForCommitResponse { .. }
		| Effect::ConfirmResponse { .. }
		| Effect::ReleasePayload { .. } => unreachable!("reply dispatch is separate"),
	}
}

/// Undeliverable pages still own handles until explicitly released or the executor is dropped.
fn release_page(payloads: &Mutex<Payloads>, event: &Input) {
	if let Input::ReadVolumeLogEntriesResponse {
		result: Ok((_, entries)),
		..
	} = event
	{
		let mut payloads = payloads.lock();
		for entry in entries {
			payloads.release(entry.payload);
		}
	}
}
