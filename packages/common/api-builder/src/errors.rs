use rivet_error::*;

#[derive(RivetError)]
#[error("api", "not_found", "The requested resource was not found")]
pub struct ApiNotFound;

#[derive(RivetError)]
#[error("api", "invalid_token", "The provided authentication token is invalid")]
pub struct ApiInvalidToken;

#[derive(RivetError)]
#[error("api", "unauthorized", "Authentication required")]
pub struct ApiUnauthorized;

#[derive(RivetError)]
#[error("api", "forbidden", "Access denied")]
pub struct ApiForbidden;

#[derive(RivetError)]
#[error("api", "internal_error", "An internal server error occurred")]
pub struct ApiInternalError;
