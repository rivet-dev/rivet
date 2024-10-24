use std::fmt::Debug;

pub trait Message: prost::Message + Default + Clone + 'static {
	const NAME: &'static str;
	const PARAMETERS: &'static [MessageSubjectParameter];
	const TOPIC: Option<MessageTopic>;
	const TAIL_TTL: Option<i64>;
	const HISTORY: bool;

	// HACK: We need static strings for perf marks, so we generate them here
	const PERF_LABEL_SUBSCRIBE: &'static str;
	const PERF_LABEL_TAIL: &'static str;
	const PERF_LABEL_TAIL_READ: &'static str;
	const PERF_LABEL_TAIL_ANCHOR: &'static str;
	const PERF_LABEL_TAIL_ALL: &'static str;
	const PERF_LABEL_WRITE_STREAM: &'static str;
	const PERF_LABEL_WRITE_TAIL: &'static str;
	const PERF_LABEL_PUBLISH: &'static str;
}

#[derive(Debug)]
pub struct MessageSubjectParameter {
	pub wildcard: bool,
}

#[derive(Debug)]
pub struct MessageTopic {}
