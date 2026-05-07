use super::*;
use rivet_error::{MacroMarker, RivetErrorSchema};

#[test]
fn preserves_internal_anyhow_message_until_client_boundary() {
	let error = ActionDispatchError::from_anyhow(anyhow::anyhow!("plain failure"));

	assert_eq!(error.group, "core");
	assert_eq!(error.code, "internal_error");
	assert_eq!(error.message, "plain failure");
	assert_eq!(error.client_message(), "An internal error occurred");
	assert_eq!(error.client_metadata(), None);
}

#[test]
fn preserves_public_error_message_for_client_boundary() {
	static TEST_ERROR: RivetErrorSchema = RivetErrorSchema {
		group: "actor",
		code: "action_not_found",
		default_message: "action `missing` was not found",
		meta_type: None,
		_macro_marker: MacroMarker { _private: () },
	};
	let error = ActionDispatchError::from_anyhow(TEST_ERROR.build());

	assert_eq!(error.group, "actor");
	assert_eq!(error.code, "action_not_found");
	assert_eq!(error.message, "action `missing` was not found");
	assert_eq!(error.client_message(), "action `missing` was not found");
}

#[test]
fn masks_private_structured_message_at_client_boundary() {
	static TEST_ERROR: RivetErrorSchema = RivetErrorSchema {
		group: "sqlite",
		code: "remote_execution_failed",
		default_message: "private storage failure",
		meta_type: None,
		_macro_marker: MacroMarker { _private: () },
	};
	let error = ActionDispatchError::from_anyhow(TEST_ERROR.build());

	assert_eq!(error.group, "sqlite");
	assert_eq!(error.code, "remote_execution_failed");
	assert_eq!(error.message, "private storage failure");
	assert_eq!(error.client_message(), "An internal error occurred");
	assert_eq!(error.client_metadata(), None);
}
