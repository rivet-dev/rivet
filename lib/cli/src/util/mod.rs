pub mod wf;
pub mod format;
pub mod db;

pub fn now() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}
