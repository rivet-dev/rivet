pub const fn bytes(v: u64) -> u64 {
	v
}

pub const fn kilobytes(v: u64) -> u64 {
	v * 1000
}

pub const fn megabytes(v: u64) -> u64 {
	v * 1000 * 1000
}

pub const fn gigabytes(v: u64) -> u64 {
	v * 1000 * 1000 * 1000
}

pub const fn kibibytes(v: u64) -> u64 {
	v * 1024
}

pub const fn mebibytes(v: u64) -> u64 {
	v * 1024 * 1024
}

pub const fn gibibytes(v: u64) -> u64 {
	v * 1024 * 1024 * 1024
}
