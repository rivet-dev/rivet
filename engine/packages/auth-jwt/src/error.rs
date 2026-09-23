use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
	#[error("JWT exceeds the protocol size limit")]
	TokenTooLarge,
	#[error("JWT has an invalid compact encoding")]
	InvalidEncoding,
	#[error("JWT has an unsupported header")]
	InvalidHeader,
	#[error("JWT key ID is invalid")]
	InvalidKeyId,
	#[error("JWT key is invalid")]
	InvalidKey,
	#[error("JWT signature is invalid")]
	InvalidSignature,
	#[error("JWT claims are invalid")]
	InvalidClaims,
	#[error("JWT has expired")]
	Expired,
	#[error("JWT was issued in the future")]
	IssuedInFuture,
	#[error("JWT exceeds the maximum protocol lifetime")]
	LifetimeTooLong,
	#[error("JWT grants are invalid")]
	InvalidGrants,
}
