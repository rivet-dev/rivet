pub fn is_linux() -> bool {
	std::env::consts::OS == "linux"
}

#[cfg(target_os = "linux")]
pub fn is_root() -> bool {
	nix::unistd::Uid::current().is_root()
}

#[cfg(not(target_os = "linux"))]
pub fn is_root() -> bool {
	false
}

/// There are a lot of edge cases for Linux root that we need to frequently handle.
pub fn is_linux_and_root() -> bool {
	is_linux() && is_root()
}
