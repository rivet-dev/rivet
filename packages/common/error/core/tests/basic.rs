use rivet_error::*;
use serde::{Deserialize, Serialize};

// Set RIVET_ERROR_OUTPUT_DIR to avoid writing docs during tests
fn init_test() {
	unsafe {
		std::env::set_var("RIVET_ERROR_OUTPUT_DIR", "/tmp/rivet_error_test");
	}
}

// Simple error without metadata
#[derive(RivetError)]
#[error("test", "simple_error", "This is a simple test error")]
struct TestError;

// Error with metadata
#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"test",
	"meta_error",
	"Default message",
	"Error with value {value} and name {name}"
)]
struct TestMetaError {
	value: i64,
	name: String,
}

#[test]
fn test_simple_error_build() {
	init_test();
	let error = TestError.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "test");
	assert_eq!(rivet_error.code(), "simple_error");
	assert_eq!(rivet_error.message(), "This is a simple test error");
	assert!(rivet_error.meta.is_none());
}

#[test]
fn test_meta_error_build() {
	init_test();
	let error = TestMetaError {
		value: 42,
		name: "test_name".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "test");
	assert_eq!(rivet_error.code(), "meta_error");
	assert_eq!(
		rivet_error.message(),
		"Error with value 42 and name test_name"
	);
	assert!(rivet_error.meta.is_some());
}

#[test]
fn test_internal_error_extraction() {
	let regular_error = anyhow::anyhow!("Some random error");
	let rivet_error = RivetError::extract(&regular_error);

	assert_eq!(rivet_error.group(), "core");
	assert_eq!(rivet_error.code(), "internal_error");
	assert!(rivet_error.message().contains("An internal error occurred"));
	assert!(rivet_error.meta.is_some());
}

#[test]
fn test_error_serialization() {
	init_test();
	let error = TestError.build();
	let rivet_error = RivetError::extract(&error);

	let json = serde_json::to_string(&rivet_error).unwrap();
	let value: serde_json::Value = serde_json::from_str(&json).unwrap();

	assert_eq!(value["group"], "test");
	assert_eq!(value["code"], "simple_error");
	assert_eq!(value["message"], "This is a simple test error");
	assert!(value.get("meta").is_none());
}

#[test]
fn test_meta_error_serialization() {
	init_test();
	let error = TestMetaError {
		value: 100,
		name: "serialization_test".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	let json = serde_json::to_string(&rivet_error).unwrap();
	let value: serde_json::Value = serde_json::from_str(&json).unwrap();

	assert_eq!(value["group"], "test");
	assert_eq!(value["code"], "meta_error");
	assert_eq!(
		value["message"],
		"Error with value 100 and name serialization_test"
	);

	let meta_value = value["meta"].as_object().unwrap();
	assert_eq!(meta_value["value"], 100);
	assert_eq!(meta_value["name"], "serialization_test");
}

#[test]
fn test_error_chaining() {
	init_test();
	let _base_error = anyhow::anyhow!("Base error");
	let wrapped_error = TestError.build();

	let extracted = RivetError::extract(&wrapped_error);
	assert_eq!(extracted.code(), "simple_error");
}

// Test struct without formatted description (from derive.rs)
#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"namespace",
	"invalid_name",
	"
	Invalid namespace name.
	
	Namespace names must be valid DNS subdomains.
	"
)]
struct NamespaceInvalidName {
	name: String,
	reason: String,
}

#[test]
fn test_struct_without_formatted_description() {
	init_test();
	let error = NamespaceInvalidName {
		name: "invalid name".to_string(),
		reason: "contains spaces".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "namespace");
	assert_eq!(rivet_error.code(), "invalid_name");
	assert!(rivet_error.message().contains("Invalid namespace name"));
	assert!(rivet_error.meta.is_some());

	// Check that metadata was serialized
	let meta_value: serde_json::Value =
		serde_json::from_str(rivet_error.meta.as_ref().unwrap().get()).unwrap();
	assert_eq!(meta_value["name"], "invalid name");
	assert_eq!(meta_value["reason"], "contains spaces");
}

// Test multiline description formatting (from derive.rs)
#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"api",
	"rate_limited",
	"
	Rate limit exceeded.
	
	The API rate limit has been exceeded for this endpoint.
	Please wait before making additional requests.
	",
	"Rate limit exceeded. Limit: {limit}, resets at: {reset_at}"
)]
struct ApiRateLimited {
	limit: u32,
	reset_at: i64,
}

#[test]
fn test_multiline_descriptions() {
	init_test();
	let error = ApiRateLimited {
		limit: 100,
		reset_at: 1234567890,
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "api");
	assert_eq!(rivet_error.code(), "rate_limited");
	assert_eq!(
		rivet_error.message(),
		"Rate limit exceeded. Limit: 100, resets at: 1234567890"
	);
}

#[test]
fn test_metadata_for_simple_error() {
	init_test();
	let error = TestError.build();
	let rivet_error = RivetError::extract(&error);

	// Simple error should not have metadata
	assert!(rivet_error.metadata().is_none());
}

#[test]
fn test_metadata_for_error_with_fields() {
	init_test();
	let error = TestMetaError {
		value: 42,
		name: "test_name".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	// Error with fields should have metadata
	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert_eq!(metadata["value"], 42);
	assert_eq!(metadata["name"], "test_name");
}

#[test]
fn test_metadata_for_namespace_error() {
	init_test();
	let error = NamespaceInvalidName {
		name: "invalid-name".to_string(),
		reason: "contains dash".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert_eq!(metadata["name"], "invalid-name");
	assert_eq!(metadata["reason"], "contains dash");
}

#[test]
fn test_metadata_for_rate_limited_error() {
	init_test();
	let error = ApiRateLimited {
		limit: 1000,
		reset_at: 987654321,
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert_eq!(metadata["limit"], 1000);
	assert_eq!(metadata["reset_at"], 987654321);
}

#[test]
fn test_metadata_for_internal_error() {
	let regular_error = anyhow::anyhow!("Some internal error");
	let rivet_error = RivetError::extract(&regular_error);

	// Internal errors should have metadata with the error details
	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert!(metadata["error"].is_string());
	assert!(
		metadata["error"]
			.as_str()
			.unwrap()
			.contains("Some internal error")
	);
}

// Test enum with variants similar to pegboard errors
#[derive(RivetError, Serialize, Deserialize, Clone)]
#[error("test")]
enum TestEnumError {
	#[error("not_found", "The resource does not exist.")]
	NotFound,

	#[error(
		"input_too_large",
		"Input too large.",
		"Input too large (max {max_size})."
	)]
	InputTooLarge { max_size: usize },

	#[error(
		"key_too_large",
		"Key too large.",
		"Key '{key_preview}' too large (max {max_size} bytes)."
	)]
	KeyTooLarge {
		max_size: usize,
		key_preview: String,
	},
}

#[test]
fn test_enum_variant_without_metadata() {
	init_test();
	let error = TestEnumError::NotFound.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "test");
	assert_eq!(rivet_error.code(), "not_found");
	assert_eq!(rivet_error.message(), "The resource does not exist.");
	assert!(rivet_error.metadata().is_none());
}

#[test]
fn test_enum_variant_input_too_large_metadata() {
	init_test();
	let error = TestEnumError::InputTooLarge { max_size: 1024 }.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "test");
	assert_eq!(rivet_error.code(), "input_too_large");
	assert_eq!(rivet_error.message(), "Input too large (max 1024).");

	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert_eq!(metadata["max_size"], 1024);
}

#[test]
fn test_enum_variant_key_too_large_metadata() {
	init_test();
	let error = TestEnumError::KeyTooLarge {
		max_size: 256,
		key_preview: "very_long_key_name...".to_string(),
	}
	.build();
	let rivet_error = RivetError::extract(&error);

	assert_eq!(rivet_error.group(), "test");
	assert_eq!(rivet_error.code(), "key_too_large");
	assert_eq!(
		rivet_error.message(),
		"Key 'very_long_key_name...' too large (max 256 bytes)."
	);

	let metadata = rivet_error.metadata().expect("Should have metadata");
	assert_eq!(metadata["max_size"], 256);
	assert_eq!(metadata["key_preview"], "very_long_key_name...");
}
