#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// Represents the state of an external account linking process.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompleteStatus {
	/// The current linking process has already completed.
	AlreadyComplete,
	/// The current linking process has expired.
	Expired,
	/// The code given to the current linking process is incorrect.
	Incorrect,
	/// The linking process succeeded and the new account is now added.
	LinkedAccountAdded,
	/// The linking process succeeded and will now switch identities.
	SwitchIdentity,
	/// The current linking process has been tried too many times.
	TooManyAttempts,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for CompleteStatus {
	fn from(s: &str) -> Self {
		match s {
			"already_complete" => CompleteStatus::AlreadyComplete,
			"expired" => CompleteStatus::Expired,
			"incorrect" => CompleteStatus::Incorrect,
			"linked_account_added" => CompleteStatus::LinkedAccountAdded,
			"switch_identity" => CompleteStatus::SwitchIdentity,
			"too_many_attempts" => CompleteStatus::TooManyAttempts,
			other => CompleteStatus::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for CompleteStatus {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(CompleteStatus::from(s))
	}
}
impl CompleteStatus {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			CompleteStatus::AlreadyComplete => "already_complete",
			CompleteStatus::Expired => "expired",
			CompleteStatus::Incorrect => "incorrect",
			CompleteStatus::LinkedAccountAdded => "linked_account_added",
			CompleteStatus::SwitchIdentity => "switch_identity",
			CompleteStatus::TooManyAttempts => "too_many_attempts",
			CompleteStatus::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&[
			"already_complete",
			"expired",
			"incorrect",
			"linked_account_added",
			"switch_identity",
			"too_many_attempts",
		]
	}
}
impl AsRef<str> for CompleteStatus {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Methods to verify a captcha.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaConfig {
	/// hCaptcha configuration.
	Hcaptcha(CaptchaConfigHcaptcha),
	/// Cloudflare Turnstile configuration.
	Turnstile(CaptchaConfigTurnstile),
}

/// Cloudflare Turnstile configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaptchaConfigTurnstile {
	#[allow(missing_docs)] // documentation missing in model
	pub client_response: std::string::String,
}

/// hCaptcha configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaptchaConfigHcaptcha {
	#[allow(missing_docs)] // documentation missing in model
	pub client_response: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteEmailVerificationRequest {
	/// A universally unique identifier.
	pub verification_id: std::string::String,
	/// The code sent to the requestee's email.
	pub code: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartEmailVerificationRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub email: std::string::String,
	/// Methods to verify a captcha.
	pub captcha: CaptchaConfig,
	/// A universally unique identifier.
	pub game_id: std::option::Option<std::string::String>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefreshIdentityTokenRequest {
	/// When `true`, the current identity for the provided cookie will be logged out and a new identity will be returned.
	pub logout: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteEmailVerificationResponse {
	/// Represents the state of an external account linking process.
	pub status: CompleteStatus,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartEmailVerificationResponse {
	/// A universally unique identifier.
	pub verification_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefreshIdentityTokenResponse {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
	/// Token expiration time (in milliseconds).
	pub exp: chrono::DateTime<chrono::Utc>,
	/// A universally unique identifier.
	pub identity_id: std::string::String,
}

