use tiny_http::{Response, Server, StatusCode};

// TODO: This can't pick up SIGTERM
fn main() {
	let port = std::env::var("PORT")
		.ok()
		.unwrap_or_else(|| "8080".to_string());
	let addr = format!("0.0.0.0:{port}");
	let server = Server::http(&addr).unwrap();
	println!("Listening on {addr}");

	for mut request in server.incoming_requests() {
		let mut content = Vec::new();
		request.as_reader().read_to_end(&mut content).unwrap();

		let response = Response::new(
			StatusCode(200),
			Vec::new(),
			std::io::Cursor::new(content),
			request.body_length(),
			None,
		);
		request.respond(response).unwrap();
	}
}
