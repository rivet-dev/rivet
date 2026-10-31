//! Scoped subject budgets isolate traffic classes without retaining idle peer identities.

use super::{
	PubSub,
	class::{ClassScope, RequestClass},
	request::{RequestLimit, RequestPermit},
};
use crate::errors;
use anyhow::{Result, ensure};
use std::sync::Arc;

/// Separate incoming and outgoing windows shared by every reservation of these exact subject bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubjectCapacities {
	pub incoming: usize,
	pub outgoing: usize,
}

/// Keep this guard for the service or transport's lifetime. Accepted requests retain the same
/// budgets even if the last explicit reservation is dropped while they are still in flight.
#[must_use]
#[derive(Clone)]
pub struct SubjectReservation {
	_limits: Arc<SubjectLimits>,
}

pub(super) struct SubjectLimits {
	_registry_usage: crate::metrics::BufferUsage,
	/// The only class that may travel on this subject.
	pub(super) class: RequestClass,
	capacities: SubjectCapacities,
	incoming: RequestLimit,
	outgoing: RequestLimit,
}

impl PubSub {
	/// Reserve isolated capacity before subscribing or dispatching. Concurrent callers must agree
	/// on the class and both capacities; an active subject's limits cannot be replaced underneath
	/// its permits. Requests of any other class on this subject fail with `invalid_message`.
	pub fn reserve_request_subject(
		&self,
		subject: impl AsRef<[u8]>,
		class: RequestClass,
		capacities: SubjectCapacities,
	) -> Result<SubjectReservation> {
		let subject = subject.as_ref();
		self.validate(subject, &[])?;
		ensure!(
			capacities.incoming > 0
				&& capacities.incoming <= tokio::sync::Semaphore::MAX_PERMITS
				&& capacities.outgoing > 0
				&& capacities.outgoing <= tokio::sync::Semaphore::MAX_PERMITS,
			"invalid subject request capacities"
		);
		let mut subjects = self.inner.subject_limits.lock();
		if let Some(limits) = subjects.get(subject).and_then(|limits| limits.upgrade()) {
			ensure!(
				limits.capacities == capacities && limits.class == class,
				"conflicting class or capacities for an active request subject"
			);
			return Ok(SubjectReservation { _limits: limits });
		}

		// Dead weak entries carry no budget and are reclaimed before admitting another identity.
		subjects.retain(|_, limits| limits.strong_count() != 0);
		if subjects.len() >= self.inner.options.max_request_subjects {
			return Err(crate::metrics::overloaded("request_subjects", None));
		}
		let limits = Arc::new(SubjectLimits {
			_registry_usage: self.inner.subject_usage.acquire(1),
			class,
			capacities,
			incoming: RequestLimit::for_class(
				"subject_request_incoming",
				capacities.incoming,
				class,
				"incoming",
				true,
			),
			outgoing: RequestLimit::for_class(
				"subject_request_outgoing",
				capacities.outgoing,
				class,
				"outgoing",
				false,
			),
		});
		subjects.insert(subject.to_vec(), Arc::downgrade(&limits));
		Ok(SubjectReservation { _limits: limits })
	}

	pub(super) fn subject_queue_capacity(&self, subject: &[u8]) -> usize {
		self.subject_limits(subject)
			.map_or(self.inner.options.request_subscription_capacity, |limits| {
				limits.capacities.incoming
			})
	}

	fn subject_limits(&self, subject: &[u8]) -> Option<Arc<SubjectLimits>> {
		self.inner
			.subject_limits
			.lock()
			.get(subject)
			.and_then(|limits| limits.upgrade())
	}

	/// Takes a reserved subject's per-target window first, then the class's process-wide outgoing
	/// permit, so a full window rejects without consuming class capacity. A class that differs
	/// from the subject's reservation, or a subject-scoped class on an unreserved subject, is a
	/// caller bug and fails before admission.
	pub(super) fn outgoing_permit(
		&self,
		class: RequestClass,
		subject: &[u8],
	) -> Result<RequestPermit> {
		match self.subject_limits(subject) {
			Some(limits) => {
				if limits.class != class {
					return Err(errors::InvalidMessage.build());
				}
				let mut permit = limits.outgoing.try_acquire()?;
				let class_permit = self.inner.classes.outgoing(class).try_acquire()?;
				permit._subject = Some(limits);
				permit._class = Some(Box::new(class_permit));
				Ok(permit)
			}
			None => match class.scope() {
				ClassScope::Subject => Err(errors::InvalidMessage.build()),
				ClassScope::Process => self.inner.classes.outgoing(class).try_acquire(),
			},
		}
	}

	/// The permit a decoded request holds until reply or drop. A reserved subject takes its
	/// window after checking the class; an unreserved subject takes the class's incoming permit.
	/// A subject-scoped class on an unreserved subject has no Depot server here, so it returns
	/// `no_responders`, which Depot callers retry during a restart.
	pub(super) fn incoming_permit(
		&self,
		class: RequestClass,
		subject: &[u8],
	) -> Result<RequestPermit> {
		match self.subject_limits(subject) {
			Some(limits) => {
				if limits.class != class {
					return Err(errors::InvalidMessage.build());
				}
				let mut permit = limits.incoming.try_acquire()?;
				permit._subject = Some(limits);
				Ok(permit)
			}
			None => match self.inner.classes.incoming(class) {
				Some(limit) => limit.try_acquire(),
				None => Err(errors::NoResponders.build()),
			},
		}
	}

	/// Header reads are bounded by one shared pool until their class is known.
	pub(super) fn header_permit(&self) -> Result<RequestPermit> {
		self.inner.header.try_acquire()
	}
}
