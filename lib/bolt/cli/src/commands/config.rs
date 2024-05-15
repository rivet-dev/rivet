use anyhow::*;
use bolt_core::{
	context::{self, ProjectContextData, RunContext},
	dep, tasks,
};
use clap::Parser;
use rivet_term::console::style;
use tokio::fs;

#[derive(Parser)]
pub enum SubCommand {
	/// Generates the namespace and secret config.
	///
	/// This can be ran multiple times in case parameters get changed.
	#[clap(alias = "gen")]
	Generate {
		#[clap(index = 1)]
		namespace: String,
	},
	/// Sets the selected namespace in `Bolt.local.toml`.
	#[clap(alias = "set-ns")]
	SetNamespace {
		#[clap(index = 1)]
		namespace: String,
	},
	ServiceDependencies {
		#[clap(index = 1)]
		svc_name: String,
		#[clap(long, short = 'r')]
		recursive: bool,
		#[clap(long)]
		test: bool,
	},
	Show,
	SourceHash,
	/// Pull namespace config and secrets from 1Password.
	Pull {
		#[clap(index = 1)]
		namespace: Option<String>,
		#[clap(long, short = 'y')]
		yes: bool,
		#[clap(long, env = "OP_SERVICE_ACCOUNT_TOKEN")]
		op_service_account_token: Option<String>,
		#[clap(long)]
		op_namespace_path: Option<String>,
		#[clap(long)]
		op_secrets_path: Option<String>,
	},
	/// Push namespace config and secrets to 1Password.
	Push {
		#[clap(index = 1)]
		namespace: Option<String>,
		#[clap(long, short = 'y')]
		yes: bool,
		#[clap(long, env = "OP_SERVICE_ACCOUNT_TOKEN")]
		op_service_account_token: Option<String>,
		#[clap(long)]
		op_namespace_path: Option<String>,
		#[clap(long)]
		op_secrets_path: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			Self::Generate { namespace } => {
				let project_root = context::ProjectContextData::seek_project_root().await;
				tasks::config::generate(&project_root, &namespace).await?;
			}
			Self::SetNamespace { namespace } => {
				tasks::config::set_namespace(&namespace).await?;
			}
			Self::ServiceDependencies {
				svc_name,
				recursive,
				test,
			} => {
				let run_context = if *test {
					RunContext::Test {
						test_id: String::new(),
					}
				} else {
					RunContext::Service {}
				};

				// Build project
				let ctx = ProjectContextData::new(std::env::var("BOLT_NAMESPACE").ok()).await;

				// Read deps
				let deps = if *recursive {
					ctx.recursive_dependencies(&[&svc_name], &run_context).await
				} else {
					let svc = ctx.service_with_name(svc_name).await;
					svc.dependencies(&run_context).await
				};

				// Print deps
				for dep in deps {
					println!("{}", dep.name());
				}
			}
			Self::Show => {
				let ctx = ProjectContextData::new(std::env::var("BOLT_NAMESPACE").ok()).await;

				println!("{:#?}", ctx.ns());
			}
			Self::Pull {
				namespace,
				yes,
				op_service_account_token,
				op_namespace_path,
				op_secrets_path,
			} => {
				let term = rivet_term::terminal();

				let project_root = ProjectContextData::seek_project_root().await;
				let config_local =
					ProjectContextData::read_config_local(project_root.as_path()).await;
				let ns_id = namespace
					.clone()
					.or_else(|| std::env::var("BOLT_NAMESPACE").ok())
					.or_else(|| config_local.namespace.clone());

				// If no namespace id could be resolved, prompt for one
				let ns_id = if let Some(ns_id) = ns_id {
					rivet_term::status::progress("Pulling config for", &ns_id);

					ns_id
				} else {
					rivet_term::prompt::PromptBuilder::default()
						.message("Namespace name")
						.build()?
						.string(&term)
						.await?
				};

				let namespace_path = project_root
					.join("namespaces")
					.join(format!("{ns_id}.toml"));
				let mut op_namespace_path = op_namespace_path.clone();
				let mut op_secrets_path = op_secrets_path.clone();

				// Load namespace config
				let ns_config = if fs::metadata(&namespace_path).await.is_ok() {
					// TODO (RVT-3747): Parse as plain toml
					let ns_config =
						ProjectContextData::read_ns(project_root.as_path(), &ns_id).await;

					// Try to read op paths from ns config
					if let Some(_1password) = &ns_config.secrets._1password {
						if op_namespace_path.is_none() {
							op_namespace_path = Some(_1password.namespace_path.clone());
						}

						if op_secrets_path.is_none() {
							op_secrets_path = Some(_1password.secrets_path.clone());
						}
					}

					Some(ns_config)
				} else {
					None
				};

				// If no op paths could be resolved, prompt
				let op_namespace_path = if let Some(op_namespace_path) = op_namespace_path {
					op_namespace_path
				} else {
					rivet_term::prompt::PromptBuilder::default()
						.message(
							"1Password namespace config reference (i.e. \"op://vault/item/field\")",
						)
						.build()?
						.string(&term)
						.await?
				};
				let op_secrets_path =
					if let Some(op_secrets_path) = op_secrets_path {
						op_secrets_path
					} else {
						rivet_term::prompt::PromptBuilder::default()
							.message("1Password secret config reference (i.e. \"op://vault/item/field\")")
							.build()?
							.string(&term)
							.await?
					};

				// Determine local secrets path
				let secrets_path =
					ProjectContextData::get_secrets_path(ns_config.as_ref(), &project_root, &ns_id);

				// Load secrets
				let secrets = if fs::metadata(&secrets_path).await.is_ok() {
					Some(
						ProjectContextData::read_secrets(
							ns_config.as_ref(),
							project_root.as_path(),
							&ns_id,
						)
						.await,
					)
				} else {
					None
				};

				// Try to read service account token from local config
				let mut op_service_account_token = op_service_account_token.clone().or_else(|| {
					config_local
						._1password
						.as_ref()
						.map(|c| c.service_account_token.clone())
				});

				// If service account token could not be resolved, prompt for one
				if op_service_account_token.is_none() {
					let response = rivet_term::prompt::PromptBuilder::default()
						.message("1Password service account token (leave empty for manual login)")
						.allow_empty(true)
						.build()?
						.string_secure(&term)
						.await?;

					if response.is_empty() {
						dep::one_password::cli::login().await;
					} else {
						op_service_account_token = Some(response);
					}
				}

				eprintln!();

				// Fetch and parse configs from 1Password
				let op_namespace_str = dep::one_password::cli::read(
					op_service_account_token.as_deref(),
					&op_namespace_path,
				)
				.await;
				let op_namespace = toml::from_str::<serde_json::Value>(&op_namespace_str)
					.context("failed to read op namespace config")?;
				let op_secrets_str = dep::one_password::cli::read(
					op_service_account_token.as_deref(),
					&op_secrets_path,
				)
				.await;
				let op_secrets = toml::from_str::<serde_json::Value>(&op_secrets_str)
					.context("failed to read op secrets")?;

				// Check for diffs between the local and 1Password configs
				if ns_config.is_some() {
					let local_namespace_str = fs::read_to_string(&namespace_path).await?;
					let namespace = toml::from_str::<serde_json::Value>(&local_namespace_str)
						.context("failed to read namespace config")?;

					let patches = json_patch::diff(&namespace, &op_namespace);
					if !patches.is_empty() {
						rivet_term::status::warn("Warning",
						format!("Diff detected between local namespace file ({}) and 1Password namespace reference ({}):",
							namespace_path.display(),
							op_namespace_path
						));
						bolt_core::utils::render_diff(2, &patches);

						let term = rivet_term::terminal();
						let response = *yes
							|| rivet_term::prompt::PromptBuilder::default()
								.message("Overwrite local file?")
								.build()?
								.bool(&term)
								.await?;
						eprintln!();

						if response {
							rivet_term::status::progress("Overwriting local namespace config", "");
							fs::write(&namespace_path, op_namespace_str).await?;
						} else {
							rivet_term::status::info("Keeping local namespace config", "");
						}
					} else {
						rivet_term::status::info("No changes in namespace config", "");
					}
				} else {
					rivet_term::status::progress(
						"Writing namespace config",
						style(namespace_path.display()),
					);
					fs::write(&namespace_path, op_namespace_str).await?;
				}

				if let Some(secrets) = secrets {
					let patches = json_patch::diff(&secrets, &op_secrets);
					if !patches.is_empty() {
						rivet_term::status::warn("Warning",
						format!("Diff detected between local secrets file ({}) and 1Password secrets reference ({}):",
							secrets_path.display(),
							op_secrets_path
						));
						bolt_core::utils::render_diff(2, &patches);

						let term = rivet_term::terminal();
						let response = *yes
							|| rivet_term::prompt::PromptBuilder::default()
								.message("Overwrite local file?")
								.build()?
								.bool(&term)
								.await?;
						eprintln!();

						if response {
							rivet_term::status::progress("Overwriting local secrets", "");
							fs::write(&secrets_path, op_secrets_str).await?;
						} else {
							rivet_term::status::info("Keeping local secrets", "");
						}
					} else {
						rivet_term::status::info("No changes in secrets config", "");
					}
				} else {
					rivet_term::status::progress(
						"Writing secrets config",
						style(secrets_path.display()),
					);
					fs::write(&secrets_path, op_secrets_str).await?;
				}
			}
			Self::Push {
				namespace,
				yes,
				op_service_account_token,
				op_namespace_path,
				op_secrets_path,
			} => {
				let term = rivet_term::terminal();
				let ctx = ProjectContextData::new(
					namespace
						.clone()
						.or_else(|| std::env::var("BOLT_NAMESPACE").ok()),
				)
				.await;

				rivet_term::status::progress("Pushing config for", ctx.ns_id());

				let mut op_namespace_path = op_namespace_path.clone();
				let mut op_secrets_path = op_secrets_path.clone();

				// Try to read op paths from ns config
				if let Some(_1password) = &ctx.ns().secrets._1password {
					if op_namespace_path.is_none() {
						op_namespace_path = Some(_1password.namespace_path.clone());
					}

					if op_secrets_path.is_none() {
						op_secrets_path = Some(_1password.secrets_path.clone());
					}
				}

				// If no op paths could be resolved, prompt
				let op_namespace_path = if let Some(op_namespace_path) = op_namespace_path {
					op_namespace_path
				} else {
					rivet_term::prompt::PromptBuilder::default()
						.message(
							"1Password namespace config reference (i.e. \"op://vault/item/field\")",
						)
						.build()?
						.string(&term)
						.await?
				};
				let op_secrets_path =
					if let Some(op_secrets_path) = op_secrets_path {
						op_secrets_path
					} else {
						rivet_term::prompt::PromptBuilder::default()
							.message("1Password secret config reference (i.e. \"op://vault/item/field\")")
							.build()?
							.string(&term)
							.await?
					};

				// Try to read service account token from local config
				let mut op_service_account_token = op_service_account_token.clone().or_else(|| {
					ctx.config_local()
						._1password
						.as_ref()
						.map(|c| c.service_account_token.clone())
				});

				// If service account token could not be resolved, prompt for one
				if op_service_account_token.is_none() {
					let response = rivet_term::prompt::PromptBuilder::default()
						.message("1Password service account token (leave empty for manual login)")
						.allow_empty(true)
						.build()?
						.string_secure(&term)
						.await?;

					if response.is_empty() {
						dep::one_password::cli::login().await;
					} else {
						op_service_account_token = Some(response);
					}
				}

				eprintln!();

				// Fetch and parse configs from 1Password
				let op_namespace_str = dep::one_password::cli::read(
					op_service_account_token.as_deref(),
					&op_namespace_path,
				)
				.await;
				let op_namespace = toml::from_str::<serde_json::Value>(&op_namespace_str)
					.context("failed to parse op namespace config")?;
				let op_secrets_str = dep::one_password::cli::read(
					op_service_account_token.as_deref(),
					&op_secrets_path,
				)
				.await;
				let op_secrets = toml::from_str::<serde_json::Value>(&op_secrets_str)
					.context("failed to parse op secrets")?;

				// Fetch local configs
				let ns_id = ctx.ns_id();
				let namespace_path = ctx.ns_path().join(format!("{ns_id}.toml"));
				let secrets_path = ctx.secrets_path();

				let local_namespace_str = fs::read_to_string(&namespace_path).await?;
				let namespace = toml::from_str::<serde_json::Value>(&local_namespace_str)
					.context("failed to read namespace config")?;
				let local_secrets_str = fs::read_to_string(&secrets_path).await?;
				let secrets =
					ProjectContextData::read_secrets(Some(ctx.ns()), ctx.path(), &ns_id).await;

				let ns_patches = json_patch::diff(&op_namespace, &namespace);
				if !ns_patches.is_empty() {
					eprintln!("Changes between local namespace file ({}) and 1Password namespace reference ({}):",
						namespace_path.display(),
						op_namespace_path
					);
					bolt_core::utils::render_diff(2, &ns_patches);
				}

				let secrets_patches = json_patch::diff(&op_secrets, &secrets);
				if !secrets_patches.is_empty() {
					if !ns_patches.is_empty() {
						eprintln!();
					}

					eprintln!("Changes between local secrets file ({}) and 1Password secrets reference ({}):",
						secrets_path.display(),
						op_secrets_path,
					);
					bolt_core::utils::render_diff(2, &secrets_patches);
				}

				if !ns_patches.is_empty() || !secrets_patches.is_empty() {
					let response = *yes
						|| rivet_term::prompt::PromptBuilder::default()
							.message("Continue?")
							.build()?
							.bool(&term)
							.await?;
					eprintln!();

					if response {
						if !ns_patches.is_empty() {
							rivet_term::status::progress("Pushing namespace config...", "");
							dep::one_password::cli::write(
								op_service_account_token.as_deref(),
								&op_namespace_path,
								&ctx.gen_path().join("one_password").join("ns.json"),
								&local_namespace_str,
							)
							.await;
						}

						if !secrets_patches.is_empty() {
							rivet_term::status::progress("Pushing secrets...", "");
							dep::one_password::cli::write(
								op_service_account_token.as_deref(),
								&op_secrets_path,
								&ctx.gen_path().join("one_password").join("secrets.json"),
								&local_secrets_str,
							)
							.await;
						}

						rivet_term::status::success("Pushed", "");
					}
				} else {
					rivet_term::status::info("No changes to push", "");
				}
			}
			Self::SourceHash => {
				let ctx = bolt_core::context::ProjectContextData::new(
					std::env::var("BOLT_NAMESPACE").ok(),
				)
				.await;

				print!("{}", ctx.source_hash());
			}
		}

		Ok(())
	}
}
