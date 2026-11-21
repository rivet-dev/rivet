use rivetkit_client::{Client, ClientConfig, GetOrCreateOptions};

const TOKEN: &str = "secret-engine-token";

#[tokio::test]
async fn action_errors_do_not_quote_the_token() {
	// A port that was free a moment ago refuses the connection.
	let port = std::net::TcpListener::bind("127.0.0.1:0")
		.unwrap()
		.local_addr()
		.unwrap()
		.port();
	let client = Client::new(
		ClientConfig::new(format!("http://127.0.0.1:{port}"))
			.token(TOKEN)
			.disable_metadata_lookup(true),
	);
	let handle = client
		.get_or_create("counter", vec!["a".into()], GetOrCreateOptions::default())
		.unwrap();

	let error = handle.action("increment", vec![]).await.unwrap_err();

	let text = format!("{error:#} {error:?}");
	assert!(!text.contains(TOKEN), "error quotes the token: {text}");
}
