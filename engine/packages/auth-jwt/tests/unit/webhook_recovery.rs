use super::*;
use crate::webhook::Ring;
use std::{cell::RefCell, collections::BTreeMap};

fn recovery_page(
	namespace_ids: Vec<Id>,
	next: Option<Vec<u8>>,
) -> namespace::ops::signing_recovery_page::Output {
	namespace::ops::signing_recovery_page::Output {
		namespace_ids,
		next,
	}
}

#[tokio::test]
async fn corrupt_ring_does_not_block_healthy_ring_or_advance_into_provisioning() {
	let corrupt = Id::new_v1(1);
	let healthy = Id::new_v1(1);
	let absent = Id::new_v1(1);
	let records = BTreeMap::from([
		(corrupt, vec![255, 255]),
		(
			healthy,
			Ring::bootstrap(healthy, 1, 1, 0).unwrap().encode().unwrap(),
		),
	]);
	let original = records.clone();
	let dispatched = RefCell::new(Vec::new());
	let next = vec![1, 2, 3];
	let cursor = recover_page(
		recovery_page(vec![corrupt, healthy, absent], Some(next.clone())),
		|namespace_id| {
			let records = &records;
			async move {
				records
					.get(&namespace_id)
					.map(|bytes| Ring::decode(namespace_id, bytes).map(|_| true))
					.unwrap_or(Ok(false))
			}
		},
		|namespace_id| {
			dispatched.borrow_mut().push(namespace_id);
			async { Ok(()) }
		},
	)
	.await;
	assert_eq!(cursor, Some(next));
	assert_eq!(*dispatched.borrow(), vec![healthy]);
	assert_eq!(
		records, original,
		"recovery must never overwrite corrupt rings or provision absent ones"
	);
}

#[tokio::test]
async fn dispatch_failure_does_not_block_later_namespaces_and_is_retried_next_sweep() {
	let first = Id::new_v1(1);
	let second = Id::new_v1(1);
	let dispatched = RefCell::new(Vec::new());
	let attempts = RefCell::new(0);
	for _ in 0..2 {
		let cursor = recover_page(
			recovery_page(vec![first, second], None),
			|_| async { Ok(true) },
			|namespace_id| {
				let fail = namespace_id == first && *attempts.borrow() == 0;
				if namespace_id == first {
					*attempts.borrow_mut() += 1;
				}
				if !fail {
					dispatched.borrow_mut().push(namespace_id);
				}
				async move {
					if fail {
						anyhow::bail!("simulated dispatch unavailable");
					}
					Ok(())
				}
			},
		)
		.await;
		assert_eq!(
			cursor, None,
			"failed dispatch must not prevent completing the sweep"
		);
	}
	assert_eq!(*dispatched.borrow(), vec![second, first, second]);
	assert_eq!(*attempts.borrow(), 2);
}

#[tokio::test]
async fn idle_ring_is_selected_for_recovery_after_authority_changes() {
	let namespace_id = Id::new_v1(1);
	let persisted = Ring::bootstrap(namespace_id, 1, 1, 0)
		.unwrap()
		.encode()
		.unwrap();
	let recovered = RefCell::new(None);
	let now = crate::webhook::policy().rotation_interval_ms;
	recover_page(
		recovery_page(vec![namespace_id], None),
		|id| std::future::ready(Ring::decode(id, &persisted).map(|_| true)),
		|id| {
			// Run the same planner used by the redispatched lifecycle worker, with no API traffic.
			let mut ring = Ring::decode(id, &persisted).unwrap();
			while let Some(next) = ring.reconcile(2, 2, now).unwrap() {
				ring = next;
			}
			recovered.replace(Some(ring));
			async { Ok(()) }
		},
	)
	.await;
	let recovered = recovered.into_inner().unwrap();
	assert_eq!(recovered.keys.leader_datacenter_id, 2);
	assert_eq!(recovered.keys.leader_config_generation, 2);
	assert!(
		recovered.keys.pending.is_some(),
		"idle rotation must publish a pending key after authority recovery"
	);
	assert!(
		recovered.reconcile(1, 1, now).is_err(),
		"old authority must remain fenced"
	);
}
