use rivet_config::config::Pegboard;

#[test]
fn actor_liveness_check_interval_defaults_to_two_minutes() {
	assert_eq!(Pegboard::default().actor_liveness_check_interval(), 120_000);
	let pegboard: Pegboard =
		serde_json::from_str(r#"{"actor_liveness_check_interval":30000}"#).unwrap();
	assert_eq!(pegboard.actor_liveness_check_interval(), 30_000);
}

#[test]
fn actor_liveness_check_interval_rejects_zero() {
	let pegboard: Pegboard =
		serde_json::from_str(r#"{"actor_liveness_check_interval":0}"#).unwrap();
	let err = pegboard.validate().unwrap_err();
	assert!(
		err.to_string()
			.contains("pegboard.actor_liveness_check_interval")
	);
}
