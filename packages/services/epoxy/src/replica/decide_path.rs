use epoxy_protocol::protocol;
use universaldb::{FdbBindingError, Transaction};

use crate::replica::utils;

pub fn decide_path(
	_tx: &Transaction,
	pre_accept_oks: Vec<protocol::Payload>,
	payload: &protocol::Payload,
) -> Result<protocol::Path, FdbBindingError> {
	tracing::info!(instance=?payload.instance, "deciding path");

	let mut new_payload = payload.clone();
	let mut path = protocol::Path::PathFast(protocol::PathFast {
		payload: payload.clone(),
	});

	for pre_accept_ok in pre_accept_oks {
		let protocol::Payload {
			proposal: _,
			seq,
			deps,
			instance: _,
		} = pre_accept_ok.clone();

		// EPaxos Steps 10
		if seq == payload.seq && deps == payload.deps {
			// EPaxos Steps 11 (returns PathFast)
			continue;
		} else {
			tracing::info!(?pre_accept_ok.deps, "received dissenting voice");

			// EPaxos Step 13
			let new_deps = utils::union_deps(new_payload.deps, pre_accept_ok.deps);
			new_payload.deps = new_deps.clone();

			// EPaxos Step 14
			if pre_accept_ok.seq > new_payload.seq {
				new_payload.seq = pre_accept_ok.seq;
			}

			// EPaxos Step 15
			path = protocol::Path::PathSlow(protocol::PathSlow {
				payload: new_payload.clone(),
			});
		}
	}

	Ok(path)
}
