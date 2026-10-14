use super::*;
#[tokio::test]
async fn raw_body_is_preserved_and_content_encoding_is_rejected() {
	let body = vec![0, 255, b'\r', b'\n', b'{', b'}'];
	let request = Request::builder()
		.header("content-type", "application/octet-stream")
		.body(axum::body::Body::from(body.clone()))
		.unwrap();
	let (actual, permit) = read_body(request).await.unwrap();
	assert_eq!(actual, body);
	drop(permit);
	let request = Request::builder()
		.header("content-type", "application/octet-stream")
		.header("content-encoding", "gzip")
		.body(axum::body::Body::empty())
		.unwrap();
	assert!(read_body(request).await.is_err());
}
