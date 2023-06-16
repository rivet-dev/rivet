use crate::{config::service::RuntimeKind, context::ServiceContext};

pub fn pool_name(svc: &ServiceContext) -> String {
	match svc.config().runtime {
		// Use the index instead of the name since the pool name needs to be as short as possible
		RuntimeKind::Redis { index, .. } => format!("red-{index}"),
		_ => unreachable!(),
	}
}

pub fn server_port(svc: &ServiceContext) -> u16 {
	match svc.config().runtime {
		RuntimeKind::Redis { index, .. } => 6300 + index * 10,
		_ => unreachable!(),
	}
}
