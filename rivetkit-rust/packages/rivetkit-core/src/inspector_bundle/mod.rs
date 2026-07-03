//! Inspector UI bundle serving for the shared inspector paths:
//!
//!   GET /inspector/ui/             -> index.html
//!   GET /inspector/ui/<rel>        -> assets/... or other static files
//!   GET /inspector/tab.css         -> shared --rivet-* token stylesheet
//!
//! The bundle is embedded into the binary at build time via `include_dir!`
//! (staged from `frontend/dist/inspector-ui` and `frontend/dist/inspector-tab`
//! by `build.rs`) and served entirely from memory. This is the only serving
//! path: there is no filesystem-root mode and no CDN fallback, so every runner
//! (native or wasm) serves the same embedded bytes.
//!
//! Per-actor public paths (`/inspector/custom-tabs/*`) are NOT served here;
//! they depend on the actor's config and live in the inspector handler itself.

use std::collections::HashMap;

use ::http::StatusCode;
use include_dir::{Dir, include_dir};
use rivet_envoy_client::config::HttpResponse;
use serde_json::json;

/// Inspector-UI frontend bundle. Staged into `$OUT_DIR/inspector-ui` by
/// `build.rs` and embedded at compile time, so the bytes ship inside the
/// binary on every target including wasm.
static INSPECTOR_UI_DIR: Dir<'_> = include_dir!("$OUT_DIR/inspector-ui");

/// Shared stylesheet served to custom inspector tabs at `/inspector/tab.css`.
/// Authored by `frontend/scripts/generate-inspector-tab-css.mjs`, which mirrors
/// the dashboard's design tokens so tabs that `<link>` to it look native.
static INSPECTOR_TAB_DIR: Dir<'_> = include_dir!("$OUT_DIR/inspector-tab");

/// Output filename written by `frontend/scripts/generate-inspector-tab-css.mjs`.
/// Renaming the generator output without updating this produces a silent 404.
const TAB_STYLESHEET_FILE: &str = "styles.css";

/// Inline HTML shown inside the custom-tab iframe on wasm runtimes, which
/// cannot read `inspector.tabs[].source` files from disk. The tab still
/// appears in the dashboard tab strip because tab config keeps flowing from
/// core via the inspector WS `Init` message; only the custom-tab content
/// iframe degrades.
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

/// Try to serve a request from the embedded inspector-UI bundle.
///
/// Returns `Some(HttpResponse)` for any of the shared public paths (a 200 with
/// the embedded bytes, or a 404 if the asset isn't bundled). Returns `None` for
/// paths that aren't part of the shared bundle so the caller can fall through to
/// per-actor / authenticated handling.
pub fn serve_inspector_bundle(method: &str, pathname: &str) -> Option<HttpResponse> {
	if method != "GET" {
		return None;
	}
	if pathname == "/inspector/tab.css" {
		return Some(serve_tab_stylesheet());
	}
	let rel = map_ui_pathname_to_rel(pathname)?;
	if is_unsafe_rel(&rel) {
		return Some(not_found_response());
	}
	Some(serve_ui_asset(&rel))
}

/// Wasm runtimes cannot read `inspector.tabs[].source` files from disk, so
/// `GET /inspector/custom-tabs/*` short-circuits to this styled HTML page
/// inside the iframe.
pub fn serve_wasm_custom_tab_unavailable() -> HttpResponse {
	HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers: shared_response_headers("text/html; charset=utf-8"),
		body: Some(WASM_CUSTOM_TAB_UNAVAILABLE_HTML.as_bytes().to_vec()),
		body_stream: None,
	}
}

// =============================================================================
// Embedded serve
// =============================================================================

fn serve_ui_asset(rel: &str) -> HttpResponse {
	// Single-entry SPA: `/inspector/ui/` serves index.html, every other path
	// under that prefix is an asset relative to the bundle root.
	let stripped = rel.trim_start_matches('/');
	let candidate = if stripped.is_empty() || stripped.ends_with('/') {
		format!("{stripped}index.html")
	} else {
		stripped.to_owned()
	};
	match INSPECTOR_UI_DIR.get_file(&candidate) {
		Some(file) => ok_response(mime_of(&candidate), file.contents().to_vec()),
		None => not_found_response(),
	}
}

fn serve_tab_stylesheet() -> HttpResponse {
	match INSPECTOR_TAB_DIR.get_file(TAB_STYLESHEET_FILE) {
		Some(file) => ok_response("text/css; charset=utf-8", file.contents().to_vec()),
		None => not_found_response(),
	}
}

// =============================================================================
// Helpers
// =============================================================================

fn map_ui_pathname_to_rel(pathname: &str) -> Option<String> {
	if pathname == "/inspector/ui/" || pathname == "/inspector/ui" {
		return Some("index.html".to_owned());
	}
	pathname
		.strip_prefix("/inspector/ui/")
		.map(|rest| rest.to_owned())
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

fn ok_response(content_type: &str, body: Vec<u8>) -> HttpResponse {
	HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers: shared_response_headers(content_type),
		body: Some(body),
		body_stream: None,
	}
}

fn not_found_response() -> HttpResponse {
	let body = json!({
		"group": "inspector",
		"code": "ui_asset_not_found",
		"message": "Inspector UI asset was not found in the embedded bundle",
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn map_ui_pathname_to_rel_maps_known_paths() {
		assert_eq!(
			map_ui_pathname_to_rel("/inspector/ui").as_deref(),
			Some("index.html"),
		);
		assert_eq!(
			map_ui_pathname_to_rel("/inspector/ui/").as_deref(),
			Some("index.html"),
		);
		assert_eq!(
			map_ui_pathname_to_rel("/inspector/ui/assets/app.js").as_deref(),
			Some("assets/app.js"),
		);
		assert!(map_ui_pathname_to_rel("/inspector/state").is_none());
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
	fn non_get_and_unrelated_paths_are_passthrough() {
		assert!(serve_inspector_bundle("POST", "/inspector/ui/").is_none());
		assert!(serve_inspector_bundle("GET", "/inspector/state").is_none());
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
}
