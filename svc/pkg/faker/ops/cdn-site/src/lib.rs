use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-cdn-site")]
async fn handle(
	ctx: OperationContext<faker::cdn_site::Request>,
) -> GlobalResult<faker::cdn_site::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();

	let index_html = format!("Hello, {}!", Uuid::new_v4());
	let hello_index_html = format!("/hello/index.html {}", Uuid::new_v4());
	let hello_test_html = format!("/hello/test.html {}", Uuid::new_v4());
	let hello_world_txt = format!("/hello/world.txt {}", Uuid::new_v4());

	let files = vec![
		("index.html", &index_html, "text/html"),
		("hello/index.html", &hello_index_html, "text/html"),
		("hello/test.html", &hello_test_html, "text/html"),
		("hello/world.txt", &hello_world_txt, "text/plain"),
	];

	// Create site
	tracing::info!("creating site");
	let create_res = op!([ctx] cdn_site_create {
		game_id: Some(game_id.into()),
		display_name: util::faker::display_name(),
		files: files
			.iter()
			.map(|(path, body, mime)| backend::upload::PrepareFile {
				path: path.to_string(),
				mime: Some(mime.to_string()),
				content_length: body.len() as u64,
				..Default::default()
			})
			.collect(),
	})
	.await?;
	let site_id = internal_unwrap!(create_res.site_id).as_uuid();

	for (path, body, mime) in &files {
		let presigned_req = internal_unwrap_owned!(create_res
			.presigned_requests
			.iter()
			.find(|x| x.path == *path));

		// Upload files
		let url = &presigned_req.url;
		tracing::info!(?url, len = %(body.as_bytes().len() / 1024), "uploading file");
		let res = reqwest::Client::new()
			.put(url)
			.header(reqwest::header::CONTENT_TYPE, *mime)
			.header(reqwest::header::CONTENT_LENGTH, body.len())
			.body(body.as_bytes().to_vec())
			.send()
			.await?;
		let res_status = res.status();
		let res_text = res.text().await;
		tracing::info!(status = %res_status, body = ?res_text, "upload response");
		internal_assert!(res_status.is_success(), "failed to upload site");
	}

	// Complete the upload
	op!([ctx] upload_complete {
		upload_id: create_res.upload_id,
		bucket: Some("bucket-cdn".into()),
	})
	.await?;

	Ok(faker::cdn_site::Response {
		site_id: Some(site_id.into()),
		index_html,
		hello_index_html,
		hello_test_html,
		hello_world_txt,
	})
}
