// cargo-deps: ctrlc="3.4.2"

use std::{env, thread, time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}}};

fn main() {
	// Parse the first command-line argument as the array size
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		eprintln!("missing size arg");
		std::process::exit(1);
	}
	let size = args[1].parse::<usize>().expect("Error parsing size");

	// Allocate the array
	println!("allocating {size} bytes");
	let _array = vec![0u8; size];

	handle_ctrl_c();

	println!("exiting");
}

fn handle_ctrl_c() {
	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();

	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	// Wait for ctrl + c
	while running.load(Ordering::SeqCst) {
		thread::sleep(Duration::from_secs(1));
	}
}
