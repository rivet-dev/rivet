use global_error::prelude::*;
use jsonwebtoken::{Algorithm, DecodingKey};
use prost::Message;
use std::convert::TryFrom;

mod schema {
	pub use types::rivet::{backend, claims::*};
}

pub const ALGORITHM: Algorithm = Algorithm::EdDSA;

lazy_static::lazy_static! {
	static ref JWT_KEY_PUBLIC: Option<String> = std::env::var("RIVET_JWT_KEY_PUBLIC").ok();
}

pub mod ent {
	use global_error::prelude::*;
	use std::convert::{TryFrom, TryInto};
	use uuid::Uuid;

	use super::schema;

	#[derive(Clone, Debug)]
	pub struct Refresh {
		pub session_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::Refresh> for Refresh {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::Refresh) -> GlobalResult<Self> {
			Ok(Refresh {
				session_id: unwrap!(value.session_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct User {
		pub user_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::User> for User {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::User) -> GlobalResult<Self> {
			Ok(User {
				user_id: unwrap!(value.user_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct GameNamespacePublic {
		pub namespace_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::GameNamespacePublic> for GameNamespacePublic {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::GameNamespacePublic) -> GlobalResult<Self> {
			Ok(GameNamespacePublic {
				namespace_id: unwrap!(value.namespace_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct MatchmakerLobby {
		pub lobby_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::MatchmakerLobby> for MatchmakerLobby {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::MatchmakerLobby) -> GlobalResult<Self> {
			Ok(MatchmakerLobby {
				lobby_id: unwrap!(value.lobby_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct MatchmakerPlayer {
		pub player_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::MatchmakerPlayer> for MatchmakerPlayer {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::MatchmakerPlayer) -> GlobalResult<Self> {
			Ok(MatchmakerPlayer {
				player_id: unwrap!(value.player_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct JobRun {
		pub run_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::JobRun> for JobRun {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::JobRun) -> GlobalResult<Self> {
			Ok(JobRun {
				run_id: unwrap!(value.run_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct GameCloud {
		pub game_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::GameCloud> for GameCloud {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::GameCloud) -> GlobalResult<Self> {
			Ok(GameCloud {
				game_id: unwrap!(value.game_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct GameNamespaceDevelopment {
		pub namespace_id: Uuid,
		pub hostname: String,
		pub lobby_ports: Vec<DevelopmentPort>,
	}

	#[derive(Clone, Debug)]
	pub struct DevelopmentPort {
		pub label: String,
		pub target_port: Option<u32>,
		pub port_range: Option<DevelopmentPortRange>,
		pub proxy_protocol: DevelopmentProxyProtocol,
	}

	impl TryFrom<schema::backend::matchmaker::lobby_runtime::Port> for DevelopmentPort {
		type Error = GlobalError;

		fn try_from(value: schema::backend::matchmaker::lobby_runtime::Port) -> GlobalResult<Self> {
			Ok(DevelopmentPort {
				label: value.label,
				target_port: value.target_port,
				port_range: value.port_range.map(TryInto::try_into).transpose()?,
				proxy_protocol: unwrap!(
					schema::backend::matchmaker::lobby_runtime::ProxyProtocol::from_i32(
						value.proxy_protocol,
					)
				)
				.into(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct DevelopmentPortRange {
		pub min: u32,
		pub max: u32,
	}

	impl TryFrom<schema::backend::matchmaker::lobby_runtime::PortRange> for DevelopmentPortRange {
		type Error = GlobalError;

		fn try_from(
			value: schema::backend::matchmaker::lobby_runtime::PortRange,
		) -> GlobalResult<Self> {
			Ok(DevelopmentPortRange {
				min: value.min,
				max: value.max,
			})
		}
	}

	#[derive(Clone, Debug, PartialEq)]
	pub enum DevelopmentProxyProtocol {
		Http,
		Https,
		Tcp,
		TcpTls,
		Udp,
	}

	impl From<schema::backend::matchmaker::lobby_runtime::ProxyProtocol> for DevelopmentProxyProtocol {
		fn from(value: schema::backend::matchmaker::lobby_runtime::ProxyProtocol) -> Self {
			match value {
				schema::backend::matchmaker::lobby_runtime::ProxyProtocol::Http => {
					DevelopmentProxyProtocol::Http
				}
				schema::backend::matchmaker::lobby_runtime::ProxyProtocol::Https => {
					DevelopmentProxyProtocol::Https
				}
				schema::backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp => {
					DevelopmentProxyProtocol::Tcp
				}
				schema::backend::matchmaker::lobby_runtime::ProxyProtocol::TcpTls => {
					DevelopmentProxyProtocol::TcpTls
				}
				schema::backend::matchmaker::lobby_runtime::ProxyProtocol::Udp => {
					DevelopmentProxyProtocol::Udp
				}
			}
		}
	}

	impl TryFrom<&schema::entitlement::GameNamespaceDevelopment> for GameNamespaceDevelopment {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::GameNamespaceDevelopment) -> GlobalResult<Self> {
			Ok(GameNamespaceDevelopment {
				namespace_id: unwrap!(value.namespace_id).as_uuid(),
				hostname: value.hostname.to_owned(),
				lobby_ports: value
					.lobby_ports
					.clone()
					.into_iter()
					.map(TryInto::try_into)
					.collect::<GlobalResult<Vec<_>>>()?,
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct MatchmakerDevelopmentPlayer {
		pub namespace_id: Uuid,
		pub player_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::MatchmakerDevelopmentPlayer> for MatchmakerDevelopmentPlayer {
		type Error = GlobalError;

		fn try_from(
			value: &schema::entitlement::MatchmakerDevelopmentPlayer,
		) -> GlobalResult<Self> {
			Ok(MatchmakerDevelopmentPlayer {
				namespace_id: unwrap!(value.namespace_id).as_uuid(),
				player_id: unwrap!(value.player_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct GameUser {
		pub game_user_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::GameUser> for GameUser {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::GameUser) -> GlobalResult<Self> {
			Ok(GameUser {
				game_user_id: unwrap!(value.game_user_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct GameUserLink {
		pub link_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::GameUserLink> for GameUserLink {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::GameUserLink) -> GlobalResult<Self> {
			Ok(GameUserLink {
				link_id: unwrap!(value.link_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct UploadFile {
		pub upload_id: Uuid,
		pub path: String,
		pub content_length: u64,
	}

	impl TryFrom<&schema::entitlement::UploadFile> for UploadFile {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::UploadFile) -> GlobalResult<Self> {
			Ok(UploadFile {
				upload_id: unwrap!(value.upload_id).as_uuid(),
				path: value.path.clone(),
				content_length: value.content_length,
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct CloudDeviceLink {
		pub device_link_id: Uuid,
	}

	impl TryFrom<&schema::entitlement::CloudDeviceLink> for CloudDeviceLink {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::CloudDeviceLink) -> GlobalResult<Self> {
			Ok(CloudDeviceLink {
				device_link_id: unwrap!(value.device_link_id).as_uuid(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct Bypass {}

	impl TryFrom<&schema::entitlement::Bypass> for Bypass {
		type Error = GlobalError;

		fn try_from(_value: &schema::entitlement::Bypass) -> GlobalResult<Self> {
			Ok(Bypass {})
		}
	}

	#[derive(Clone, Debug)]
	pub struct AccessToken {
		pub name: String,
	}

	impl TryFrom<&schema::entitlement::AccessToken> for AccessToken {
		type Error = GlobalError;

		fn try_from(value: &schema::entitlement::AccessToken) -> GlobalResult<Self> {
			Ok(AccessToken {
				name: value.name.clone(),
			})
		}
	}

	#[derive(Clone, Debug)]
	pub struct ProvisionedServer {}

	impl TryFrom<&schema::entitlement::ProvisionedServer> for ProvisionedServer {
		type Error = GlobalError;

		fn try_from(_value: &schema::entitlement::ProvisionedServer) -> GlobalResult<Self> {
			Ok(ProvisionedServer {})
		}
	}
}

pub trait ClaimsDecode {
	fn as_refresh(&self) -> GlobalResult<ent::Refresh>;
	fn as_user(&self) -> GlobalResult<ent::User>;
	fn as_game_namespace_public(&self) -> GlobalResult<ent::GameNamespacePublic>;
	fn as_game_namespace_public_option(&self) -> GlobalResult<Option<ent::GameNamespacePublic>>;
	fn as_matchmaker_lobby(&self) -> GlobalResult<ent::MatchmakerLobby>;
	fn as_matchmaker_lobby_option(&self) -> GlobalResult<Option<ent::MatchmakerLobby>>;
	fn as_matchmaker_player(&self) -> GlobalResult<ent::MatchmakerPlayer>;
	fn as_job_run(&self) -> GlobalResult<ent::JobRun>;
	fn as_game_cloud(&self) -> GlobalResult<ent::GameCloud>;
	fn as_game_namespace_development_option(
		&self,
	) -> GlobalResult<Option<ent::GameNamespaceDevelopment>>;
	fn as_matchmaker_development_player(&self) -> GlobalResult<ent::MatchmakerDevelopmentPlayer>;
	fn as_game_user(&self) -> GlobalResult<ent::GameUser>;
	fn as_game_user_option(&self) -> GlobalResult<Option<ent::GameUser>>;
	fn as_game_user_link(&self) -> GlobalResult<ent::GameUserLink>;
	fn as_upload_file(&self) -> GlobalResult<ent::UploadFile>;
	fn as_cloud_device_link(&self) -> GlobalResult<ent::CloudDeviceLink>;
	fn as_bypass(&self) -> GlobalResult<ent::Bypass>;
	fn as_access_token(&self) -> GlobalResult<ent::AccessToken>;
	fn as_provisioned_server(&self) -> GlobalResult<ent::ProvisionedServer>;
}

impl ClaimsDecode for schema::Claims {
	fn as_refresh(&self) -> GlobalResult<ent::Refresh> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::Refresh(ent)) => Some(ent::Refresh::try_from(ent)),
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "Refresh"
			))
			.and_then(std::convert::identity)
	}

	fn as_user(&self) -> GlobalResult<ent::User> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::User(ent)) => Some(ent::User::try_from(ent)),
				_ => None,
			})
			.ok_or(err_code!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User"))
			.and_then(std::convert::identity)
	}

	fn as_game_namespace_public(&self) -> GlobalResult<ent::GameNamespacePublic> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameNamespacePublic(ent)) => {
					Some(ent::GameNamespacePublic::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "GameNamespacePublic"
			))
			.and_then(std::convert::identity)
	}

	fn as_game_namespace_public_option(&self) -> GlobalResult<Option<ent::GameNamespacePublic>> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameNamespacePublic(ent)) => {
					Some(ent::GameNamespacePublic::try_from(ent))
				}
				_ => None,
			})
			.transpose()
	}

	fn as_matchmaker_lobby(&self) -> GlobalResult<ent::MatchmakerLobby> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::MatchmakerLobby(ent)) => {
					Some(ent::MatchmakerLobby::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "MatchmakerLobby"
			))
			.and_then(std::convert::identity)
	}

	fn as_matchmaker_lobby_option(&self) -> GlobalResult<Option<ent::MatchmakerLobby>> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::MatchmakerLobby(ent)) => {
					Some(ent::MatchmakerLobby::try_from(ent))
				}
				_ => None,
			})
			.transpose()
	}

	fn as_matchmaker_player(&self) -> GlobalResult<ent::MatchmakerPlayer> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::MatchmakerPlayer(ent)) => {
					Some(ent::MatchmakerPlayer::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "MatchmakerPlayer"
			))
			.and_then(std::convert::identity)
	}

	fn as_job_run(&self) -> GlobalResult<ent::JobRun> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::JobRun(ent)) => Some(ent::JobRun::try_from(ent)),
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "JobRun"
			))
			.and_then(std::convert::identity)
	}

	fn as_game_cloud(&self) -> GlobalResult<ent::GameCloud> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameCloud(ent)) => {
					Some(ent::GameCloud::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "GameCloud"
			))
			.and_then(std::convert::identity)
	}

	fn as_game_namespace_development_option(
		&self,
	) -> GlobalResult<Option<ent::GameNamespaceDevelopment>> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameNamespaceDevelopment(ent)) => {
					Some(ent::GameNamespaceDevelopment::try_from(ent))
				}
				_ => None,
			})
			.transpose()
	}

	fn as_matchmaker_development_player(&self) -> GlobalResult<ent::MatchmakerDevelopmentPlayer> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::MatchmakerDevelopmentPlayer(ent)) => {
					Some(ent::MatchmakerDevelopmentPlayer::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "MatchmakerDevelopmentPlayer"
			))
			.and_then(std::convert::identity)
	}

	fn as_game_user(&self) -> GlobalResult<ent::GameUser> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameUser(ent)) => {
					Some(ent::GameUser::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "GameUser"
			))
			.and_then(std::convert::identity)
	}

	fn as_game_user_option(&self) -> GlobalResult<Option<ent::GameUser>> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameUser(ent)) => {
					Some(ent::GameUser::try_from(ent))
				}
				_ => None,
			})
			.transpose()
	}

	fn as_game_user_link(&self) -> GlobalResult<ent::GameUserLink> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::GameUserLink(ent)) => {
					Some(ent::GameUserLink::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "GameUserLink"
			))
			.and_then(std::convert::identity)
	}

	fn as_upload_file(&self) -> GlobalResult<ent::UploadFile> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::UploadFile(ent)) => {
					Some(ent::UploadFile::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "UploadFile"
			))
			.and_then(std::convert::identity)
	}

	fn as_cloud_device_link(&self) -> GlobalResult<ent::CloudDeviceLink> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::CloudDeviceLink(ent)) => {
					Some(ent::CloudDeviceLink::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "CloudDeviceLink"
			))
			.and_then(std::convert::identity)
	}

	fn as_bypass(&self) -> GlobalResult<ent::Bypass> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::Bypass(ent)) => Some(ent::Bypass::try_from(ent)),
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "Bypass"
			))
			.and_then(std::convert::identity)
	}

	fn as_provisioned_server(&self) -> GlobalResult<ent::ProvisionedServer> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::ProvisionedServer(ent)) => Some(ent::ProvisionedServer::try_from(ent)),
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlement = "ProvisionedServer"
			))
			.and_then(std::convert::identity)
	}

	fn as_access_token(&self) -> GlobalResult<ent::AccessToken> {
		self.entitlements
			.iter()
			.find_map(|ent| match &ent.kind {
				Some(schema::entitlement::Kind::AccessToken(ent)) => {
					Some(ent::AccessToken::try_from(ent))
				}
				_ => None,
			})
			.ok_or(err_code!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "AccessToken"
			))
			.and_then(std::convert::identity)
	}
}

pub trait EntitlementTag {
	fn tag(&self) -> Option<u8>;
}

impl EntitlementTag for schema::Entitlement {
	/// Returns a flag representing this entity kind's tag. Should match the
	/// tags in the `oneof kind` in the protobuf.
	fn tag(&self) -> Option<u8> {
		self.kind.as_ref().map(|x| match x {
			schema::entitlement::Kind::Refresh(_) => 1,
			schema::entitlement::Kind::User(_) => 2,
			schema::entitlement::Kind::GameNamespacePublic(_) => 3,
			schema::entitlement::Kind::MatchmakerLobby(_) => 4,
			schema::entitlement::Kind::MatchmakerPlayer(_) => 5,
			schema::entitlement::Kind::JobRun(_) => 6,
			schema::entitlement::Kind::GameCloud(_) => 7,
			schema::entitlement::Kind::GameNamespaceDevelopment(_) => 8,
			schema::entitlement::Kind::MatchmakerDevelopmentPlayer(_) => 9,
			schema::entitlement::Kind::GameUser(_) => 10,
			schema::entitlement::Kind::GameUserLink(_) => 11,
			schema::entitlement::Kind::UploadFile(_) => 12,
			schema::entitlement::Kind::CloudDeviceLink(_) => 14,
			schema::entitlement::Kind::Bypass(_) => 15,
			schema::entitlement::Kind::AccessToken(_) => 16,
			schema::entitlement::Kind::ProvisionedServer(_) => 17,
		})
	}
}

pub fn decode(token: &str) -> GlobalResult<GlobalResult<schema::Claims>> {
	let pem_public = unwrap!(
		JWT_KEY_PUBLIC.as_ref(),
		"missing env var `RIVET_JWT_KEY_PUBLIC`"
	);

	let mut validation = jsonwebtoken::Validation::default();
	validation.algorithms = vec![ALGORITHM];
	validation.validate_exp = false;
	validation.validate_nbf = false;

	decode_proto(
		token,
		&DecodingKey::from_ed_pem(pem_public.as_bytes())?,
		&validation,
	)
}

/// Modified from jsonwebtoken::decode with Protobuf instead.
fn decode_proto(
	token: &str,
	key: &jsonwebtoken::DecodingKey,
	validation: &jsonwebtoken::Validation,
) -> GlobalResult<GlobalResult<schema::Claims>> {
	// TODO:
	// for alg in &validation.algorithms {
	// 	if key.family != alg.family() {
	// 		bail_with!(TOKEN_INVALID, reason = "invalid algorithm");
	// 	}
	// }

	// Count the # of separators to determine if we need to remove the label
	let sep_count = token
		.chars()
		.fold(0, |acc, x| if x == '.' { acc + 1 } else { acc });
	let token = if sep_count == 3 {
		// Discard label
		let (_, token) = unwrap!(token.split_once('.'), "unreachable");
		token
	} else {
		token
	};

	// Split up parts
	let mut iter = token.rsplitn(2, '.');
	let (signature, message) = match (iter.next(), iter.next()) {
		(Some(signature), Some(message)) => (signature, message),
		_ => bail_with!(TOKEN_INVALID, reason = "invalid separator count"),
	};
	let mut iter = message.rsplit('.');
	let (claims, header) = match (iter.next(), iter.next(), iter.next()) {
		(Some(claims), Some(header), None) => (claims, {
			let decoded = base64::decode_config(header, base64::URL_SAFE_NO_PAD)?;
			let s = String::from_utf8(decoded)?;
			serde_json::from_str::<jsonwebtoken::Header>(&s)?
		}),
		_ => bail_with!(TOKEN_INVALID, reason = "invalid separator count"),
	};

	if !validation.algorithms.contains(&header.alg) {
		bail_with!(TOKEN_INVALID, reason = "invalid algorithm");
	}

	if !jsonwebtoken::crypto::verify(signature, message.as_bytes(), key, header.alg)? {
		bail_with!(TOKEN_INVALID, reason = "invalid signature");
	}

	let claims_buf = base64::decode_config(&claims, base64::URL_SAFE_NO_PAD)?;
	let claims = schema::Claims::decode(claims_buf.as_slice())?;

	Ok(validate(&claims).map(|_| claims))
}

fn validate(claims: &schema::Claims) -> GlobalResult<()> {
	let now = rivet_util::timestamp::now();
	let claims_exp = claims.exp.unwrap_or_default();

	// Validate claims
	ensure_with!(claims_exp == 0 || now <= claims_exp, TOKEN_EXPIRED);

	Ok(())
}
