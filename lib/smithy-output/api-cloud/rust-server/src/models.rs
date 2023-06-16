#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadPresignedRequest {
	/// The name of the file to upload. This is the same as the one given in the upload prepare file.
	pub path: std::string::String,
	/// The URL of the presigned request for which to upload your file to.
	pub url: std::string::String,
}

/// A custom avatar summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomAvatarSummary {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// The URL of this custom avatar image. Only present if upload is complete.
	pub url: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
	/// Whether or not this custom avatar has completely been uploaded.
	pub complete: bool,
}

/// A service performance summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SvcPerf {
	/// The name of the service.
	pub svc_name: std::string::String,
	/// RFC3339 timestamp.
	pub ts: chrono::DateTime<chrono::Utc>,
	/// Unsigned 64 bit integer.
	pub duration: i64,
	/// A universally unique identifier.
	pub req_id: std::option::Option<std::string::String>,
	/// A list of performance spans.
	pub spans: std::vec::Vec<LogsPerfSpan>,
	/// A list of performance marks.
	pub marks: std::vec::Vec<LogsPerfMark>,
}

/// A performance mark.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogsPerfMark {
	/// The label given to this performance mark.
	pub label: std::string::String,
	/// RFC3339 timestamp.
	pub ts: chrono::DateTime<chrono::Utc>,
	/// A universally unique identifier.
	pub ray_id: std::option::Option<std::string::String>,
	/// A universally unique identifier.
	pub req_id: std::option::Option<std::string::String>,
}

/// A performance span.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogsPerfSpan {
	/// The label given to this performance span.
	pub label: std::string::String,
	/// RFC3339 timestamp.
	pub start_ts: chrono::DateTime<chrono::Utc>,
	/// RFC3339 timestamp.
	pub finish_ts: std::option::Option<chrono::DateTime<chrono::Utc>>,
	/// A universally unique identifier.
	pub req_id: std::option::Option<std::string::String>,
}

/// An error given by failed content validation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidationError {
	/// A list of strings denoting the origin of a validation error.
	pub path: std::vec::Vec<std::string::String>,
}

/// A group's billing invoice.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupBillingInvoice {
	/// RFC3339 timestamp.
	pub issuing_ts: chrono::DateTime<chrono::Utc>,
	/// A URL to this invoice's PDF document.
	pub file_url: std::option::Option<std::string::String>,
}

/// The status of a developer group.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
	#[allow(missing_docs)] // documentation missing in model
	Active,
	#[allow(missing_docs)] // documentation missing in model
	PaymentFailed,
	#[allow(missing_docs)] // documentation missing in model
	SetupIncomplete,
	#[allow(missing_docs)] // documentation missing in model
	SpendingLimitReached,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GroupStatus {
	fn from(s: &str) -> Self {
		match s {
			"active" => GroupStatus::Active,
			"payment_failed" => GroupStatus::PaymentFailed,
			"setup_incomplete" => GroupStatus::SetupIncomplete,
			"spending_limit_reached" => GroupStatus::SpendingLimitReached,
			other => GroupStatus::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GroupStatus {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GroupStatus::from(s))
	}
}
impl GroupStatus {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GroupStatus::Active => "active",
			GroupStatus::PaymentFailed => "payment_failed",
			GroupStatus::SetupIncomplete => "setup_incomplete",
			GroupStatus::SpendingLimitReached => "spending_limit_reached",
			GroupStatus::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&[
			"active",
			"payment_failed",
			"setup_incomplete",
			"spending_limit_reached",
		]
	}
}
impl AsRef<str> for GroupStatus {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A region server tier.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionTier {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub tier_name_id: std::string::String,
	/// Together with the denominator, denotes the portion of the CPU a given server uses.
	pub rivet_cores_numerator: i32,
	/// Together with the numerator, denotes the portion of the CPU a given server uses.
	pub rivet_cores_denominator: i32,
	/// CPU frequency (MHz).
	pub cpu: i64,
	/// Allocated memory (MB).
	pub memory: i64,
	/// Allocated disk space (MB).
	pub disk: i64,
	/// Internet bandwidth (MB).
	pub bandwidth: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
	/// Stderrs tream from the given process.
	StdErr,
	/// Stdout stream from the given processs.
	StdOut,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for LogStream {
	fn from(s: &str) -> Self {
		match s {
			"std_err" => LogStream::StdErr,
			"std_out" => LogStream::StdOut,
			other => LogStream::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for LogStream {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(LogStream::from(s))
	}
}
impl LogStream {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			LogStream::StdErr => "std_err",
			LogStream::StdOut => "std_out",
			LogStream::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["std_err", "std_out"]
	}
}
impl AsRef<str> for LogStream {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

/// A file being prepared to upload.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadPrepareFile {
	/// The path/filename of the file.
	pub path: std::string::String,
	/// The MIME type of the file.
	pub content_type: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

/// A CDN site summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnSiteSummary {
	/// A universally unique identifier.
	pub site_id: std::string::String,
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
	/// Whether or not this site has completely been uploaded.
	pub complete: bool,
}

/// A build summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildSummary {
	/// A universally unique identifier.
	pub build_id: std::string::String,
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
	/// Whether or not this build has completely been uploaded.
	pub complete: bool,
}

/// Metrics relating to a job service.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SvcMetrics {
	/// The job name.
	pub job: std::string::String,
	/// CPU metrics.
	pub cpu: std::vec::Vec<f32>,
	/// Memory metrics.
	pub memory: std::vec::Vec<i64>,
	/// Peak memory metrics.
	pub memory_max: std::vec::Vec<i64>,
	/// Total allocated memory (MB).
	pub allocated_memory: i64,
}

/// A logs summary for a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogsLobbySummary {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub lobby_group_name_id: std::string::String,
	/// A universally unique identifier.
	pub region_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// RFC3339 timestamp.
	pub start_ts: std::option::Option<chrono::DateTime<chrono::Utc>>,
	/// RFC3339 timestamp.
	pub ready_ts: std::option::Option<chrono::DateTime<chrono::Utc>>,
	/// A union representing the state of a lobby.
	pub status: LogsLobbyStatus,
}

/// A union representing the state of a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogsLobbyStatus {
	/// A running lobby.
	Running(Unit),
	/// The status of a stopped lobby.
	Stopped(LogsLobbyStatusStopped),
}

/// The status of a stopped lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogsLobbyStatusStopped {
	/// RFC3339 timestamp.
	pub stop_ts: chrono::DateTime<chrono::Utc>,
	/// Whether or not the lobby failed or stopped successfully.
	pub failed: bool,
	/// The exit code returned by the lobby's main process when stopped.
	pub exit_code: i32,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unit {}

/// Analyical information about a lobby.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnalyticsLobbySummary {
	/// A universally unique identifier.
	pub lobby_id: std::string::String,
	/// A universally unique identifier.
	pub lobby_group_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub lobby_group_name_id: std::string::String,
	/// A universally unique identifier.
	pub region_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Whether or not this lobby is ready.
	pub is_ready: bool,
	/// Whether or not this lobby is idle.
	pub is_idle: bool,
	/// Whether or not this lobby is in a closed state.
	pub is_closed: bool,
	/// Whether or not this lobby is outdated.
	pub is_outdated: bool,
	/// Unsigned 32 bit integer.
	pub max_players_normal: i32,
	/// Unsigned 32 bit integer.
	pub max_players_direct: i32,
	/// Unsigned 32 bit integer.
	pub max_players_party: i32,
	/// Unsigned 32 bit integer.
	pub total_player_count: i32,
	/// Unsigned 32 bit integer.
	pub registered_player_count: i32,
}

/// A value denoting what type of authentication to use for a game namespace's CDN.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CdnAuthType {
	#[allow(missing_docs)] // documentation missing in model
	Basic,
	#[allow(missing_docs)] // documentation missing in model
	None,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for CdnAuthType {
	fn from(s: &str) -> Self {
		match s {
			"basic" => CdnAuthType::Basic,
			"none" => CdnAuthType::None,
			other => CdnAuthType::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for CdnAuthType {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(CdnAuthType::from(s))
	}
}
impl CdnAuthType {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			CdnAuthType::Basic => "basic",
			CdnAuthType::None => "none",
			CdnAuthType::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["basic", "none"]
	}
}
impl AsRef<str> for CdnAuthType {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A docker port.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyGroupRuntimeDockerPort {
	/// The label of this docker port.
	pub label: std::string::String,
	/// The port number to connect to.
	pub target_port: std::option::Option<i32>,
	/// The port range to connect to for UDP.
	pub port_range: std::option::Option<PortRange>,
	/// A proxy protocol.
	pub proxy_protocol: ProxyProtocol,
}

/// A proxy protocol.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
	#[allow(missing_docs)] // documentation missing in model
	Http,
	#[allow(missing_docs)] // documentation missing in model
	Https,
	#[allow(missing_docs)] // documentation missing in model
	Udp,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for ProxyProtocol {
	fn from(s: &str) -> Self {
		match s {
			"http" => ProxyProtocol::Http,
			"https" => ProxyProtocol::Https,
			"udp" => ProxyProtocol::Udp,
			other => ProxyProtocol::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for ProxyProtocol {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(ProxyProtocol::from(s))
	}
}
impl ProxyProtocol {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			ProxyProtocol::Http => "http",
			ProxyProtocol::Https => "https",
			ProxyProtocol::Udp => "udp",
			ProxyProtocol::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["http", "https", "udp"]
	}
}
impl AsRef<str> for ProxyProtocol {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Range of ports that can be connected to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortRange {
	/// Unsigned 32 bit integer.
	pub min: i32,
	/// Unsigned 32 bit integer.
	pub max: i32,
}

/// A full namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamespaceFull {
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub version_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Cloud configuration for a given namespace.
	pub config: CloudNamespaceConfig,
}

/// Cloud configuration for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CloudNamespaceConfig {
	/// CDN configuration for a given namespace.
	pub cdn: CdnNamespaceConfig,
	/// Matchmaker configuration for a given namespace.
	pub matchmaker: MatchmakerNamespaceConfig,
	/// KV configuration for a given namespace.
	pub kv: KvNamespaceConfig,
	/// Identity configuration for a given namespace.
	pub identity: IdentityNamespaceConfig,
}

/// Identity configuration for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityNamespaceConfig {}

/// KV configuration for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KvNamespaceConfig {}

/// Matchmaker configuration for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerNamespaceConfig {
	/// Unsigned 32 bit integer.
	pub lobby_count_max: i32,
	/// Unsigned 32 bit integer.
	pub max_players_per_client: i32,
	/// Unsigned 32 bit integer.
	pub max_players_per_client_vpn: i32,
	/// Unsigned 32 bit integer.
	pub max_players_per_client_proxy: i32,
	/// Unsigned 32 bit integer.
	pub max_players_per_client_tor: i32,
	/// Unsigned 32 bit integer.
	pub max_players_per_client_hosting: i32,
}

/// CDN configuration for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnNamespaceConfig {
	/// Whether or not to allow users to connect to the given namespace via domain name.
	pub enable_domain_public_auth: bool,
	/// A list of CDN domains for a given namespace.
	pub domains: std::vec::Vec<CdnNamespaceDomain>,
	/// A value denoting what type of authentication to use for a game namespace's CDN.
	pub auth_type: CdnAuthType,
	/// A list of CDN authenticated users for a given namespace.
	pub auth_user_list: std::vec::Vec<CdnNamespaceAuthUser>,
}

/// An authenticated CDN user for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnNamespaceAuthUser {
	/// A user name.
	pub user: std::string::String,
}

/// A CDN domain for a given namespace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnNamespaceDomain {
	/// A valid domain name (no protocol).
	pub domain: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A value denoting the status of a CDN domain's verification status.
	pub verification_status: CdnNamespaceDomainVerificationStatus,
	/// A union representing the verification method used for this CDN domain.
	pub verification_method: CdnNamespaceDomainVerificationMethod,
	#[allow(missing_docs)] // documentation missing in model
	pub verification_errors: std::vec::Vec<std::string::String>,
}

/// A union representing the verification method used for this CDN domain.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CdnNamespaceDomainVerificationMethod {
	/// CDN Namespace domain verification method HTTP variant.
	Http(CdnNamespaceDomainVerificationMethodHttp),
	/// CDN Namespace domain verification method variant denoting that this record is invalid.
	Invalid(CdnNamespaceDomainVerificationMethodInvalid),
}

/// CDN Namespace domain verification method HTTP variant.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnNamespaceDomainVerificationMethodHttp {
	/// The CNAME record this domain should point to.
	pub cname_record: std::string::String,
}

/// CDN Namespace domain verification method variant denoting that this record is invalid.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnNamespaceDomainVerificationMethodInvalid {}

/// A value denoting the status of a CDN domain's verification status.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CdnNamespaceDomainVerificationStatus {
	#[allow(missing_docs)] // documentation missing in model
	Active,
	#[allow(missing_docs)] // documentation missing in model
	Failed,
	#[allow(missing_docs)] // documentation missing in model
	Pending,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for CdnNamespaceDomainVerificationStatus {
	fn from(s: &str) -> Self {
		match s {
			"active" => CdnNamespaceDomainVerificationStatus::Active,
			"failed" => CdnNamespaceDomainVerificationStatus::Failed,
			"pending" => CdnNamespaceDomainVerificationStatus::Pending,
			other => CdnNamespaceDomainVerificationStatus::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for CdnNamespaceDomainVerificationStatus {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(CdnNamespaceDomainVerificationStatus::from(s))
	}
}
impl CdnNamespaceDomainVerificationStatus {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			CdnNamespaceDomainVerificationStatus::Active => "active",
			CdnNamespaceDomainVerificationStatus::Failed => "failed",
			CdnNamespaceDomainVerificationStatus::Pending => "pending",
			CdnNamespaceDomainVerificationStatus::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["active", "failed", "pending"]
	}
}
impl AsRef<str> for CdnNamespaceDomainVerificationStatus {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// Cloud configuration for a given version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CloudVersionConfig {
	/// CDN configuration for a given version.
	pub cdn: std::option::Option<CdnVersionConfig>,
	/// Matchmaker configuration for a given version.
	pub matchmaker: std::option::Option<MatchmakerVersionConfig>,
	/// KV configuration for a given version.
	pub kv: std::option::Option<KvVersionConfig>,
	/// Identity configuration for a given version.
	pub identity: std::option::Option<IdentityVersionConfig>,
}

/// Identity configuration for a given version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityVersionConfig {
	#[allow(missing_docs)] // documentation missing in model
	pub custom_display_names: std::vec::Vec<CustomDisplayName>,
	#[allow(missing_docs)] // documentation missing in model
	pub custom_avatars: std::vec::Vec<CustomAvatar>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomAvatar {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomDisplayName {
	#[allow(missing_docs)] // documentation missing in model
	pub display_name: std::string::String,
}

/// KV configuration for a given version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KvVersionConfig {}

/// Matchmaker configuration for a given version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerVersionConfig {
	/// A list of game modes.
	pub lobby_groups: std::option::Option<std::vec::Vec<LobbyGroup>>,
	/// Matchmaker captcha configuration.
	pub captcha: std::option::Option<MatchmakerCaptcha>,
}

/// Matchmaker captcha configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerCaptcha {
	/// Denotes how many requests a connection can make before it is required to reverify a captcha.
	pub requests_before_reverify: i32,
	/// Denotes how long a connection can continue to reconnect without having to reverify a captcha (in milliseconds).
	pub verification_ttl: i64,
	/// hCpatcha configuration.
	pub hcaptcha: std::option::Option<MatchmakerCaptchaHcaptcha>,
}

/// hCpatcha configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchmakerCaptchaHcaptcha {
	/// How hard a captcha should be.
	pub level: CaptchaLevel,
}

/// How hard a captcha should be.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaLevel {
	#[allow(missing_docs)] // documentation missing in model
	AlwaysOn,
	#[allow(missing_docs)] // documentation missing in model
	Difficult,
	#[allow(missing_docs)] // documentation missing in model
	Easy,
	#[allow(missing_docs)] // documentation missing in model
	Moderate,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for CaptchaLevel {
	fn from(s: &str) -> Self {
		match s {
			"always_on" => CaptchaLevel::AlwaysOn,
			"difficult" => CaptchaLevel::Difficult,
			"easy" => CaptchaLevel::Easy,
			"moderate" => CaptchaLevel::Moderate,
			other => CaptchaLevel::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for CaptchaLevel {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(CaptchaLevel::from(s))
	}
}
impl CaptchaLevel {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			CaptchaLevel::AlwaysOn => "always_on",
			CaptchaLevel::Difficult => "difficult",
			CaptchaLevel::Easy => "easy",
			CaptchaLevel::Moderate => "moderate",
			CaptchaLevel::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["always_on", "difficult", "easy", "moderate"]
	}
}
impl AsRef<str> for CaptchaLevel {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A game mode.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyGroup {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// A list of game mode regions.
	pub regions: std::vec::Vec<LobbyGroupRegion>,
	/// Unsigned 32 bit integer.
	pub max_players_normal: i32,
	/// Unsigned 32 bit integer.
	pub max_players_direct: i32,
	/// Unsigned 32 bit integer.
	pub max_players_party: i32,
	/// A union representing the runtime a game mode runs on.
	pub runtime: LobbyGroupRuntime,
}

/// A union representing the runtime a game mode runs on.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyGroupRuntime {
	/// A game mode runtime running through Docker.
	Docker(LobbyGroupRuntimeDocker),
}

/// A game mode runtime running through Docker.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyGroupRuntimeDocker {
	/// A universally unique identifier.
	pub build_id: std::option::Option<std::string::String>,
	/// A list of docker arguments.
	pub args: std::vec::Vec<std::string::String>,
	/// A list of docker environment variables.
	pub env_vars: std::vec::Vec<LobbyGroupRuntimeDockerEnvVar>,
	/// The network mode the job should run on.
	pub network_mode: std::option::Option<NetworkMode>,
	/// A list of docker ports.
	pub ports: std::vec::Vec<LobbyGroupRuntimeDockerPort>,
}

/// The network mode the job should run on.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
	#[allow(missing_docs)] // documentation missing in model
	Bridge,
	#[allow(missing_docs)] // documentation missing in model
	Host,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for NetworkMode {
	fn from(s: &str) -> Self {
		match s {
			"bridge" => NetworkMode::Bridge,
			"host" => NetworkMode::Host,
			other => NetworkMode::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for NetworkMode {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(NetworkMode::from(s))
	}
}
impl NetworkMode {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			NetworkMode::Bridge => "bridge",
			NetworkMode::Host => "host",
			NetworkMode::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["bridge", "host"]
	}
}
impl AsRef<str> for NetworkMode {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A docker environment variable.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyGroupRuntimeDockerEnvVar {
	/// The key of this environment variable.
	pub key: std::string::String,
	/// The value of this environment variable.
	pub value: std::string::String,
}

/// A game mode region.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyGroupRegion {
	/// A universally unique identifier.
	pub region_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub tier_name_id: std::string::String,
	/// Configuration for how many idle lobbies a game version should have.
	pub idle_lobbies: std::option::Option<IdleLobbiesConfig>,
}

/// Configuration for how many idle lobbies a game version should have.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdleLobbiesConfig {
	/// Unsigned 32 bit integer.
	pub min_idle_lobbies: i32,
	/// Unsigned 32 bit integer.
	pub max_idle_lobbies: i32,
}

/// CDN configuration for a given version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnVersionConfig {
	/// A universally unique identifier.
	pub site_id: std::option::Option<std::string::String>,
	/// Client-side configuration
	pub build_command: std::option::Option<std::string::String>,
	/// Client-side configuration
	pub build_output: std::option::Option<std::string::String>,
	/// Multiple CDN version routes.
	pub routes: std::option::Option<std::vec::Vec<CdnVersionRoute>>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnVersionRoute {
	#[allow(missing_docs)] // documentation missing in model
	pub glob: std::string::String,
	/// Unsigned 32 bit integer.
	pub priority: i32,
	/// Multiple CDN version middleware.
	pub middlewares: std::vec::Vec<CdnVersionMiddleware>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnVersionMiddleware {
	#[allow(missing_docs)] // documentation missing in model
	pub kind: CdnVersionMiddlewareKind,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CdnVersionMiddlewareKind {
	#[allow(missing_docs)] // documentation missing in model
	CustomHeaders(CdnVersionCustomHeadersMiddleware),
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnVersionCustomHeadersMiddleware {
	#[allow(missing_docs)] // documentation missing in model
	pub headers: std::vec::Vec<CdnVersionHeader>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CdnVersionHeader {
	#[allow(missing_docs)] // documentation missing in model
	pub name: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub value: std::string::String,
}

/// A full version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionFull {
	/// A universally unique identifier.
	pub version_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// Cloud configuration for a given version.
	pub config: CloudVersionConfig,
}

/// A billing plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBillingPlan {
	/// A value denoting a game's billing plan.
	pub code: GameBillingPlanCode,
	#[allow(missing_docs)] // documentation missing in model
	pub name: std::string::String,
	/// The interval a billing plan acts on.
	pub interval: BillingInterval,
	/// Signed 64 bit integer.
	pub amount: i64,
	#[allow(missing_docs)] // documentation missing in model
	pub currency: std::string::String,
}

/// The interval a billing plan acts on.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingInterval {
	#[allow(missing_docs)] // documentation missing in model
	Monthly,
	#[allow(missing_docs)] // documentation missing in model
	Yearly,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for BillingInterval {
	fn from(s: &str) -> Self {
		match s {
			"monthly" => BillingInterval::Monthly,
			"yearly" => BillingInterval::Yearly,
			other => BillingInterval::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for BillingInterval {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(BillingInterval::from(s))
	}
}
impl BillingInterval {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			BillingInterval::Monthly => "monthly",
			BillingInterval::Yearly => "yearly",
			BillingInterval::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["monthly", "yearly"]
	}
}
impl AsRef<str> for BillingInterval {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A value denoting a game's billing plan.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameBillingPlanCode {
	#[allow(missing_docs)] // documentation missing in model
	Enterprise,
	#[allow(missing_docs)] // documentation missing in model
	Free,
	#[allow(missing_docs)] // documentation missing in model
	GameHobbyMonthly,
	#[allow(missing_docs)] // documentation missing in model
	GameHobbyYearly,
	#[allow(missing_docs)] // documentation missing in model
	GameStudioMonthly,
	#[allow(missing_docs)] // documentation missing in model
	GameStudioYearly,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GameBillingPlanCode {
	fn from(s: &str) -> Self {
		match s {
			"enterprise" => GameBillingPlanCode::Enterprise,
			"free" => GameBillingPlanCode::Free,
			"game_hobby_monthly" => GameBillingPlanCode::GameHobbyMonthly,
			"game_hobby_yearly" => GameBillingPlanCode::GameHobbyYearly,
			"game_studio_monthly" => GameBillingPlanCode::GameStudioMonthly,
			"game_studio_yearly" => GameBillingPlanCode::GameStudioYearly,
			other => GameBillingPlanCode::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GameBillingPlanCode {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GameBillingPlanCode::from(s))
	}
}
impl GameBillingPlanCode {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GameBillingPlanCode::Enterprise => "enterprise",
			GameBillingPlanCode::Free => "free",
			GameBillingPlanCode::GameHobbyMonthly => "game_hobby_monthly",
			GameBillingPlanCode::GameHobbyYearly => "game_hobby_yearly",
			GameBillingPlanCode::GameStudioMonthly => "game_studio_monthly",
			GameBillingPlanCode::GameStudioYearly => "game_studio_yearly",
			GameBillingPlanCode::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&[
			"enterprise",
			"free",
			"game_hobby_monthly",
			"game_hobby_yearly",
			"game_studio_monthly",
			"game_studio_yearly",
		]
	}
}
impl AsRef<str> for GameBillingPlanCode {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// A region summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionSummary {
	/// A universally unique identifier.
	pub region_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub region_name_id: std::string::String,
	/// The server provider of this region.
	pub provider: std::string::String,
	/// A universal number given to this region.
	pub universal_region: i16,
	/// Represent a resource's readable display name.
	pub provider_display_name: std::string::String,
	/// Represent a resource's readable display name.
	pub region_display_name: std::string::String,
}

/// Region tier metrics.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionTierMetrics {
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
	/// A universally unique identifier.
	pub region_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub tier_name_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub lobby_group_name_id: std::string::String,
	/// How long a region tier has been active (in seconds).
	pub uptime: i64,
}

/// A namespace summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamespaceSummary {
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub version_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
}

/// A game handle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameHandle {
	/// A universally unique identifier.
	pub game_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL of this game's logo image.
	pub logo_url: std::option::Option<std::string::String>,
	/// The URL of this game's banner image.
	pub banner_url: std::option::Option<std::string::String>,
}

/// A full game.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameFull {
	/// A universally unique identifier.
	pub game_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub developer_group_id: std::string::String,
	/// Unsigned 32 bit integer.
	pub total_player_count: i32,
	/// The URL of this game's logo image.
	pub logo_url: std::option::Option<std::string::String>,
	/// The URL of this game's banner image.
	pub banner_url: std::option::Option<std::string::String>,
	/// A list of namespace summaries.
	pub namespaces: std::vec::Vec<NamespaceSummary>,
	/// A list of version summaries.
	pub versions: std::vec::Vec<VersionSummary>,
	/// A list of region summaries.
	pub available_regions: std::vec::Vec<RegionSummary>,
}

/// A version summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionSummary {
	/// A universally unique identifier.
	pub version_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
}

/// A group summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupSummary {
	/// A universally unique identifier.
	pub group_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// The URL of this group's avatar image.
	pub avatar_url: std::option::Option<std::string::String>,
	/// External links for this group.
	pub external: GroupExternalLinks,
	/// Whether or not this group is a developer.
	pub is_developer: bool,
	/// Detailed information about a profile.
	pub bio: std::string::String,
	/// Whether or not the current identity is a member of this group.
	pub is_current_identity_member: bool,
	/// The current publicity value for the given group.
	pub publicity: GroupPublicity,
	/// Unsigned 32 bit integer.
	pub member_count: i32,
	/// A universally unique identifier.
	pub owner_identity_id: std::string::String,
}

/// The current publicity value for the given group.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupPublicity {
	#[allow(missing_docs)] // documentation missing in model
	Closed,
	#[allow(missing_docs)] // documentation missing in model
	Open,
	/// Unknown contains new variants that have been added since this code was generated.
	Unknown(String),
}
impl std::convert::From<&str> for GroupPublicity {
	fn from(s: &str) -> Self {
		match s {
			"closed" => GroupPublicity::Closed,
			"open" => GroupPublicity::Open,
			other => GroupPublicity::Unknown(other.to_owned()),
		}
	}
}
impl std::str::FromStr for GroupPublicity {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(GroupPublicity::from(s))
	}
}
impl GroupPublicity {
	/// Returns the `&str` value of the enum member.
	pub fn as_str(&self) -> &str {
		match self {
			GroupPublicity::Closed => "closed",
			GroupPublicity::Open => "open",
			GroupPublicity::Unknown(s) => s.as_ref(),
		}
	}
	/// Returns all the `&str` values of the enum members.
	pub fn values() -> &'static [&'static str] {
		&["closed", "open"]
	}
}
impl AsRef<str> for GroupPublicity {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

/// External links for this group.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupExternalLinks {
	/// A link to this group's profile page.
	pub profile: std::string::String,
	/// A link to this group's chat page.
	pub chat: std::string::String,
}

/// A game summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameSummary {
	/// A universally unique identifier.
	pub game_id: std::string::String,
	/// RFC3339 timestamp.
	pub create_ts: chrono::DateTime<chrono::Utc>,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub developer_group_id: std::string::String,
	/// Unsigned 32 bit integer.
	pub total_player_count: i32,
	/// The URL of this game's logo image.
	pub logo_url: std::option::Option<std::string::String>,
	/// The URL of this game's banner image.
	pub banner_url: std::option::Option<std::string::String>,
}

/// The current authenticated agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthAgent {
	/// The current authenticated game cloud.
	GameCloud(AuthAgentGameCloud),
	/// The current authenticated identity.
	Identity(AuthAgentIdentity),
}

/// The current authenticated game cloud.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthAgentGameCloud {
	/// A universally unique identifier.
	pub game_id: std::string::String,
}

/// The current authenticated identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthAgentIdentity {
	/// A universally unique identifier.
	pub identity_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteCustomAvatarUploadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareCustomAvatarUploadRequest {
	/// The path/filename of the custom avatar.
	pub path: std::string::String,
	/// The MIME type of the custom avatar.
	pub mime: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameCustomAvatarsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRayPerfLogsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGroupRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupBillingCheckoutRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConvertGroupRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupInvoicesListRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupBillingRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRegionTiersRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExportLobbyLogsRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub stream: LogStream,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLobbyLogsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExportMatchmakerLobbyHistoryRequest {
	/// Unsigned 64 bit integer.
	pub query_start: i64,
	/// Unsigned 64 bit integer.
	pub query_end: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteMatchmakerLobbyRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameCdnSiteRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A list of files preparing to upload.
	pub files: std::vec::Vec<UploadPrepareFile>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameCdnSitesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameBuildRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A tag given to the game build.
	pub image_tag: std::string::String,
	/// A file being prepared to upload.
	pub image_file: UploadPrepareFile,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameBuildsRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateCloudTokenRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetNamespaceLobbyRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListNamespaceLobbiesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetNamespaceAnalyticsMatchmakerLiveRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetNamespaceCdnAuthTypeRequest {
	/// A value denoting what type of authentication to use for a game namespace's CDN.
	pub auth_type: CdnAuthType,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveNamespaceCdnAuthUserRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateNamespaceCdnAuthUserRequest {
	/// A user name.
	pub user: std::string::String,
	/// A bcrypt encrypted password. An error is returned if the given string is not properly encrypted.
	pub password: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceMatchmakerConfigRequest {
	/// Unsigned 32 bit integer.
	pub lobby_count_max: i32,
	/// Unsigned 32 bit integer.
	pub max_players: i32,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceTokenDevelopmentRequest {
	#[allow(missing_docs)] // documentation missing in model
	pub hostname: std::string::String,
	/// A list of docker ports.
	pub lobby_ports: std::vec::Vec<LobbyGroupRuntimeDockerPort>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGameNamespaceMatchmakerConfigRequest {
	/// Unsigned 32 bit integer.
	pub lobby_count_max: i32,
	/// Unsigned 32 bit integer.
	pub max_players: i32,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToggleNamespaceDomainPublicAuthRequest {
	/// Whether or not to enable authentication based on domain.
	pub enabled: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveNamespaceDomainRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNamespaceDomainRequest {
	/// A valid domain name (no protocol).
	pub domain: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceTokenDevelopmentRequest {
	/// The hostname used for the token.
	pub hostname: std::string::String,
	/// A list of docker ports.
	pub lobby_ports: std::vec::Vec<LobbyGroupRuntimeDockerPort>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceTokenPublicRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGameNamespaceVersionRequest {
	/// A universally unique identifier.
	pub version_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameNamespaceByIdRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub version_id: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameVersionRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// Cloud configuration for a given version.
	pub config: CloudVersionConfig,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameVersionByIdRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameVersionRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// Cloud configuration for a given version.
	pub config: CloudVersionConfig,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameBillingPlansRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetGameBillingPlanRequest {
	/// A value denoting a game's billing plan.
	pub plan: GameBillingPlanCode,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameBillingRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBannerUploadCompleteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBannerUploadPrepareRequest {
	/// The path/filename of the game banner.
	pub path: std::string::String,
	/// The MIME type of the game banner.
	pub mime: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameLogoUploadCompleteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameLogoUploadPrepareRequest {
	/// The path/filename of the game logo.
	pub path: std::string::String,
	/// The MIME type of the game logo.
	pub mime: std::option::Option<std::string::String>,
	/// Unsigned 64 bit integer.
	pub content_length: i64,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameRequest {
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameByIdRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameRequest {
	/// A human readable short identifier used to references resources. Different than a `rivet.common#Uuid` because this is intended to be human readable. Different than `rivet.common#DisplayName` because this should not include special characters and be short.
	pub name_id: std::string::String,
	/// Represent a resource's readable display name.
	pub display_name: std::string::String,
	/// A universally unique identifier.
	pub developer_group_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGamesRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteUploadRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InspectRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteCustomAvatarUploadResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrepareCustomAvatarUploadResponse {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameCustomAvatarsResponse {
	/// A list of custom avatar summaries.
	pub custom_avatars: std::vec::Vec<CustomAvatarSummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRayPerfLogsResponse {
	/// A list of service performance summaries.
	pub perf_lists: std::vec::Vec<SvcPerf>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGroupResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupBillingCheckoutResponse {
	/// The URL of the checkout session.
	pub url: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConvertGroupResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupInvoicesListResponse {
	/// A list of a group's billing invoices.
	pub invoices: std::vec::Vec<GroupBillingInvoice>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGroupBillingResponse {
	/// Signed 64 bit integer.
	pub usage: i64,
	/// The status of a developer group.
	pub status: GroupStatus,
	/// Whether or not the given group can actively host games.
	pub active: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRegionTiersResponse {
	/// A list of region server tiers.
	pub tiers: std::vec::Vec<RegionTier>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExportLobbyLogsResponse {
	/// The URL to a CSV file for the given lobby history.
	pub url: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLobbyLogsResponse {
	/// Sorted old to new.
	pub lines: std::vec::Vec<std::string::String>,
	/// Sorted old to new.
	pub timestamps: std::vec::Vec<chrono::DateTime<chrono::Utc>>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExportMatchmakerLobbyHistoryResponse {
	/// The URL to a CSV file for the given lobby history.
	pub url: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteMatchmakerLobbyResponse {
	/// Whether or not the lobby was successfully stopped.
	pub did_remove: bool,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameCdnSiteResponse {
	/// A universally unique identifier.
	pub site_id: std::string::String,
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub presigned_requests: std::vec::Vec<UploadPresignedRequest>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameCdnSitesResponse {
	/// A list of CDN site summaries.
	pub sites: std::vec::Vec<CdnSiteSummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameBuildResponse {
	/// A universally unique identifier.
	pub build_id: std::string::String,
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub image_presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListGameBuildsResponse {
	/// A list of build summaries.
	pub builds: std::vec::Vec<BuildSummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateCloudTokenResponse {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetNamespaceLobbyResponse {
	/// A logs summary for a lobby.
	pub lobby: LogsLobbySummary,
	/// Metrics relating to a job service.
	pub metrics: std::option::Option<SvcMetrics>,
	/// A list of URLs.
	pub stdout_presigned_urls: std::vec::Vec<std::string::String>,
	/// A list of URLs.
	pub stderr_presigned_urls: std::vec::Vec<std::string::String>,
	/// A list of service performance summaries.
	pub perf_lists: std::vec::Vec<SvcPerf>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListNamespaceLobbiesResponse {
	/// A list of lobby log summaries.
	pub lobbies: std::vec::Vec<LogsLobbySummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetNamespaceAnalyticsMatchmakerLiveResponse {
	/// A list of analytics lobby summaries.
	pub lobbies: std::vec::Vec<AnalyticsLobbySummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetNamespaceCdnAuthTypeResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveNamespaceCdnAuthUserResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateNamespaceCdnAuthUserResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceMatchmakerConfigResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceTokenDevelopmentResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameNamespaceResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGameNamespaceMatchmakerConfigResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToggleNamespaceDomainPublicAuthResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoveNamespaceDomainResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNamespaceDomainResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceTokenDevelopmentResponse {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceTokenPublicResponse {
	/// A JSON Web Token. Slightly modified to include a description prefix and use Protobufs of JSON.
	pub token: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGameNamespaceVersionResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameNamespaceByIdResponse {
	/// A full namespace.
	pub namespace: NamespaceFull,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameNamespaceResponse {
	/// A universally unique identifier.
	pub namespace_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameVersionResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameVersionByIdResponse {
	/// A full version.
	pub version: VersionFull,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameVersionResponse {
	/// A universally unique identifier.
	pub version_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameBillingPlansResponse {
	/// A list of billing plans.
	pub plans: std::vec::Vec<GameBillingPlan>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetGameBillingPlanResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameBillingResponse {
	/// A game handle.
	pub game: GameHandle,
	/// A list of namespace summaries.
	pub namespaces: std::vec::Vec<NamespaceSummary>,
	/// A list of multiple region tier metrics.
	pub metrics: std::vec::Vec<RegionTierMetrics>,
	/// The status of a developer group.
	pub group_status: GroupStatus,
	/// Whether or not the given game can actively host games.
	pub group_active: bool,
	/// A value denoting a game's billing plan.
	pub plan: GameBillingPlanCode,
	/// A list of region summaries.
	pub available_regions: std::vec::Vec<RegionSummary>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBannerUploadCompleteResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBannerUploadPrepareResponse {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameLogoUploadCompleteResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameLogoUploadPrepareResponse {
	/// A universally unique identifier.
	pub upload_id: std::string::String,
	/// A presigned request used to upload files. Upload your file to the given URL via a PUT request.
	pub presigned_request: UploadPresignedRequest,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValidateGameResponse {
	/// A list of validation errors.
	pub errors: std::vec::Vec<ValidationError>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGameByIdResponse {
	/// A full game.
	pub game: GameFull,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateGameResponse {
	/// A universally unique identifier.
	pub game_id: std::string::String,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetGamesResponse {
	/// A list of game summaries.
	pub games: std::vec::Vec<GameSummary>,
	/// A list of group summaries.
	pub groups: std::vec::Vec<GroupSummary>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompleteUploadResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InspectResponse {
	/// The current authenticated agent.
	pub agent: AuthAgent,
}

