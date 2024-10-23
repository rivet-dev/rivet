use std::path::PathBuf;

/// Name of the config directory holding the rivet config.
const CONFIG_DIR_NAME: &str = "rivet";

#[cfg(target_os = "linux")]
pub fn system_config_dir() -> PathBuf {
	PathBuf::from("/etc").join(CONFIG_DIR_NAME)
}

#[cfg(target_os = "macos")]
pub fn system_config_dir() -> PathBuf {
	PathBuf::from("/Library/Application Support").join(CONFIG_DIR_NAME)
}

#[cfg(target_os = "windows")]
pub fn system_config_dir() -> PathBuf {
	PathBuf::from("C:\\ProgramData").join(CONFIG_DIR_NAME)
}

// This will cause a compile-time error if an unsupported OS is targeted.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn system_config_dir() -> PathBuf {
	compile_error!("Unsupported OS");
}
