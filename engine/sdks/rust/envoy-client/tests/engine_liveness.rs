//! The engine ping silence baseline must never be later than the engine's liveness timestamp,
//! must not count a ping from an earlier connection against actors started on a claimed
//! connection, must stay bounded before the first ping of a connection, and must not move forward
//! on a reconnect the engine never claims.

use std::time::{Duration, Instant};

use rivet_envoy_client::context::EngineLiveness;

fn at(base: Instant, ms: u64) -> Instant {
	base + Duration::from_millis(ms)
}

#[test]
fn no_connection_has_no_baseline() {
	assert_eq!(EngineLiveness::default().baseline(), None);
}

#[test]
fn fresh_connection_counts_from_install_before_the_first_ping() {
	let t = Instant::now();
	let liveness = EngineLiveness::default();
	liveness.connection_installed(at(t, 100));
	assert_eq!(liveness.baseline(), Some(at(t, 100)));
}

#[test]
fn reconnect_claimed_by_commands_ignores_the_previous_connection_ping() {
	// The reported sequence: ping, disconnect, reconnect, missed start commands before the
	// first ping on the new connection.
	let t = Instant::now();
	let liveness = EngineLiveness::default();
	liveness.connection_installed(at(t, 0));
	liveness.ping_received(at(t, 1_000));
	liveness.connection_installed(at(t, 8_000));
	assert!(liveness.commands_received(), "commands prove the claim");
	assert_eq!(liveness.baseline(), Some(at(t, 8_000)));
}

#[test]
fn unclaimed_reconnect_keeps_the_previous_ping() {
	// The engine refreshes its timestamp only when it claims the connection. Until then, and
	// for good if the claim fails, it is still counting from the previous ping.
	let t = Instant::now();
	let liveness = EngineLiveness::default();
	liveness.connection_installed(at(t, 0));
	liveness.ping_received(at(t, 1_000));
	liveness.connection_installed(at(t, 8_000));
	assert_eq!(liveness.baseline(), Some(at(t, 1_000)));
}

#[test]
fn first_ping_on_a_connection_claims_it() {
	let t = Instant::now();
	let liveness = EngineLiveness::default();
	liveness.connection_installed(at(t, 0));
	assert!(liveness.ping_received(at(t, 500)));
	assert!(
		!liveness.ping_received(at(t, 3_500)),
		"only the first ping claims"
	);
	assert_eq!(liveness.baseline(), Some(at(t, 3_500)));
}

#[test]
fn unclaimed_reconnect_keeps_a_claim_proven_only_by_commands() {
	// The engine refreshed its timestamp when it claimed the first connection, which only
	// commands proved. A reconnect it never claims must keep counting from that claim.
	let t = Instant::now();
	let liveness = EngineLiveness::default();
	liveness.connection_installed(at(t, 0));
	assert!(liveness.commands_received());
	liveness.connection_installed(at(t, 8_000));
	assert_eq!(liveness.baseline(), Some(at(t, 0)));
	assert!(liveness.commands_received(), "commands prove the new claim");
	assert_eq!(liveness.baseline(), Some(at(t, 8_000)));
}
