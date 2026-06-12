use std::{borrow::Cow, fmt::Display};

use uuid::Uuid;

pub trait Subject: Display {
	/// Used for cardinality-bounded metrics. Return only stable subject families here.
	fn root<'a>() -> Option<Cow<'a, str>> {
		None
	}

	fn subject_root<'a>(&'a self) -> Option<Cow<'a, str>> {
		Self::root()
	}

	fn as_str(&self) -> Option<&str> {
		None
	}

	fn as_cow<'a>(&'a self) -> Cow<'a, str> {
		if let Some(subject) = self.as_str() {
			Cow::Borrowed(subject)
		} else {
			Cow::Owned(self.to_string())
		}
	}
}

impl Subject for &str {
	fn as_str(&self) -> Option<&str> {
		Some(self)
	}
}

impl Subject for &String {
	fn as_str(&self) -> Option<&str> {
		Some(self)
	}
}

impl<T: Subject + ?Sized> Subject for &T {
	fn root<'a>() -> Option<Cow<'a, str>> {
		T::root()
	}

	fn subject_root<'a>(&'a self) -> Option<Cow<'a, str>> {
		T::subject_root(*self)
	}

	fn as_str(&self) -> Option<&str> {
		T::as_str(*self)
	}
}

#[derive(Clone)]
pub struct InboxSubject {
	pub(crate) node_id: Option<Uuid>,
	pub(crate) id: Uuid,
}

impl InboxSubject {
	pub fn new() -> Self {
		Self {
			node_id: None,
			id: Uuid::new_v4(),
		}
	}

	pub fn new_with_node_id(node_id: Uuid) -> Self {
		Self {
			node_id: Some(node_id),
			id: Uuid::new_v4(),
		}
	}

	pub fn from_existing(subject: &str) -> Option<Self> {
		let Some((_, subject)) = subject.split_once("_INBOX.") else {
			return None;
		};

		let Some((node_id, id)) = subject.split_once(".") else {
			if let Ok(id) = Uuid::parse_str(subject) {
				return Some(Self { node_id: None, id });
			}

			return None;
		};

		let Ok(node_id) = Uuid::parse_str(node_id) else {
			return None;
		};

		let Ok(id) = Uuid::parse_str(id) else {
			return None;
		};

		Some(Self {
			node_id: Some(node_id),
			id,
		})
	}

	pub fn prefix() -> &'static str {
		"_INBOX"
	}
}

impl Display for InboxSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.as_cow().fmt(f)
	}
}

impl Subject for InboxSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("_inbox"))
	}

	fn as_cow<'a>(&'a self) -> Cow<'a, str> {
		if let Some(node_id) = self.node_id {
			Cow::Owned(format!("{}.{node_id}.{}", InboxSubject::prefix(), self.id))
		} else {
			Cow::Owned(format!("{}.{}", InboxSubject::prefix(), self.id))
		}
	}
}

/// Cross-process source of truth for slow-consumer labels. Typed `Subject::root()` is only
/// available at local publish and subscribe sites.
pub fn subject_root_from_str(subject: &str) -> &'static str {
	if subject.starts_with("pegboard.runner.eviction-by-id.") {
		"pegboard.runner.eviction-by-id"
	} else if subject.starts_with("pegboard.runner.eviction-by-name.") {
		"pegboard.runner.eviction-by-name"
	} else if subject.starts_with("pegboard.runner.") {
		"pegboard.runner"
	} else if subject.starts_with("pegboard.gateway.") {
		"pegboard.gateway"
	} else if subject.starts_with("pegboard.envoy.eviction.") {
		"pegboard.envoy.eviction"
	} else if subject.starts_with("pegboard.envoy.") {
		"pegboard.envoy"
	} else if subject == "pegboard.serverless.outbound" {
		"pegboard.serverless.outbound"
	} else if subject == "gasoline.worker.bump" {
		"gasoline.worker.bump"
	} else if subject.starts_with("gasoline.workflow.created.") {
		"gasoline.workflow.created"
	} else if subject.starts_with("gasoline.workflow.complete.") {
		"gasoline.workflow.complete"
	} else if subject.starts_with("gasoline.signal.for-workflow.") {
		"gasoline.signal.for-workflow"
	} else if subject.starts_with("gasoline.msg.") {
		"gasoline.msg"
	} else if subject == "rivet.cache.purge" {
		"rivet.cache.purge"
	} else if subject == "rivet.debug.tracing.config" {
		"rivet.debug.tracing.config"
	} else if subject.starts_with(InboxSubject::prefix()) {
		"_inbox"
	} else {
		"unknown"
	}
}
