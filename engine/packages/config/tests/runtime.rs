use rivet_config::config::Runtime;

#[test]
fn udb_transaction_tags_default_to_disabled_and_can_be_enabled() {
	assert!(!Runtime::default().udb_transaction_tags_enabled());
	let runtime: Runtime = serde_json::from_str("{}").unwrap();
	assert!(!runtime.udb_transaction_tags_enabled());
	let runtime: Runtime =
		serde_json::from_str(r#"{"udb_transaction_tags_enabled":true}"#).unwrap();
	assert!(runtime.udb_transaction_tags_enabled());
}
