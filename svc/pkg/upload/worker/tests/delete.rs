use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

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

	let request_id = Uuid::new_v4();
	msg!([ctx] upload::msg::delete(request_id) -> upload::msg::delete_complete {
		request_id: Some(request_id.into()),
		upload_ids: vec![upload_id, Uuid::new_v4().into()],
	})
	.await
	.unwrap();

	let res = op!([ctx] upload_get {
		upload_ids: vec![upload_id],
	})
	.await
	.unwrap();
	let upload = res.uploads.first().unwrap();

	assert!(upload.deleted_ts.is_some());
}
