use crate::{DecodedToken, KeyId};

#[derive(Clone, Debug)]
pub struct VerifiedJwt {
	pub token: DecodedToken,
	pub kid: KeyId,
	/// Absolute Unix timestamp in seconds. Long-lived streams close at this deadline.
	pub authorization_deadline: u64,
}
