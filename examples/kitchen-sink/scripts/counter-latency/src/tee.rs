// Mirror every stdout line written via `out!` to a per-run /tmp/counter-latency-<id>.txt
// transcript. Initialized once at startup; all log helpers route through `out!`.

use std::fs::File;
use std::io::{Write, stdout};
use std::sync::{Mutex, OnceLock};

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static LOG_FILE_PATH: OnceLock<String> = OnceLock::new();

pub fn init(id: &str) -> std::io::Result<String> {
	let path = format!("/tmp/counter-latency-{}.txt", id);
	let file = File::create(&path)?;
	LOG_FILE
		.set(Mutex::new(file))
		.map_err(|_| std::io::Error::new(std::io::ErrorKind::AlreadyExists, "log file already initialized"))?;
	LOG_FILE_PATH
		.set(path.clone())
		.map_err(|_| std::io::Error::new(std::io::ErrorKind::AlreadyExists, "log path already set"))?;
	Ok(path)
}

pub fn log_file_path() -> Option<&'static str> {
	LOG_FILE_PATH.get().map(|s| s.as_str())
}

pub fn emit(line: &str) {
	{
		let mut out = stdout().lock();
		let _ = writeln!(out, "{}", line);
	}
	if let Some(file_mu) = LOG_FILE.get() {
		if let Ok(mut f) = file_mu.lock() {
			let _ = writeln!(f, "{}", line);
		}
	}
}

#[macro_export]
macro_rules! out {
	() => {
		$crate::tee::emit("");
	};
	($($arg:tt)*) => {
		$crate::tee::emit(&format!($($arg)*));
	};
}
