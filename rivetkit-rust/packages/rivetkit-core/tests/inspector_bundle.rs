//! Release check that the embedded Inspector UI bundle is actually served.
//!
//! Gated on `RIVETKIT_ASSERT_INSPECTOR_BUNDLE`: the bundle is only staged during
//! publish (see `scripts/stage-inspector-bundle.mjs`), so ordinary `cargo test`
//! runs without a built frontend must still pass. `scripts/verify-inspector-bundle.mjs`
//! sets the env var during the release check.

use rivetkit_core::inspector_bundle::serve_inspector_bundle;

#[test]
fn embedded_bundle_serves_index_when_required() {
	if std::env::var_os("RIVETKIT_ASSERT_INSPECTOR_BUNDLE").is_none() {
		eprintln!(
			"skipping: set RIVETKIT_ASSERT_INSPECTOR_BUNDLE=1 to require the embedded bundle"
		);
		return;
	}

	let resp = serve_inspector_bundle("GET", "/inspector/ui/")
		.expect("/inspector/ui/ is a public bundle path");

	let body = resp.body.expect("response has a body");
	let text = String::from_utf8_lossy(&body);

	assert!(
		!text.contains("ui_asset_not_found"),
		"embedded inspector bundle is empty: GET /inspector/ui/ served ui_asset_not_found",
	);
	assert_eq!(
		resp.status, 200,
		"expected 200 for index.html, got {}",
		resp.status
	);

	let lower = text.to_ascii_lowercase();
	assert!(
		lower.contains("<!doctype html") || lower.contains("<html"),
		"expected HTML index.html, got: {}",
		&text[..text.len().min(200)],
	);
}
