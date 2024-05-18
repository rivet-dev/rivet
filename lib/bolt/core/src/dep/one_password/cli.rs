use std::{path::Path, process::Command};

use tokio::fs;

use crate::utils::command_helper::CommandHelper;

pub async fn command(service_token: Option<&str>) -> Command {
	let mut cmd = Command::new("which");
	cmd.arg("op");

	let installed = cmd.exec_quiet(true, true).await.is_ok();
	assert!(installed, "1Password secret management is enabled in the namespace config but the 1Password CLI (`op`) is not installed");

	let mut cmd = Command::new("op");

	if let Some(service_token) = service_token {
		if std::env::var("OP_CONNECT_HOST").is_ok() || std::env::var("OP_CONNECT_TOKEN").is_ok() {
			eprintln!("WARNING: `OP_CONNECT_HOST` or `OP_CONNECT_TOKEN` are set which take precedence over `OP_SERVICE_ACCOUNT_TOKEN`.");
		}

		cmd.env("OP_SERVICE_ACCOUNT_TOKEN", service_token);
	}

	cmd
}

pub async fn login() {
	let mut cmd = command(None).await;
	cmd.arg("signin").arg("-f");
	cmd.exec().await.unwrap();
}

pub async fn read(service_token: Option<&str>, path: &str) -> String {
	let mut cmd = command(service_token).await;
	cmd.arg("read").arg(path);
	cmd.exec_string_with_stderr(true).await.unwrap()
}

pub async fn write(service_token: Option<&str>, op_path: &str, tmp_path: &Path, content: &str) {
	let mut split = (&op_path[5..]).split('/');
	let vault = split.next().expect("invalid one password path");
	let item = split.next().expect("invalid one password path");
	let field = split.next().expect("invalid one password path");

	// Read existing item json
	let mut cmd = command(service_token).await;
	cmd.arg("item")
		.arg("get")
		.arg("--vault")
		.arg(vault)
		.arg("--format")
		.arg("json")
		.arg(item);
	let reference_str = cmd.exec_string_with_stderr(true).await.unwrap();
	let mut reference_json = serde_json::from_str::<serde_json::Value>(&reference_str).unwrap();

	// Update item json
	let fields_json = reference_json["fields"].as_array_mut().unwrap();
	let field_json = fields_json
		.iter_mut()
		.find(|f| f["label"] == field)
		.unwrap_or_else(|| panic!("could not find field {field} in {item}"));
	field_json["value"] = content.into();

	// Save to file
	let mut tmp_path_dir = tmp_path.to_path_buf();
	tmp_path_dir.pop();
	fs::create_dir_all(&tmp_path_dir).await.unwrap();
	fs::write(&tmp_path, serde_json::to_string(&reference_json).unwrap())
		.await
		.unwrap();

	// Edit item with template
	let mut cmd = command(service_token).await;
	cmd.arg("item")
		.arg("edit")
		.arg("--vault")
		.arg(vault)
		.arg("--template")
		.arg(tmp_path.display().to_string())
		.arg(item);
	cmd.exec_with_stderr(true).await.unwrap();

	fs::remove_file(&tmp_path).await.unwrap();
}
