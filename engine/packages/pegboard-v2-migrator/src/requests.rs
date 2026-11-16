//! Operator requests to migrate one V2 actor, sent over worker-addressed UPS to the worker that
//! holds the singleton tenure. The migrator state machine decides; this module only carries the
//! request in and the reply out.
//!
//! A lost reply is an unknown outcome, and sending the request again is safe: an actor already in
//! flight is answered as admitted once its entry is stored.
use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, ensure};
use rivet_data::versioned::{MigrateFromV2ActorRequest, MigrateFromV2ActorResponse};
use rivet_singleton_coordinator::{
	authority::Authority, singleton_coordinator_machine::types::CoordinatorTenure,
};
use rivet_topology::Topology;
use rivet_util::Id;
use tokio::sync::{mpsc, oneshot};
use universalpubsub::{PubSub, RequestClass, SubjectCapacities};
use vbare::Versioned;

use crate::{
	keys::ups::V2MigratorSubject,
	migrator_machine::{
		event::Event,
		types::{MigrateActorResult, RequestId, wire},
	},
};

/// How long the holder waits for the state machine's answer. It stays below UPS's 30 s request
/// timeout so the operator gets a reply instead of a transport timeout.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(25);

/// Requests are answered one at a time, so one slot each way suffices.
fn capacities() -> SubjectCapacities {
	SubjectCapacities {
		incoming: 1,
		outgoing: 1,
	}
}

/// The reply channel of the request being answered. The state machine owns the request itself.
#[derive(Default)]
pub struct Replies(parking_lot::Mutex<ReplyState>);

#[derive(Default)]
struct ReplyState {
	next: u64,
	pending: Option<(RequestId, oneshot::Sender<MigrateActorResult>)>,
}

/// Clears the pending reply when its caller stops waiting.
struct PendingReply<'a> {
	replies: &'a Replies,
	request: RequestId,
}

impl Drop for PendingReply<'_> {
	fn drop(&mut self) {
		let mut state = self.replies.0.lock();
		if state
			.pending
			.as_ref()
			.is_some_and(|(pending, _)| *pending == self.request)
		{
			state.pending = None;
		}
	}
}

impl Replies {
	/// Delivers the state machine's answer. An answer to a request nobody waits for any more is
	/// dropped.
	pub fn complete(&self, request: RequestId, result: MigrateActorResult) {
		let mut state = self.0.lock();
		if let Some((pending, _)) = &state.pending
			&& *pending == request
			&& let Some((_, sender)) = state.pending.take()
		{
			let _ = sender.send(result);
		}
	}

	/// Drops the pending reply, so its caller learns the tenure ended.
	pub fn close(&self) {
		self.0.lock().pending = None;
	}

	/// Queues one request and waits for its answer. It holds the runner's inbox only while
	/// queueing, so a caller waiting for an answer never keeps a stopping runner alive.
	async fn request(
		&self,
		inbox: &mpsc::WeakSender<Event>,
		v2_actor_id: Id,
	) -> Result<MigrateActorResult> {
		let (sender, receiver) = oneshot::channel();
		let request = {
			let mut state = self.0.lock();
			ensure!(
				state.pending.is_none(),
				"a migrate request is already pending"
			);
			state.next = state
				.next
				.checked_add(1)
				.context("migrate request sequence exhausted")?;
			let request = RequestId(state.next);
			state.pending = Some((request, sender));
			request
		};
		let _pending = PendingReply {
			replies: self,
			request,
		};
		let inbox = inbox.upgrade().context("the migrator stopped")?;
		inbox
			.send(Event::MigrateActorRequest {
				request,
				v2_actor_id,
			})
			.await
			.context("the migrator stopped")?;
		drop(inbox);
		receiver
			.await
			.context("the migrator tenure ended before answering")
	}
}

/// Sends one request to the holder of `tenure` and returns its answer. It is sent once; the
/// operator decides whether to send it again.
pub async fn request(
	pubsub: &PubSub,
	topology: &Topology,
	tenure: &CoordinatorTenure,
	v2_actor_id: Id,
	timeout: Duration,
) -> Result<wire::MigrateFromV2ActorResponse> {
	ensure!(topology.may_admit()?, "the operator process is not serving");
	let worker = tenure.owner.worker_id.clone();
	let subject = V2MigratorSubject::new(worker.clone());
	let _reservation =
		pubsub.reserve_request_subject(&subject, RequestClass::DepotCoordinator, capacities())?;
	let request = wire::MigrateFromV2ActorRequest {
		v2_actor_id: v2_actor_id.as_bytes().into(),
	};
	let bytes = MigrateFromV2ActorRequest::serialize_with_embedded_version(
		&request,
		MigrateFromV2ActorRequest::LATEST,
	)?;
	let reply = pubsub
		.request_with_timeout_guarded(
			RequestClass::DepotCoordinator,
			&worker,
			subject,
			&bytes,
			timeout,
			&|| topology.may_admit().unwrap_or(false),
		)
		.await
		.context(
			"the migrator did not answer; check `migrate-from-v2 status`, then send the request again",
		)?;
	MigrateFromV2ActorResponse::deserialize_with_embedded_version(&reply)
}

/// Answers operator requests for one tenure, one at a time. The subscription lives only while the
/// tenure runs, so a request that reaches a former holder finds no subscriber. It holds only a weak
/// sender: the tenure joins the migrator runner, which stops once its last strong sender is
/// dropped, while this transport may still be waiting for the next request.
pub(crate) async fn serve(
	pubsub: PubSub,
	authority: Authority,
	inbox: mpsc::WeakSender<Event>,
	replies: Arc<Replies>,
) -> Result<()> {
	ensure!(
		authority.tenure().owner.worker_id == *pubsub.worker_id()
			&& authority.tenure().owner.instance_id.as_bytes() == pubsub.instance_id().as_bytes(),
		"the migrator singleton and UPS must share worker identity"
	);
	let subject = V2MigratorSubject::new(pubsub.worker_id().clone());
	let _reservation =
		pubsub.reserve_request_subject(&subject, RequestClass::DepotCoordinator, capacities())?;
	let mut requests = pubsub.subscribe_request(subject)?;
	while let Some(message) = requests.next().await? {
		if !authority.permits() || message.is_request_expired()? {
			continue;
		}
		let request =
			match MigrateFromV2ActorRequest::deserialize_with_embedded_version(&message.payload) {
				Ok(request) => request,
				Err(_) => {
					let _ = message
						.fail(&universalpubsub::errors::InvalidMessage.build())
						.await;
					continue;
				}
			};
		let result = match Id::from_slice(&request.v2_actor_id) {
			Ok(v2_actor_id) => {
				let answer = rivet_env::timeout(
					pubsub.env().clock().as_ref(),
					ANSWER_TIMEOUT,
					replies.request(&inbox, v2_actor_id),
				)
				.await;
				match answer {
					Ok(Ok(result)) => result,
					Ok(Err(error)) => MigrateActorResult::Failed(format!("{error:#}")),
					Err(_) => MigrateActorResult::Failed(
						"the migrator did not answer in time; the actor may still be admitted, so check `migrate-from-v2 status`".into(),
					),
				}
			}
			Err(error) => MigrateActorResult::Failed(format!("invalid V2 actor id: {error:#}")),
		};
		let bytes = MigrateFromV2ActorResponse::serialize_with_embedded_version(
			&to_wire(result),
			MigrateFromV2ActorResponse::LATEST,
		)?;
		if authority.permits() && !message.is_request_expired()? {
			let _ = message.reply(&bytes);
		}
	}
	Ok(())
}

fn to_wire(result: MigrateActorResult) -> wire::MigrateFromV2ActorResponse {
	match result {
		MigrateActorResult::Admitted {
			v3_actor_id,
			migration_id,
		} => wire::MigrateFromV2ActorResponse::MigrateFromV2ActorAdmitted(
			wire::MigrateFromV2ActorAdmitted {
				v3_actor_id: v3_actor_id.as_bytes().into(),
				migration_id,
			},
		),
		MigrateActorResult::Ineligible(reason) => {
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorIneligible(
				wire::MigrateFromV2ActorIneligible {
					reason: reason.as_str().to_owned().into(),
				},
			)
		}
		MigrateActorResult::NotFound => {
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorNotFound
		}
		MigrateActorResult::Unavailable(unavailable) => {
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorUnavailable(
				wire::MigrateFromV2ActorUnavailable {
					reason: unavailable.as_str().to_owned().into(),
				},
			)
		}
		MigrateActorResult::Failed(message) => {
			wire::MigrateFromV2ActorResponse::MigrateFromV2ActorFailed(
				wire::MigrateFromV2ActorFailed {
					message: message.into(),
				},
			)
		}
	}
}
