use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::ApiFrom;

#[derive(Debug, Deserialize)]
pub struct CloudflareResponse {
	pub result: CloudflareResult,
}

#[derive(Debug, Deserialize)]
pub struct CloudflareResult {
	pub hostname: String,
	pub status: CloudflareVerificationStatus,
	#[serde(default = "Vec::new")]
	pub verification_errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudflareVerificationStatus {
	Active,
	Pending,
	ActiveRedeploying,
	Moved,
	PendingDeletion,
	Deleted,
	PendingBlocked,
	PendingMigration,
	PendingProvisioned,
	TestPending,
	TestActive,
	TestActiveApex,
	TestBlocked,
	TestFailed,
	Provisioned,
	Blocked,
}

impl ApiFrom<CloudflareVerificationStatus> for models::CloudCdnNamespaceDomainVerificationStatus {
	fn api_from(value: CloudflareVerificationStatus) -> Self {
		use CloudflareVerificationStatus::*;

		match value {
			Active | ActiveRedeploying | TestActive | TestActiveApex => {
				models::CloudCdnNamespaceDomainVerificationStatus::Active
			}
			Pending | PendingMigration | PendingProvisioned | TestPending => {
				models::CloudCdnNamespaceDomainVerificationStatus::Pending
			}
			Deleted | PendingDeletion | Moved | PendingBlocked | TestBlocked | TestFailed
			| Provisioned | Blocked => models::CloudCdnNamespaceDomainVerificationStatus::Failed,
		}
	}
}

impl ApiFrom<CloudflareVerificationStatus> for backend::cf::custom_hostname::Status {
	fn api_from(value: CloudflareVerificationStatus) -> Self {
		use CloudflareVerificationStatus::*;

		match value {
			Active | ActiveRedeploying | TestActive | TestActiveApex => {
				backend::cf::custom_hostname::Status::Active
			}
			Pending | PendingMigration | PendingProvisioned | TestPending => {
				backend::cf::custom_hostname::Status::Pending
			}
			Deleted | PendingDeletion | Moved | PendingBlocked | TestBlocked | TestFailed
			| Provisioned | Blocked => backend::cf::custom_hostname::Status::Failed,
		}
	}
}
