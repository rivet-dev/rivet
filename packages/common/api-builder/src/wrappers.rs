use anyhow::Result;
use axum::{
	body::Bytes,
	extract::{Extension, Path, Query},
	response::{IntoResponse, Json},
	routing::{
		delete as axum_delete, get as axum_get, patch as axum_patch, post as axum_post,
		put as axum_put,
	},
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

use crate::{context::ApiCtx, error_response::ApiError};

/// Macro to generate wrapper functions for HTTP methods
macro_rules! create_method_wrapper {
	// Variant for methods without body (GET, DELETE)
	($name:ident, $axum_method:ident, without_body) => {
		pub fn $name<P, Q, R, F, Fut>(
			handler: F,
		) -> axum::routing::MethodRouter<crate::GlobalApiCtx>
		where
			P: DeserializeOwned + Send + 'static,
			Q: DeserializeOwned + Send + 'static,
			R: Serialize + Send + 'static,
			F: FnOnce(ApiCtx, P, Q) -> Fut + Clone + Send + Sync + 'static,
			Fut: Future<Output = Result<R>> + Send,
		{
			$axum_method(
				move |Extension(ctx): Extension<ApiCtx>,
				      Path(path): Path<P>,
				      Query(query): Query<Q>| async move {
					match handler(ctx, path, query).await {
						Ok(response) => Json(response).into_response(),
						Err(err) => ApiError::from(err).into_response(),
					}
				},
			)
		}
	};

	// Variant for methods with body (POST, PUT, PATCH)
	($name:ident, $axum_method:ident, with_body) => {
		pub fn $name<P, Q, B, R, F, Fut>(
			handler: F,
		) -> axum::routing::MethodRouter<crate::GlobalApiCtx>
		where
			P: DeserializeOwned + Send + 'static,
			Q: DeserializeOwned + Send + 'static,
			B: DeserializeOwned + Send + 'static,
			R: Serialize + Send + 'static,
			F: FnOnce(ApiCtx, P, Q, B) -> Fut + Clone + Send + Sync + 'static,
			Fut: Future<Output = Result<R>> + Send,
		{
			$axum_method(
				move |Extension(ctx): Extension<ApiCtx>,
				      Path(path): Path<P>,
				      Query(query): Query<Q>,
				      Json(body): Json<B>| async move {
					match handler(ctx, path, query, body).await {
						Ok(response) => Json(response).into_response(),
						Err(err) => ApiError::from(err).into_response(),
					}
				},
			)
		}
	};
}

// Generate wrapper functions for each HTTP method
create_method_wrapper!(get, axum_get, without_body);
create_method_wrapper!(delete, axum_delete, without_body);
create_method_wrapper!(post, axum_post, with_body);
create_method_wrapper!(put, axum_put, with_body);
create_method_wrapper!(patch, axum_patch, with_body);

/// Macro to generate binary wrapper functions for HTTP methods
macro_rules! create_binary_method_wrapper {
	// Variant for methods without body (GET, DELETE)
	($name:ident, $axum_method:ident, without_body) => {
		pub fn $name<P, Q, F, Fut>(handler: F) -> axum::routing::MethodRouter<crate::GlobalApiCtx>
		where
			P: DeserializeOwned + Send + 'static,
			Q: DeserializeOwned + Send + 'static,
			F: FnOnce(ApiCtx, P, Q) -> Fut + Clone + Send + Sync + 'static,
			Fut: Future<Output = Result<Vec<u8>>> + Send,
		{
			$axum_method(
				move |Extension(ctx): Extension<ApiCtx>,
				      Path(path): Path<P>,
				      Query(query): Query<Q>| async move {
					match handler(ctx, path, query).await {
						Ok(response) => Bytes::from(response).into_response(),
						Err(err) => ApiError::from(err).into_response(),
					}
				},
			)
		}
	};

	// Variant for methods with body (POST, PUT, PATCH)
	($name:ident, $axum_method:ident, with_body) => {
		pub fn $name<P, Q, F, Fut>(handler: F) -> axum::routing::MethodRouter<crate::GlobalApiCtx>
		where
			P: DeserializeOwned + Send + 'static,
			Q: DeserializeOwned + Send + 'static,
			F: FnOnce(ApiCtx, P, Q, Bytes) -> Fut + Clone + Send + Sync + 'static,
			Fut: Future<Output = Result<Vec<u8>>> + Send,
		{
			$axum_method(
				move |Extension(ctx): Extension<ApiCtx>,
				      Path(path): Path<P>,
				      Query(query): Query<Q>,
				      body: Bytes| async move {
					match handler(ctx, path, query, body).await {
						Ok(response) => Bytes::from(response).into_response(),
						Err(err) => ApiError::from(err).into_response(),
					}
				},
			)
		}
	};
}

// Generate wrapper functions for each HTTP method
pub mod bin {
	use super::*;

	create_binary_method_wrapper!(get, axum_get, without_body);
	create_binary_method_wrapper!(delete, axum_delete, without_body);
	create_binary_method_wrapper!(post, axum_post, with_body);
	create_binary_method_wrapper!(put, axum_put, with_body);
	create_binary_method_wrapper!(patch, axum_patch, with_body);
}
