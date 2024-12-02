use anyhow::*;
use std::path::PathBuf;

pub const DENO_BINARY: &[u8] = include_bytes!(env!("DENO_BINARY_PATH"));
pub const DENO_VERSION: &str = env!("DENO_VERSION");

pub struct DenoExecutable {
	pub executable_path: PathBuf,
}

/// Writes the executable to the file system if needed and returns the path.
pub async fn get_executable(data_dir: &PathBuf) -> Result<DenoExecutable> {
	let executable_name = if cfg!(windows) { "deno.exe" } else { "deno" };
	let executable_path = data_dir
		.join("deno")
		.join(DENO_VERSION)
		.join(executable_name);

	if tokio::fs::metadata(&executable_path).await.is_ok() {
		return Ok(DenoExecutable { executable_path });
	}

	// Ensure the parent directory exists
	if let Some(parent) = executable_path.parent() {
		tokio::fs::create_dir_all(parent).await?;
	}

	// Write the binary to the executable path
	tokio::fs::write(&executable_path, DENO_BINARY).await?;

	// Make the file executable on Unix-like systems
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = tokio::fs::metadata(&executable_path).await?.permissions();
		perms.set_mode(0o755);
		tokio::fs::set_permissions(&executable_path, perms).await?;
	}

	Ok(DenoExecutable { executable_path })
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::TempDir;
	use tokio;

	#[tokio::test]
	async fn test_deno_get_or_download_executable() -> Result<()> {
		// Create a temporary directory for the test
		let temp_dir = TempDir::new()?;
		let data_dir = temp_dir.path().to_path_buf();

		// First call: should download the executable
		let result1 = get_executable(&data_dir).await?;
		assert!(
			result1.executable_path.exists(),
			"Executable should be created"
		);

		// Check if the file is executable on Unix-like systems
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let metadata = tokio::fs::metadata(&result1.executable_path).await?;
			let permissions = metadata.permissions();
			assert_eq!(
				permissions.mode() & 0o777,
				0o755,
				"Executable should have correct permissions"
			);
		}

		// Second call: should return the existing executable
		let result2 = get_executable(&data_dir).await?;
		assert_eq!(
			result1.executable_path, result2.executable_path,
			"Should return the same executable path"
		);

		// Run deno --version and check the output
		let output = std::process::Command::new(&result1.executable_path)
			.arg("--version")
			.output()?;

		let version_output =
			String::from_utf8(output.stdout).context("parse deno version output")?;
		assert!(
			version_output.contains(DENO_VERSION),
			"Deno version output should contain the expected version"
		);

		Ok(())
	}
}
