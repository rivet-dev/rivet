use std::{
	io::{BufRead, BufReader},
	net::{TcpListener, TcpStream},
	sync::mpsc::Sender,
};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct VectorMessage {
	pub source: String,
	pub run_id: String,
	pub task: String,
	pub stream_type: u8,
	pub ts: u64,
	pub message: String,
}

fn handle_client(stream: TcpStream, msg_tx: Sender<VectorMessage>) {
	let reader = BufReader::new(stream);
	for line in reader.lines() {
		println!("Received line: {line:?}");
		match line {
			Ok(line) => {
				let json = serde_json::from_str::<VectorMessage>(&line).unwrap();
				let _ = msg_tx.send(json);
			}
			Err(e) => {
				eprintln!("Error reading line: {}", e);
				break;
			}
		}
	}

	println!("Client exited")
}

pub fn listener(port: u16, msg_tx: Sender<VectorMessage>) -> std::io::Result<()> {
	let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;

	// Handle only one client. We don't need to create a listener loop.
	let stream = listener.incoming().next().unwrap().unwrap();

	// Handle client
	handle_client(stream, msg_tx.clone());

	Ok(())
}
