//! Inspector UI bundle serving for the shared inspector paths:
//!
//!   GET /inspector/ui/             -> index.html
//!   GET /inspector/ui/<rel>        -> assets/... or other static files
//!   GET /inspector/tab.css         -> shared --rivet-* token stylesheet
//!
//! Bytes are read from a host-supplied absolute filesystem path.
//! The dashboard is responsible for routing the iframe to whoever can
//! actually serve the bundle: for NAPI runners that's the runner process
//! itself via `ServeConfig::inspector_ui_path`; for wasm runners the
//! dashboard loads the bundle directly from a CDN and never asks the
//! runner for `/inspector/ui/*` at all.
//!
//! Per-actor public paths (`/inspector/tab-config`, `/inspector/custom-tabs/*`)
//! are NOT served here; they depend on the actor's config and live in the
//! inspector handler itself.

use std::collections::HashMap;
use std::path::{Component, Path};

use ::http::StatusCode;
use rivet_envoy_client::config::HttpResponse;
use serde_json::json;

/// Inline HTML shown inside the custom-tab iframe on wasm runtimes, which
/// cannot read `inspector.tabs[].source` files from disk. The tab still
/// appears in the dashboard tab strip because tab-config keeps flowing from
/// core; only the custom-tab content iframe degrades.
const WASM_CUSTOM_TAB_UNAVAILABLE_HTML: &str = include_str!("wasm-custom-tab-unavailable.html");

// =============================================================================
// Public dispatch
// =============================================================================

/// Whether a path is one of the shared "public" inspector routes that must
/// never require the per-actor bearer token. Tab-config and custom-tabs are
/// per-actor and must be checked separately by the inspector handler since
/// this module doesn't carry their bytes.
pub fn is_public_inspector_bundle_path(method: &str, pathname: &str) -> bool {
	if method != "GET" {
		return false;
	}
	pathname == "/inspector/tab.css"
		|| pathname == "/inspector/ui/"
		|| pathname == "/inspector/ui"
		|| pathname.starts_with("/inspector/ui/")
}

/// Try to serve a request from the inspector-ui bundle by reading from
/// `fs_root`.
///
/// Returns `Some(HttpResponse)` for any of the shared public paths (a 200
/// with the bytes, or a 404 if the asset isn't on disk). Returns `None`
/// for paths that aren't part of the shared bundle so the caller can fall
/// through to per-actor / authenticated handling.
pub async fn serve_inspector_bundle(
	fs_root: &Path,
	method: &str,
	pathname: &str,
) -> Option<HttpResponse> {
	if method != "GET" {
		return None;
	}
	let rel = map_pathname_to_rel(pathname)?;
	if is_unsafe_rel(&rel) {
		return Some(not_found_response());
	}
	Some(serve_from_fs(fs_root, &rel).await)
}

/// Wasm runtimes cannot read `inspector.tabs[].source` files from disk, so
/// `GET /inspector/custom-tabs/*` short-circuits to this styled HTML page
/// inside the iframe.
pub fn serve_wasm_custom_tab_unavailable() -> HttpResponse {
	let mut headers = HashMap::new();
	headers.insert(
		"Content-Type".to_owned(),
		"text/html; charset=utf-8".to_owned(),
	);
	headers.insert("Cache-Control".to_owned(), "no-cache".to_owned());
	headers.insert("Referrer-Policy".to_owned(), "no-referrer".to_owned());
	HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers,
		body: Some(WASM_CUSTOM_TAB_UNAVAILABLE_HTML.as_bytes().to_vec()),
		body_stream: None,
	}
}

// =============================================================================
// Helpers
// =============================================================================

fn map_pathname_to_rel(pathname: &str) -> Option<String> {
	if pathname == "/inspector/tab.css" {
		return Some("tab.css".to_owned());
	}
	if pathname == "/inspector/ui/" || pathname == "/inspector/ui" {
		return Some("index.html".to_owned());
	}
	if let Some(rest) = pathname.strip_prefix("/inspector/ui/") {
		return Some(rest.to_owned());
	}
	None
}

fn is_unsafe_rel(rel: &str) -> bool {
	if rel.is_empty() {
		return true;
	}
	if rel.starts_with('/') {
		return true;
	}
	for seg in rel.split('/') {
		if seg == ".." || seg == "." {
			return true;
		}
	}
	false
}

fn mime_of(rel: &str) -> &'static str {
	let Some(dot) = rel.rfind('.') else {
		return "application/octet-stream";
	};
	let ext = rel[dot + 1..].to_ascii_lowercase();
	match ext.as_str() {
		"html" | "htm" => "text/html; charset=utf-8",
		"js" | "mjs" | "cjs" => "application/javascript; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"json" | "map" => "application/json; charset=utf-8",
		"svg" => "image/svg+xml",
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"gif" => "image/gif",
		"webp" => "image/webp",
		"ico" => "image/x-icon",
		"woff" => "font-woff",
		"woff2" => "font-woff2",
		"ttf" => "font-ttf",
		"otf" => "font-otf",
		"txt" => "text/plain; charset=utf-8",
		"wasm" => "application/wasm",
		_ => "application/octet-stream",
	}
}

fn shared_response_headers(content_type: &str) -> HashMap<String, String> {
	let mut headers = HashMap::new();
	headers.insert("Content-Type".to_owned(), content_type.to_owned());
	headers.insert("Cache-Control".to_owned(), "no-cache".to_owned());
	headers.insert("Referrer-Policy".to_owned(), "no-referrer".to_owned());
	headers
}

fn not_found_response() -> HttpResponse {
	let body = json!({
		"group": "inspector",
		"code": "ui_asset_not_found",
		"message": "Inspector UI asset was not found",
		"metadata": serde_json::Value::Null,
	});
	let bytes = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
	HttpResponse {
		status: StatusCode::NOT_FOUND.as_u16(),
		headers: shared_response_headers("application/json"),
		body: Some(bytes),
		body_stream: None,
	}
}

// -----------------------------------------------------------------------------
// Filesystem serve
// -----------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
async fn serve_from_fs(root: &Path, rel: &str) -> HttpResponse {
	// Explicit component-by-component guard. The rel-segment check already
	// rejects `..` and `.` segments, but enforce here as defense in depth in
	// case the source set a relative path with traversal bytes.
	let rel_path = Path::new(rel);
	if rel_path
		.components()
		.any(|c| !matches!(c, Component::Normal(_)))
	{
		return not_found_response();
	}

	let target = root.join(rel_path);

	// Containment check: even after rejecting `..`/`.` segments above, a
	// symlink inside the bundle directory could resolve to a target outside
	// `root`. Canonicalize the root once and require the resolved target to
	// stay within it.
	let canonical_root = match root.canonicalize() {
		Ok(p) => p,
		Err(_) => return not_found_response(),
	};
	let canonical = match target.canonicalize() {
		Ok(p) => p,
		Err(err) => {
			tracing::debug!(
				rel,
				root = %root.display(),
				error = %err,
				"inspector bundle fs read failed"
			);
			return not_found_response();
		}
	};
	if canonical != canonical_root && !canonical.starts_with(&canonical_root) {
		tracing::warn!(
			rel,
			root = %canonical_root.display(),
			resolved = %canonical.display(),
			"inspector bundle asset escaped its root via symlink"
		);
		return not_found_response();
	}

	let bytes = match tokio::fs::read(&canonical).await {
		Ok(b) => b,
		Err(_) => return not_found_response(),
	};
	HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers: shared_response_headers(mime_of(rel)),
		body: Some(bytes),
		body_stream: None,
	}
}

// Wasm has no real filesystem; if a wasm host somehow ends up with an
// `inspector_ui_path` set in its `ServeConfig`, fail closed.
#[cfg(target_arch = "wasm32")]
async fn serve_from_fs(_root: &Path, _rel: &str) -> HttpResponse {
	not_found_response()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn map_pathname_to_rel_maps_known_paths() {
		assert_eq!(
			map_pathname_to_rel("/inspector/tab.css").as_deref(),
			Some("tab.css"),
		);
		assert_eq!(
			map_pathname_to_rel("/inspector/ui").as_deref(),
			Some("index.html"),
		);
		assert_eq!(
			map_pathname_to_rel("/inspector/ui/").as_deref(),
			Some("index.html"),
		);
		assert_eq!(
			map_pathname_to_rel("/inspector/ui/assets/app.js").as_deref(),
			Some("assets/app.js"),
		);
		assert!(map_pathname_to_rel("/inspector/state").is_none());
	}

	#[test]
	fn is_unsafe_rel_rejects_traversal_and_absolute() {
		assert!(is_unsafe_rel(""));
		assert!(is_unsafe_rel("/etc/passwd"));
		assert!(is_unsafe_rel("../etc/passwd"));
		assert!(is_unsafe_rel("assets/../../etc"));
		assert!(is_unsafe_rel("./assets"));
		assert!(!is_unsafe_rel("assets/app.js"));
		assert!(!is_unsafe_rel("index.html"));
	}

	#[test]
	fn is_public_inspector_bundle_path_matches() {
		assert!(is_public_inspector_bundle_path("GET", "/inspector/ui/"));
		assert!(is_public_inspector_bundle_path("GET", "/inspector/ui"));
		assert!(is_public_inspector_bundle_path(
			"GET",
			"/inspector/ui/assets/app.js"
		));
		assert!(is_public_inspector_bundle_path("GET", "/inspector/tab.css"));
		assert!(!is_public_inspector_bundle_path("POST", "/inspector/ui/"));
		assert!(!is_public_inspector_bundle_path("GET", "/inspector/state"));
	}

	#[test]
	fn wasm_custom_tab_unavailable_is_html() {
		let resp = serve_wasm_custom_tab_unavailable();
		assert_eq!(resp.status, StatusCode::OK.as_u16());
		assert_eq!(
			resp.headers.get("Content-Type").map(String::as_str),
			Some("text/html; charset=utf-8"),
		);
		let body = resp.body.expect("body present");
		assert!(body.starts_with(b"<!doctype html>"));
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::test]
	async fn serve_inspector_bundle_reads_from_fs() {
		let tmp = tempfile::tempdir().expect("tempdir");
		std::fs::write(tmp.path().join("index.html"), b"<!doctype html><p>ok").expect("write");
		let resp = serve_inspector_bundle(tmp.path(), "GET", "/inspector/ui/")
			.await
			.expect("response");
		assert_eq!(resp.status, StatusCode::OK.as_u16());
		assert_eq!(resp.body.unwrap(), b"<!doctype html><p>ok".to_vec());
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::test]
	async fn serve_inspector_bundle_returns_404_for_missing() {
		let tmp = tempfile::tempdir().expect("tempdir");
		let resp = serve_inspector_bundle(tmp.path(), "GET", "/inspector/ui/missing.js")
			.await
			.expect("response");
		assert_eq!(resp.status, StatusCode::NOT_FOUND.as_u16());
	}
}
