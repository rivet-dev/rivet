use chirp_worker::prelude::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let res = op!([ctx] nsfw_image_score {
		image_urls: vec![
			"https://unsplash.com/photos/RqO_02KT36w/download?ixid=MnwxMjA3fDB8MXxzZWFyY2h8M3x8d29tYW4lMjBwbGF5aW5nJTIwdmlkZW8lMjBnYW1lc3xlbnwwfHx8fDE2NjkwNTYwMjM&force=true&w=640".into(),
			"https://unsplash.com/photos/4aJ9GCwB3Gw/download?ixid=MnwxMjA3fDB8MXxhbGx8fHx8fHx8fHwxNjY5MDU2MTEw&force=true&w=640".into(),
		],
	})
	.await
	.unwrap();

	assert_eq!(2, res.scores.len());
}
