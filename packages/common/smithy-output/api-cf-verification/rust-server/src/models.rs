#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerifyCustomHostnameRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerifyCustomHostnameResponse {
	#[allow(missing_docs)] // documentation missing in model
	pub body: std::string::String,
}

