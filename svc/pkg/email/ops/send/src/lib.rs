use std::convert::TryFrom;

use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

lazy_static::lazy_static! {
	static ref SENDGRID_KEY: String = std::env::var("SENDGRID_KEY").expect("no sendgrid key");
}

#[derive(Serialize, Deserialize)]
struct SendGridPersonalization {
	to: Option<Vec<SendGridAddress>>,
	cc: Option<Vec<SendGridAddress>>,
	bcc: Option<Vec<SendGridAddress>>,
	dynamic_template_data: serde_json::Value,
}

impl TryFrom<email::send::Message> for SendGridPersonalization {
	type Error = GlobalError;

	fn try_from(value: email::send::Message) -> GlobalResult<Self> {
		Ok(SendGridPersonalization {
			to: if value.to_addresses.is_empty() {
				None
			} else {
				Some(
					value
						.to_addresses
						.iter()
						.cloned()
						.map(SendGridAddress::try_from)
						.collect::<GlobalResult<Vec<_>>>()?,
				)
			},
			cc: if value.cc_addresses.is_empty() {
				None
			} else {
				Some(
					value
						.cc_addresses
						.iter()
						.cloned()
						.map(SendGridAddress::try_from)
						.collect::<GlobalResult<Vec<_>>>()?,
				)
			},
			bcc: if value.bcc_addresses.is_empty() {
				None
			} else {
				Some(
					value
						.bcc_addresses
						.iter()
						.cloned()
						.map(SendGridAddress::try_from)
						.collect::<GlobalResult<Vec<_>>>()?,
				)
			},
			dynamic_template_data: serde_json::from_str::<serde_json::Value>(
				&value.dynamic_template_data,
			)?,
		})
	}
}

#[derive(Serialize)]
struct SendGridAttachment {
	content: String,
	#[serde(rename = "type")]
	_type: String,
	filename: String,
	disposition: Option<String>,
	content_id: Option<String>,
}

impl From<email::send::Attachment> for SendGridAttachment {
	fn from(value: email::send::Attachment) -> Self {
		SendGridAttachment {
			content: base64::encode(value.content),
			_type: value.mime,
			filename: value.filename,
			disposition: value.disposition,
			content_id: value.content_id,
		}
	}
}

#[derive(Serialize, Deserialize)]
struct SendGridAddress {
	email: String,
	name: Option<String>,
}

impl TryFrom<email::send::Address> for SendGridAddress {
	type Error = GlobalError;

	fn try_from(value: email::send::Address) -> GlobalResult<Self> {
		internal_assert!(!value.email.is_empty(), "email is empty");
		Ok(SendGridAddress {
			email: value.email,
			name: if value.name.is_empty() {
				None
			} else {
				Some(value.name)
			},
		})
	}
}

#[operation(name = "email-send")]
async fn handle(
	ctx: OperationContext<email::send::Request>,
) -> GlobalResult<email::send::Response> {
	let from_address = internal_unwrap!(ctx.from_address);

	let client = reqwest::Client::new();
	let body = if ctx.attachments.is_empty() {
		json!({
			"from": SendGridAddress::try_from(from_address.clone())?,
			"template_id": ctx.template_id,
			"personalizations": ctx.messages
				.iter()
				.cloned()
				.map(SendGridPersonalization::try_from)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	} else {
		json!({
			"from": SendGridAddress::try_from(from_address.clone())?,
			"template_id": ctx.template_id,
			"personalizations": ctx.messages
				.iter()
				.cloned()
				.map(SendGridPersonalization::try_from)
				.collect::<GlobalResult<Vec<_>>>()?,
			"attachments": ctx.attachments
				.iter()
				.cloned()
				.map(SendGridAttachment::from)
				.collect::<Vec<_>>(),
		})
	};
	tracing::info!(body = %serde_json::to_string(&body).unwrap(), "body");
	let res = client
		.post("https://api.sendgrid.com/v3/mail/send")
		.header("Authorization", format!("Bearer {}", &*SENDGRID_KEY))
		.json(&body)
		.send()
		.await?;
	let status = res.status();
	if !status.is_success() {
		let text_res = res.text().await;
		tracing::error!(
			status = ?status,
			body = ?text_res,
			"send request failed"
		);
		internal_panic!("send request failed");
	}

	Ok(email::send::Response {})
}
