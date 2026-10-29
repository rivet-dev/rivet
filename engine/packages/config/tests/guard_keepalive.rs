use std::time::Duration;

use rivet_config::config::guard::Guard;

#[test]
fn websocket_keepalive_interval_is_configurable_and_positive() {
	assert_eq!(
		Guard::default().websocket_keepalive_interval(),
		Duration::from_secs(30)
	);
	let mut guard = Guard {
		websocket_keepalive_interval_ms: Some(50),
		..Default::default()
	};
	assert_eq!(
		guard.websocket_keepalive_interval(),
		Duration::from_millis(50)
	);
	assert!(guard.validate().is_ok());
	guard.websocket_keepalive_interval_ms = Some(0);
	assert!(guard.validate().is_err());
}
