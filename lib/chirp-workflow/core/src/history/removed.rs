use std::marker::PhantomData;

use super::event::EventType;
use crate::{
	activity::Activity as ActivityTrait, message::Message as MessageTrait,
	signal::Signal as SignalTrait, workflow::Workflow as WorkflowTrait,
};

pub trait Removed {
	fn event_type() -> EventType;
	fn name() -> Option<&'static str> {
		None
	}
}

pub struct Activity<T: ActivityTrait>(PhantomData<T>);

impl<T: ActivityTrait> Removed for Activity<T> {
	fn event_type() -> EventType {
		EventType::Activity
	}

	fn name() -> Option<&'static str> {
		Some(T::NAME)
	}
}

pub struct Signal<T: SignalTrait>(PhantomData<T>);

impl<T: SignalTrait> Removed for Signal<T> {
	fn event_type() -> EventType {
		EventType::SignalSend
	}

	fn name() -> Option<&'static str> {
		Some(T::NAME)
	}
}

pub struct Message<T: MessageTrait>(PhantomData<T>);

impl<T: MessageTrait> Removed for Message<T> {
	fn event_type() -> EventType {
		EventType::MessageSend
	}

	fn name() -> Option<&'static str> {
		Some(T::NAME)
	}
}

pub struct Listen;

impl Removed for Listen {
	fn event_type() -> EventType {
		EventType::Signal
	}
}

pub struct Repeat;

impl Removed for Repeat {
	fn event_type() -> EventType {
		EventType::Loop
	}
}

pub struct Sleep;

impl Removed for Sleep {
	fn event_type() -> EventType {
		EventType::Activity
	}
}

pub struct WorkflowDispatch<T: WorkflowTrait>(PhantomData<T>);

impl<T: WorkflowTrait> Removed for WorkflowDispatch<T> {
	fn event_type() -> EventType {
		EventType::SubWorkflow
	}

	fn name() -> Option<&'static str> {
		Some(T::NAME)
	}
}

pub struct WorkflowRun<T: WorkflowTrait>(PhantomData<T>);

impl<T: WorkflowTrait> Removed for WorkflowRun<T> {
	fn event_type() -> EventType {
		EventType::Branch
	}
}
