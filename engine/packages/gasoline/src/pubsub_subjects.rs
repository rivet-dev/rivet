use std::{borrow::Cow, fmt::Display, marker::PhantomData};

use universalpubsub::Subject;

use crate::message::Message;

pub struct MessageSubject<'a, M: Message> {
	topic: &'a str,
	msg_marker: PhantomData<M>,
}

impl<'a, M: Message> MessageSubject<'a, M> {
	pub fn new(topic: &'a str) -> Self {
		Self {
			topic,
			msg_marker: PhantomData,
		}
	}
}

impl<M: Message> Clone for MessageSubject<'_, M> {
	fn clone(&self) -> Self {
		Self {
			topic: self.topic,
			msg_marker: PhantomData,
		}
	}
}

impl<M: Message> Display for MessageSubject<'_, M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", M::subject(), self.topic)
	}
}

impl<M: Message> Subject for MessageSubject<'_, M> {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Owned(M::subject()))
	}
}
