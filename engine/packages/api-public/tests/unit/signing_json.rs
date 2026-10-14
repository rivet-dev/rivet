use super::*;
#[test]
fn duplicate_and_unknown_fields_are_rejected_without_echoing_payloads() {
	for bytes in [
		br#"{"audience":"one","audience":"SECRET_SENTINEL"}"#.as_slice(),
		br#"{"claims":{"ds":{"key":1,"key":"SECRET_SENTINEL"}}}"#,
	] {
		let error = parse::<Value>(bytes).unwrap_err();
		assert!(!format!("{error:?}").contains("SECRET_SENTINEL"));
	}
	assert!(parse::<Value>(br#"{"claims":{"ds":{"key":1}}}"#).is_ok());
	assert!(
		parse::<rivet_api_types::auth::application_tokens::CreateRequest>(
			br#"{"audience":"ds","purpose":"callback","claims":{},"iss":"injected"}"#
		)
		.is_err()
	);
}
