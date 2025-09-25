use anyhow::anyhow;
use axum::{
	extract::{
		Request,
		rejection::{ExtensionRejection, JsonRejection},
		{FromRequest, FromRequestParts},
	},
	response::IntoResponse,
};
use axum_extra::extract::QueryRejection;
use http::request::Parts;
use serde::Serialize;

use crate::{error_response::ApiError, errors::ApiBadRequest};

pub struct ExtractorError(ApiError);

impl IntoResponse for ExtractorError {
	fn into_response(self) -> axum::response::Response {
		let mut res = self.0.into_response();

		res.extensions_mut().insert(FailedExtraction);

		res
	}
}

#[derive(Clone, Copy)]
pub struct FailedExtraction;

pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
	axum::extract::Json<T>: FromRequest<S, Rejection = JsonRejection>,
	S: Send + Sync,
{
	type Rejection = ExtractorError;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		axum::extract::Json::<T>::from_request(req, state)
			.await
			.map(|json| Json(json.0))
			.map_err(|err| {
				ExtractorError(
					ApiBadRequest {
						reason: err.body_text(),
					}
					.build()
					.into(),
				)
			})
	}
}

impl<T: Serialize> IntoResponse for Json<T> {
	fn into_response(self) -> axum::response::Response {
		let Self(value) = self;
		axum::extract::Json(value).into_response()
	}
}

pub struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
	axum_extra::extract::Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
	S: Send + Sync,
{
	type Rejection = ExtractorError;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let res = axum_extra::extract::Query::<T>::from_request_parts(parts, state)
			.await
			.map(|query| Query(query.0))
			.map_err(|err| {
				ExtractorError(
					ApiBadRequest {
						reason: err.body_text(),
					}
					.build()
					.into(),
				)
			});

		res
	}
}

pub struct Extension<T>(pub T);

impl<S, T> FromRequestParts<S> for Extension<T>
where
	axum::extract::Extension<T>: FromRequestParts<S, Rejection = ExtensionRejection>,
	S: Send + Sync,
{
	type Rejection = ExtractorError;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		axum::extract::Extension::<T>::from_request_parts(parts, state)
			.await
			.map(|ext| Extension(ext.0))
			.map_err(|err| {
				ExtractorError(
					anyhow!("developer error: extension error: {}", err.body_text()).into(),
				)
			})
	}
}
