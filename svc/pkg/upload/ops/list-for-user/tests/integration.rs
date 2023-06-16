use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_id = Uuid::new_v4();

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
		user_id: Some(user_id.into()),
	})
	.await
	.unwrap();
	let upload_id = upload_prepare_res.upload_id.unwrap();

	let res = op!([ctx] upload_list_for_user {
		user_ids: vec![user_id.into(), Uuid::new_v4().into()],
		anchor: None,
		limit: 3,
	})
	.await
	.unwrap();

	assert_eq!(2, res.users.len());
	let upload_ids = &res.users.first().unwrap().upload_ids;
	assert_eq!(1, upload_ids.len());
	let res_upload_id = res.users.first().unwrap().upload_ids.first().unwrap();

	assert_eq!(upload_id, *res_upload_id);
}
