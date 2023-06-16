pub fn min(x: i64) -> i64 {
	secs(x * 60)
}

pub fn secs(x: i64) -> i64 {
	millis(x * 1_000)
}

pub fn millis(x: i64) -> i64 {
	x * 1_000_000
}
