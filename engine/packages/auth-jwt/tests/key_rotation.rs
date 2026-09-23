use rivet_auth_jwt::{
	IssuerReconcile, KeyId, RotationPolicy, SigningKeyRecord, activate_pending, bootstrap,
	decode_signing_key_ring, emergency_rotate, encode_signing_key_ring, next_wake_ts,
	prune_retiring, reconcile_issuer, recover_leader, stage_pending, stage_pending_forced,
	validate_emergency_request,
};

const SECOND: i64 = 1_000;
const DAY: i64 = 24 * 60 * 60 * SECOND;

fn policy() -> RotationPolicy {
	RotationPolicy {
		rotation_interval_ms: 7 * DAY,
		publish_lead_ms: 10 * 60 * SECOND,
		max_signing_lifetime_ms: 14 * DAY,
		max_token_ttl_ms: DAY,
		clock_skew_ms: 30 * SECOND,
	}
}

fn candidate(created_ts: i64) -> SigningKeyRecord {
	SigningKeyRecord::generate(created_ts)
}

#[test]
fn normal_rotation_preserves_publish_lead_and_verification_window() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	assert_eq!(initial.active.sign_until_ts, 14 * DAY);
	assert_eq!(
		next_wake_ts(&initial, policy()).unwrap(),
		7 * DAY - 10 * 60 * SECOND
	);

	let publication_ts = 7 * DAY - 10 * 60 * SECOND;
	let second = candidate(publication_ts);
	let staged = stage_pending(&initial, &second, publication_ts, policy())
		.unwrap()
		.successor;
	assert_eq!(staged.pending.as_ref().unwrap().activate_after_ts, 7 * DAY);
	assert!(activate_pending(&staged, 7 * DAY - 1, policy()).is_err());

	let activated = activate_pending(&staged, 7 * DAY, policy()).unwrap();
	assert_eq!(activated.successor.active.key.kid, second.signing_key.kid());
	assert_eq!(activated.successor.active.sign_until_ts, 21 * DAY);
	let retiring = &activated.successor.retiring[0];
	assert_eq!(retiring.max_token_exp_ts, 8 * DAY + 35 * SECOND);
	assert_eq!(retiring.verify_until_ts, 8 * DAY + 65 * SECOND);
}

#[test]
fn late_reconcile_waits_a_complete_publish_lead() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let late = 8 * DAY;
	let second = candidate(late);
	let staged = stage_pending(&initial, &second, late, policy())
		.unwrap()
		.successor;
	assert_eq!(
		staged.pending.unwrap().activate_after_ts,
		late + 10 * 60 * SECOND
	);
	assert_eq!(initial.active.sign_until_ts, 14 * DAY);
}

#[test]
fn forced_normal_rotation_uses_a_fresh_publish_lead() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;

	let forced_at = 2 * DAY;
	let second = candidate(forced_at);
	let staged = stage_pending_forced(&initial, &second, forced_at, policy())
		.unwrap()
		.successor;

	assert_eq!(
		staged.pending.unwrap().activate_after_ts,
		forced_at + 10 * 60 * SECOND
	);
}

#[test]
fn leader_recovery_preserves_the_replicated_active_signer() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let IssuerReconcile::Transition(prepared) = reconcile_issuer(
		&initial,
		"https://api.rivet.dev",
		2,
		2,
		&[],
		DAY - SECOND,
		policy(),
	)
	.unwrap() else {
		panic!("leader recovery configuration was not committed");
	};
	let active_kid = prepared.successor.active.key.kid;
	let recovered = recover_leader(&prepared.successor, 2, 2, 2)
		.unwrap()
		.successor;
	recovered.validate().unwrap();
	assert_eq!(recovered.leader_datacenter_id, 2);
	assert_eq!(recovered.active.key.kid, active_kid);
	assert!(recovered.pending.is_none());
}

#[test]
fn emergency_receipt_records_revoked_public_keys() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let second = candidate(DAY);
	let staged = stage_pending(&initial, &second, DAY, policy())
		.unwrap()
		.successor;
	let emergency = candidate(DAY + SECOND);
	let request_id = [9; 16];
	let transition = emergency_rotate(
		&staged,
		&emergency,
		request_id,
		staged.generation,
		&[first.signing_key.kid()],
		DAY + SECOND,
		policy(),
	)
	.unwrap();
	assert_eq!(
		transition.successor.active.key.kid,
		emergency.signing_key.kid()
	);
	assert!(transition.successor.pending.is_none());
	let receipt = transition.successor.last_emergency_receipt.unwrap();
	assert_eq!(receipt.request_id, request_id);
	assert!(receipt.revoked_kids.contains(&first.signing_key.kid()));
	assert!(receipt.revoked_kids.contains(&second.signing_key.kid()));
}

#[test]
fn emergency_request_rejects_duplicate_and_unknown_keys() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let active_kid = initial.active.key.kid;

	assert!(
		validate_emergency_request(&initial, initial.generation, &[active_kid, active_kid],)
			.is_err()
	);
	assert!(
		validate_emergency_request(
			&initial,
			initial.generation,
			&[KeyId::from_bytes([0xff; 16])],
		)
		.is_err()
	);
}

#[test]
fn emergency_request_remains_available_after_leader_recovery() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let IssuerReconcile::Transition(prepared) = reconcile_issuer(
		&initial,
		"https://api.rivet.dev",
		2,
		2,
		&[],
		DAY - SECOND,
		policy(),
	)
	.unwrap() else {
		panic!("leader recovery configuration was not committed");
	};
	let recovered = recover_leader(&prepared.successor, 2, 2, 2)
		.unwrap()
		.successor;

	validate_emergency_request(&recovered, recovered.generation, &[]).unwrap();
}

#[test]
fn pruning_waits_through_expiration_leeway() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let second = candidate(DAY);
	let staged = stage_pending(&initial, &second, DAY, policy())
		.unwrap()
		.successor;
	let activation_ts = staged.pending.as_ref().unwrap().activate_after_ts;
	let activated = activate_pending(&staged, activation_ts, policy())
		.unwrap()
		.successor;
	let deadline = activated.retiring[0].verify_until_ts;
	assert!(prune_retiring(&activated, deadline - 1).unwrap().is_none());
	let pruned = prune_retiring(&activated, deadline).unwrap().unwrap();
	assert!(pruned.successor.retiring.is_empty());
}

#[test]
fn issuer_migration_is_bounded_and_stale_configuration_cannot_reverse_it() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://old.example".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;

	// Preparing the future issuer does not mutate durable trust.
	assert!(
		reconcile_issuer(
			&initial,
			"https://old.example",
			1,
			1,
			&["https://new.example".into()],
			SECOND,
			policy(),
		)
		.unwrap() == IssuerReconcile::Stable
	);

	let IssuerReconcile::Transition(migrated) = reconcile_issuer(
		&initial,
		"https://new.example",
		1,
		2,
		&["https://old.example".into()],
		2 * SECOND,
		policy(),
	)
	.unwrap() else {
		panic!("issuer migration did not produce a transition");
	};
	let migrated = migrated.successor;
	assert_eq!(migrated.issuer_state.active.issuer, "https://new.example");
	assert_eq!(migrated.issuer_state.active.config_generation, 2);
	assert!(migrated.issuer_state.accepts("https://old.example", DAY));
	assert!(migrated.issuer_state.accepts("https://new.example", DAY));

	// An old process from the prepare phase is fenced by its lower generation.
	assert!(
		reconcile_issuer(
			&migrated,
			"https://old.example",
			1,
			1,
			&["https://new.example".into()],
			3 * SECOND,
			policy(),
		)
		.unwrap() == IssuerReconcile::StaleConfiguration
	);

	let deadline = migrated.issuer_state.retiring[0].accept_until_ts;
	assert_eq!(
		deadline,
		migrated.issuer_state.retiring[0].retired_ts + DAY + 65 * SECOND
	);
	assert!(
		reconcile_issuer(
			&migrated,
			"https://new.example",
			1,
			2,
			&[],
			deadline - 1,
			policy(),
		)
		.is_err()
	);
	let IssuerReconcile::Transition(drained) = reconcile_issuer(
		&migrated,
		"https://new.example",
		1,
		2,
		&[],
		deadline,
		policy(),
	)
	.unwrap() else {
		panic!("issuer retirement did not produce a transition");
	};
	let drained = drained.successor;
	assert!(drained.issuer_state.retiring.is_empty());
	assert!(
		!drained
			.issuer_state
			.accepts("https://old.example", deadline)
	);
	assert!(
		drained
			.issuer_state
			.history
			.iter()
			.any(|entry| entry.issuer == "https://old.example")
	);
}

#[test]
fn issuer_change_requires_a_new_configuration_generation() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://old.example".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;
	let error = reconcile_issuer(
		&initial,
		"https://new.example",
		1,
		1,
		&["https://old.example".into()],
		SECOND,
		policy(),
	)
	.unwrap_err();
	assert!(error.to_string().contains("issuer_generation"));
}

#[test]
fn configuration_generation_can_fence_a_leader_change_without_changing_issuer() {
	let first = candidate(0);
	let initial = bootstrap(
		"https://api.example".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&first,
		0,
		policy(),
	)
	.unwrap()
	.successor;

	let IssuerReconcile::Transition(advanced) =
		reconcile_issuer(&initial, "https://api.example", 2, 2, &[], SECOND, policy()).unwrap()
	else {
		panic!("generation advance did not produce a transition");
	};
	assert_eq!(advanced.successor.issuer_state.active.config_generation, 2);
	assert_eq!(advanced.successor.leader_config_generation, 1);
	assert!(advanced.successor.issuer_state.retiring.is_empty());

	assert!(recover_leader(&advanced.successor, 2, 2, 1,).is_err());
	let recovered = recover_leader(&advanced.successor, 2, 2, 2)
		.unwrap()
		.successor;
	assert_eq!(recovered.leader_config_generation, 2);
}

#[test]
fn versioned_signing_ring_round_trips_without_debugging_the_seed() {
	let key = candidate(6);
	assert!(!format!("{key:?}").contains(&hex::encode(key.signing_key.seed())));

	let ring = bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		4,
		5,
		&key,
		10,
		policy(),
	)
	.unwrap()
	.successor;
	let encoded_ring = encode_signing_key_ring(&ring).unwrap();
	assert_eq!(u16::from_le_bytes(encoded_ring[..2].try_into().unwrap()), 3);
	assert_eq!(decode_signing_key_ring(&encoded_ring).unwrap(), ring);

	let v1_ring = hex::decode(
		"010001001568747470733a2f2f6170692e72697665742e6465760972697665742d61706904000500000000000000010000000000000010030303030303030303030303030303030020ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c04000500000000000000060000000000000001000000000000000a000000000000000a08194800000000000000",
	)
	.unwrap();
	assert!(decode_signing_key_ring(&v1_ring).is_err());
}
