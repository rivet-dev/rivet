use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] email_send {
		from_address: Some(email::send::Address {
			email: "hello@rivet.gg".into(),
			..Default::default()
		}),
		messages: vec![
			email::send::Message {
				to_addresses: vec![email::send::Address {
					email: "test@rivet.gg".into(),
					name: "Test Contact".into(),
				}],
				cc_addresses: Vec::new(),
				bcc_addresses: Vec::new(),
				dynamic_template_data: r#"
					{
						"testValue": "Hello, world!"
					}
				"#.into(),
			},
		],
		attachments: Vec::new(),
		template_id: "d-433d0fd100a94a998331dafb70b771df".into(),
	})
	.await
	.unwrap();
}
