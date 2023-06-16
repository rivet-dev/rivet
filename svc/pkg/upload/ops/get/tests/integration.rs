use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	// Create an upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-build".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "upload.txt".into(),
				mime: Some("text/plain".into()),
				content_length: 123,
				..Default::default()
			},
		],
	})
	.await
	.unwrap();

	let upload_id = upload_prepare_res.upload_id.unwrap();

	let res = op!([ctx] upload_get {
		upload_ids: vec![upload_id, Uuid::new_v4().into()],
	})
	.await
	.unwrap();

	assert_eq!(1, res.uploads.len());
}
