//! Extractor for the raw user path after the route prefix that matched.

use axum::{
	extract::{FromRequestParts, MatchedPath, NestedPath, OriginalUri},
	http::request::Parts,
	response::{IntoResponse, Response},
};

/// The raw path after the matched route prefix, exactly as the client sent it.
///
/// The prefix comes from the router itself: a nested router reports the path it is mounted at
/// and a plain route reports its template. axum matches routes against the raw path and every
/// capture covers exactly one segment, so skipping that many raw segments leaves the user path
/// with its percent-encoding and empty segments intact. The query is not included.
pub struct RawRestPath(pub String);

impl<S: Send + Sync> FromRequestParts<S> for RawRestPath {
	type Rejection = Response;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let template = match NestedPath::from_request_parts(parts, state).await {
			Ok(nested) => nested.as_str().to_owned(),
			Err(_) => MatchedPath::from_request_parts(parts, state)
				.await
				.map_err(IntoResponse::into_response)?
				.as_str()
				.to_owned(),
		};
		let OriginalUri(uri) = OriginalUri::from_request_parts(parts, state)
			.await
			.map_err(IntoResponse::into_response)?;

		// A tail wildcard is part of the user path, not the prefix.
		let prefix_segments = template
			.split('/')
			.filter(|segment| !segment.is_empty() && !segment.starts_with("{*"))
			.count();

		let mut rest = uri.path();
		for _ in 0..prefix_segments {
			let Some(after_slash) = rest.strip_prefix('/') else {
				break;
			};
			let segment_end = after_slash.find('/').unwrap_or(after_slash.len());
			rest = &after_slash[segment_end..];
		}

		if rest.is_empty() {
			Ok(RawRestPath("/".to_owned()))
		} else {
			Ok(RawRestPath(rest.to_owned()))
		}
	}
}
