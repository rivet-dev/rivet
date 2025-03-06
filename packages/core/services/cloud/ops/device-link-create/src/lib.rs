use proto::backend::pkg::*;
use rivet_operation::prelude::*;

pub const TOKEN_TTL: i64 = util::duration::hours(1);

#[operation(name = "cloud-device-link-create")]
async fn handle(
	ctx: OperationContext<cloud::device_link_create::Request>,
) -> GlobalResult<cloud::device_link_create::Response> {
	let link_id = Uuid::new_v4();

	let token_res = op!([ctx] token_create {
		issuer: Self::NAME.into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: TOKEN_TTL,
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::CloudDeviceLink(proto::claims::entitlement::CloudDeviceLink {
							device_link_id: Some(link_id.into()),
						})
					)
				}
			],
		})),
		label: Some("device".into()),
		..Default::default()
	})
	.await?;
	let token = unwrap!(token_res.token);

	Ok(cloud::device_link_create::Response {
		device_link_id: Some(link_id.into()),
		token: token.token,
	})
}
