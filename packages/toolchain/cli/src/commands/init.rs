use anyhow::*;
use clap::Parser;
use inquire::validator::Validation;
use serde::Serialize;
use std::{fmt, result::Result as StdResult};
use tokio::fs;
use toolchain::errors;

/// Initiate a new project
#[derive(Parser, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opts {}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		// Check if project already exists
		if let Result::Ok(path) = toolchain::config::Config::config_path(None).await {
			return Err(errors::UserError::new(format!(
				"Rivet config already exists at {}",
				path.display()
			))
			.into());
		}

		// Prompt init settings
		let prompt = tokio::task::spawn_blocking(|| {
			let project_name = inquire::Text::new("What is your project name?")
				.with_default("my-app")
				.with_validator(|input: &str| {
					let is_valid = input
						.chars()
						.all(|c| c.is_alphanumeric() || c == '-' || c == '_');
					if is_valid {
						StdResult::Ok(Validation::Valid)
					} else {
						StdResult::Ok(Validation::Invalid(
							"Project name must be alphanumeric and can include '-' or '_'".into(),
						))
					}
				})
				.prompt()?;

			let lang = inquire::Select::new(
				"What language will you write your Rivet Actor in?",
				vec![Language::TypeScript, Language::JavaScript, Language::Docker],
			)
			.with_starting_cursor(0)
			.with_help_message(
				"This can be changed later. Multiple languages can be used in the same project.",
			)
			.prompt()?;

			let config_format = inquire::Select::new(
				"What config format do you prefer?",
				vec![ConfigFormat::Json, ConfigFormat::Jsonc],
			)
			.with_starting_cursor(0)
			.prompt()?;

			let login = inquire::Confirm::new("Would you like to log in to Rivet now?")
				.with_default(true)
				.with_help_message(
					"This is required to deploy. You can run this later with `rivet login`.",
				)
				.prompt()?;

			let api_endpoint = if login {
				crate::util::login::inquire_self_hosting()?
			} else {
				None
			};

			Ok(PromptOutput {
				project_name,
				lang,
				config_format,
				login,
				api_endpoint,
			})
		})
		.await??;

		let project_path = std::env::current_dir()?.join(&prompt.project_name);

		println!();
		println!("Creating new Rivet project at {}", project_path.display());

		fs::create_dir(&project_path)
			.await
			.map_err(|err| anyhow!("failed to create project dir: {err}"))?;

		// Change dir so all subsequent tasks operate in this project
		std::env::set_current_dir(&project_path).context("failed to change dir")?;

		// Generate config
		let config = match prompt.lang {
			Language::TypeScript | Language::JavaScript => {
				// Write Deno config
				let deno_config = include_str!("../../static/init/js/deno.json");
				let deno_config_name = match prompt.config_format {
					ConfigFormat::Json => "deno.json",
					ConfigFormat::Jsonc => "deno.jsonc",
				};
				fs::write(project_path.join(deno_config_name), deno_config).await?;

				// Write script
				let (config_body, readme_body, script_name, script_body, test_name, test_body) =
					match prompt.lang {
						Language::TypeScript => (
							include_str!("../../../../../examples/init-template-ts/rivet.json"),
							include_str!("../../../../../examples/init-template-ts/README.md"),
							"counter.ts",
							include_str!("../../../../../examples/init-template-ts/counter.ts"),
							"counter_test.ts",
							include_str!(
								"../../../../../examples/init-template-ts/counter_test.ts"
							),
						),
						Language::JavaScript => (
							include_str!("../../../../../examples/init-template-js/rivet.json"),
							include_str!("../../../../../examples/init-template-js/README.md"),
							"counter.js",
							include_str!("../../../../../examples/init-template-js/counter.js"),
							"counter_test.js",
							include_str!(
								"../../../../../examples/init-template-js/counter_test.js"
							),
						),
						_ => unreachable!(),
					};
				fs::write(project_path.join(script_name), script_body).await?;
				fs::write(project_path.join(test_name), test_body).await?;

				let readme_body = readme_body.replace("__NAME__", &prompt.project_name);
				fs::write(project_path.join("README.md"), readme_body).await?;

				config_body.to_string()
			}
			Language::Docker => {
				let readme_body =
					include_str!("../../../../../examples/init-template-docker/README.md");
				let readme_body = readme_body.replace("__NAME__", &prompt.project_name);
				fs::write(project_path.join("README.md"), readme_body).await?;

				let dockerfile_body =
					include_str!("../../../../../examples/init-template-docker/Dockerfile");
				fs::write(project_path.join("Dockerfile"), dockerfile_body).await?;

				let config_body =
					include_str!("../../../../../examples/init-template-docker/rivet.json");

				config_body.to_string()
			}
		};

		// Create JSON config
		let config_name = match prompt.config_format {
			ConfigFormat::Json => "rivet.json",
			ConfigFormat::Jsonc => "rivet.jsonc",
		};
		fs::write(project_path.join(config_name), config).await?;

		println!("Project created successfully.");

		// Login to Rivet
		if prompt.login {
			println!();
			println!("Login in to Rivet...");
			crate::util::login::login(prompt.api_endpoint.clone()).await?;
		}

		println!();
		println!();
		println!("    ==========   Welcome to Rivet!   ==========");
		println!();
		println!("Resources:");
		println!();
		println!("  Documentation:      https://rivet.gg/docs");
		//println!("  Examples:         https://rivet.gg/docs/examples");
		println!("  Discord:            https://rivet.gg/discord");
		println!("  Issues:             https://github.com/rivet-gg/rivet/issues");
		println!("  Questions & Ideas:  https://github.com/orgs/rivet-gg/discussions");
		println!("  Configure IDE:      https://docs.deno.com/runtime/getting_started/setup_your_environment");
		//println!("  Enterprise:       https://rivet.gg/sales");
		println!();
		println!("Next steps:");
		println!();
		println!("  $ cd {}", prompt.project_name);
		println!("  $ rivet deploy");
		println!();

		crate::util::telemetry::capture_event(
			"cli_init",
			Some(|event: &mut async_posthog::Event| {
				event.insert_prop(
					"$set",
					serde_json::json!({
						"cli_init_prompt": prompt,
					}),
				)?;
				Ok(())
			}),
		)
		.await;

		Ok(())
	}
}

#[derive(Serialize)]
struct PromptOutput {
	project_name: String,
	lang: Language,
	config_format: ConfigFormat,
	login: bool,
	api_endpoint: Option<String>,
}

#[derive(Serialize)]
enum Language {
	TypeScript,
	JavaScript,
	Docker,
}

impl fmt::Display for Language {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output = match self {
			Language::TypeScript => "TypeScript",
			Language::JavaScript => "JavaScript",
			Language::Docker => "Other (Docker)",
		};
		write!(f, "{}", output)
	}
}

#[derive(Serialize)]
enum ConfigFormat {
	Json,
	Jsonc,
}

impl fmt::Display for ConfigFormat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output = match self {
			ConfigFormat::Json => "JSON (rivet.json)",
			ConfigFormat::Jsonc => "JSON with comments (rivet.jsonc)",
			// ConfigFormat::Json => "rivet.json (vanilla JSON)",
			// ConfigFormat::Jsonc => "rivet.jsonc (JSON with comments)",
		};
		write!(f, "{}", output)
	}
}
