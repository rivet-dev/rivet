use reqwest::Client;
use tokio::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::const_new();
static CLIENT_NO_TIMEOUT: OnceCell<Client> = OnceCell::const_new();

pub async fn client() -> Result<Client, reqwest::Error> {
	CLIENT
		.get_or_try_init(|| async {
			Client::builder()
				.timeout(std::time::Duration::from_secs(30))
				.build()
		})
		.await
		.cloned()
}

pub async fn client_no_timeout() -> Result<Client, reqwest::Error> {
	CLIENT_NO_TIMEOUT
		.get_or_try_init(|| async { Client::builder().build() })
		.await
		.cloned()
}
