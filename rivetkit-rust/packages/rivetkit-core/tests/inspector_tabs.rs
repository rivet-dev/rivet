mod moved_tests {
	use std::path::PathBuf;

	use crate::ActorConfig;
	use crate::inspector::{InspectorTabEntry, validate_inspector_tabs};

	fn custom(id: &str, label: &str, source: &str) -> InspectorTabEntry {
		InspectorTabEntry::Custom {
			id: id.to_owned(),
			label: label.to_owned(),
			icon: None,
			root: PathBuf::from(source),
		}
	}

	fn hide(id: &str) -> InspectorTabEntry {
		InspectorTabEntry::HideBuiltin { id: id.to_owned() }
	}

	#[test]
	fn validate_accepts_happy_path() {
		let entries = vec![custom("hello", "Hello", "/abs/path"), hide("queue")];
		assert!(validate_inspector_tabs(&entries).is_ok());
	}

	#[test]
	fn validate_accepts_empty_list() {
		assert!(validate_inspector_tabs(&[]).is_ok());
	}

	#[test]
	fn validate_rejects_empty_custom_id() {
		let entries = vec![custom("", "Hello", "/abs/path")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(
			err.to_string().contains("must be non-empty"),
			"expected non-empty error, got: {err}",
		);
	}

	#[test]
	fn validate_rejects_custom_id_with_slash() {
		let entries = vec![custom("foo/bar", "Hello", "/abs/path")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(
			err.to_string().contains("[a-zA-Z0-9_-]"),
			"expected grammar error, got: {err}",
		);
	}

	#[test]
	fn validate_rejects_custom_id_with_unicode() {
		let entries = vec![custom("héllo", "Hello", "/abs/path")];
		assert!(validate_inspector_tabs(&entries).is_err());
	}

	#[test]
	fn validate_rejects_custom_id_with_dot() {
		// Dots are common in package-style ids; we still reject them so
		// asset URLs are unambiguous.
		let entries = vec![custom("my.tab", "Hello", "/abs/path")];
		assert!(validate_inspector_tabs(&entries).is_err());
	}

	#[test]
	fn validate_rejects_custom_id_colliding_with_builtin() {
		for builtin in &[
			"workflow",
			"database",
			"state",
			"queue",
			"connections",
			"console",
		] {
			let entries = vec![custom(builtin, "Hello", "/abs/path")];
			let err = validate_inspector_tabs(&entries).unwrap_err();
			assert!(
				err.to_string().contains("collides with a built-in tab"),
				"expected builtin-collision error for {builtin:?}, got: {err}",
			);
		}
	}

	#[test]
	fn validate_rejects_empty_label() {
		let entries = vec![custom("hello", "", "/abs/path")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("empty label"));
	}

	#[test]
	fn validate_rejects_empty_source() {
		let entries = vec![custom("hello", "Hello", "")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("empty source path"));
	}

	#[test]
	fn validate_rejects_empty_icon_string() {
		let entries = vec![InspectorTabEntry::Custom {
			id: "hello".to_owned(),
			label: "Hello".to_owned(),
			icon: Some(String::new()),
			root: PathBuf::from("/abs/path"),
		}];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("empty icon"));
	}

	#[test]
	fn validate_accepts_named_icon() {
		let entries = vec![InspectorTabEntry::Custom {
			id: "hello".to_owned(),
			label: "Hello".to_owned(),
			icon: Some("tag".to_owned()),
			root: PathBuf::from("/abs/path"),
		}];
		assert!(validate_inspector_tabs(&entries).is_ok());
	}

	#[test]
	fn validate_rejects_hide_for_unknown_builtin() {
		let entries = vec![hide("not-a-real-builtin")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("not a known built-in"));
	}

	#[test]
	fn validate_rejects_duplicate_ids() {
		let entries = vec![
			custom("hello", "Hello", "/abs/path"),
			custom("hello", "Hi", "/other/path"),
		];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("duplicate id"));
	}

	#[test]
	fn validate_rejects_duplicate_id_across_custom_and_hide() {
		// `queue` can't be both hidden and custom (the custom one would
		// have failed builtin-collision; the hide entry alone is fine).
		// This case is the order-independent duplicate id rule.
		let entries = vec![hide("queue"), hide("queue")];
		let err = validate_inspector_tabs(&entries).unwrap_err();
		assert!(err.to_string().contains("duplicate id"));
	}

	#[test]
	fn actor_config_validate_proxies_to_tabs() {
		let bad = ActorConfig {
			inspector_tabs: vec![custom("", "Hello", "/abs/path")],
			..ActorConfig::default()
		};
		assert!(bad.validate().is_err());

		let ok = ActorConfig {
			inspector_tabs: vec![custom("hello", "Hello", "/abs/path"), hide("queue")],
			..ActorConfig::default()
		};
		assert!(ok.validate().is_ok());

		let empty = ActorConfig::default();
		assert!(empty.validate().is_ok());
	}
}
