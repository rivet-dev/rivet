use crate::{context::ProjectContext, dep, tasks};

pub async fn generate_project(ctx: &ProjectContext, skip_config_sync_check: bool) {
	// println!("\n> Generating project");

	// HACK: Speed up bolt commands by skipping the generate step
	if std::env::var("BOLT_SKIP_GEN")
		.ok()
		.map_or(false, |x| x == "1")
	{
		rivet_term::status::info("Skipping generate_project", "");
		return;
	}

	// Check config and secrets are synced
	if !skip_config_sync_check {
		tasks::check::check_config_sync(ctx).await;
	}

	if !std::env::var("BOLT_IGNORE_TERRAFORM")
		.ok()
		.map_or(false, |x| x == "1")
	{
		// Generate Terraform variables
		dep::terraform::gen::project(ctx).await;
	}

	// Generate K8S configs
	dep::k8s::gen::project(ctx).await.unwrap();
}

// TODO: Add back
// async fn set_license(path: &Path) {
// 	let toml = fs::read_to_string(path)
// 		.await
// 		.unwrap_or_else(|_| panic!("could not read path: {}", path.display()));
// 	let mut doc = toml.parse::<toml_edit::Document>().unwrap();
//
// 	let mut array = toml_edit::Array::new();
// 	array.push("Rivet Gaming, LLC <developer@rivet.gg>");
// 	doc["package"]["authors"] = value(array);
//
// 	doc["package"]["license"] = value("Apache-2.0");
//
// 	write_if_different(path, &doc.to_string()).await;
// }

// /// Writes to a file if the contents are different.
// ///
// /// This prevents needlessly updating the modify timestamp of a Cargo manifest, which triggers a
// /// rebuild.
// async fn write_if_different(path: &Path, new_content: &str) {
// 	let current_content = fs::read_to_string(path).await.ok().unwrap_or_default();
//
// 	if current_content != new_content {
// 		fs::write(path, new_content).await.unwrap();
// 	}
// }
