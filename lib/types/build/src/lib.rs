use std::{
	fs, io,
	path::{Path, PathBuf},
};

pub fn compile() -> io::Result<()> {
	// Include protobuf binaries
	#[cfg(feature = "protobuf-src")]
	{
		std::env::set_var("PROTOC", protobuf_src::protoc());
		std::env::set_var("PROTOC_INCLUDE", protobuf_src::include());
	}

	compile_with_base(|| Ok(schemac::CompileOpts::default()))
}

pub fn compile_with_base<F>(base_builder: F) -> io::Result<()>
where
	F: Fn() -> io::Result<schemac::CompileOpts>,
{
	let project_roots = seek_project_roots()?;
	let main_project_root = &project_roots[0];
	let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

	let mut input_paths = Vec::new();

	// Add common proto from root of project
	//
	// This will be symlinked if there are multiple roots
	println!(
		"cargo:rerun-if-changed={}",
		main_project_root.join("proto").display()
	);
	if main_project_root.join("proto").is_dir() {
		let paths = find_all_proto(&main_project_root.join("proto"))?;
		input_paths.extend(paths);
	};

	// Add protos from all services
	for project_root in &project_roots {
		for entry in fs::read_dir(project_root.join("svc").join("pkg"))? {
			let entry = entry?;
			let proto_path = entry.path().join("types");

			if proto_path.is_dir() {
				println!("cargo:rerun-if-changed={}", proto_path.display());

				let paths = find_all_proto(&proto_path)?;
				input_paths.extend(paths);
			}
		}
	}

	// Compile
	let input_paths = input_paths.iter().map(|x| x.as_path()).collect::<Vec<_>>();
	compile_proto_input(
		base_builder()?,
		input_paths.as_slice(),
		&out_dir.join("schema.rs"),
	)?;

	Ok(())
}

pub fn compile_proto_input(
	base_opts: schemac::CompileOpts,
	input_paths: &[&Path],
	output_path: &Path,
) -> io::Result<()> {
	// Compile schema
	let opts = update_compile_opts(base_opts)?.include(input_paths);
	let schema_file = schemac::compile(opts)?;

	// Disable clippy
	let schema_file = schema_file.replace(
		"pub mod ",
		"#[allow(clippy::all, warnings, unused)] pub mod ",
	);

	// Write file
	// println!("cargo:warning=schemac output: {}", output_path.display());
	fs::write(output_path, schema_file)?;

	Ok(())
}

fn update_compile_opts(mut base: schemac::CompileOpts) -> io::Result<schemac::CompileOpts> {
	let project_roots = seek_project_roots()?;

	base = base
		.root(&project_roots[0])
		.plugin(Box::new(plugins::CommonPlugin::default()))
		.plugin(Box::new(plugins::BackendMessagePlugin::default()));

	Ok(base)
}

fn seek_project_roots() -> io::Result<Vec<PathBuf>> {
	// Find project root
	let mut project_root = std::env::current_dir()?;
	loop {
		if project_root.join("Bolt.toml").exists() {
			// Found project root
			break;
		} else if let Some(parent) = project_root.parent() {
			project_root = parent.to_owned();
		} else {
			panic!("could not find project root");
		}
	}

	// Read project roots
	let bolt_toml = fs::read_to_string(project_root.join("Bolt.toml"))?;
	let project_config = bolt_config::project::decode(&bolt_toml)
		.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

	let mut roots = vec![project_root.clone()];
	for (_, additional_root) in project_config.additional_roots.iter() {
		roots.push(project_root.join(&additional_root.path));
	}

	Ok(roots)
}

fn find_all_proto(path: &Path) -> io::Result<Vec<PathBuf>> {
	Ok(fs::read_dir(path)?
		.into_iter()
		.map(|entry| {
			let entry = entry?;
			let path = entry.path();

			if path.is_dir() {
				return Ok(find_all_proto(&path)?);
			} else if let Some(extension) = path.extension() {
				if extension == "proto" {
					return Ok(vec![path]);
				}
			}

			Ok(vec![])
		})
		.collect::<io::Result<Vec<_>>>()?
		.into_iter()
		.flatten()
		.collect())
}

mod plugins {
	use std::io;

	use indoc::{formatdoc, indoc};
	use regex::Regex;

	use super::config;

	#[derive(Debug)]
	pub struct CommonPlugin {
		regex_uuid_derive: Regex,
	}

	impl Default for CommonPlugin {
		fn default() -> Self {
			Self {
				regex_uuid_derive: Regex::new(
					r#"(?s)(?:#\[derive\(Clone, PartialEq, ::prost::Message\)\])(?P<extra>.*)struct Uuid \{[^}]*\}"#,
				)
				.expect("parse regex"),
			}
		}
	}

	impl schemac::CompilePlugin for CommonPlugin {
		fn module_pass(
			&self,
			file_contents: &mut String,
			meta: &schemac::ModuleMeta,
		) -> Result<(), io::Error> {
			if meta.name == vec!["rivet", "common"] {
				*file_contents = self
					.regex_uuid_derive
					.replace(file_contents, include_str!("../static/uuid_expanded.rs"))
					.to_string();
			} else if meta.name == vec!["rivet", "backend", "matchmaker"] {
				file_contents.push_str(indoc!(
					r#"
					impl From<lobby_runtime::Docker> for LobbyRuntime {
						fn from(rt: lobby_runtime::Docker) -> LobbyRuntime {
							LobbyRuntime {
								runtime: Some(lobby_runtime::Runtime::Docker(rt)),
							}
						}
					}

					impl From<lobby_runtime_ctx::Docker> for LobbyRuntimeCtx {
						fn from(rt: lobby_runtime_ctx::Docker) -> LobbyRuntimeCtx {
							LobbyRuntimeCtx {
								runtime: Some(lobby_runtime_ctx::Runtime::Docker(rt)),
							}
						}
					}

					impl From<lobby_runtime_meta::Docker> for LobbyRuntimeMeta {
						fn from(rt: lobby_runtime_meta::Docker) -> LobbyRuntimeMeta {
							LobbyRuntimeMeta {
								runtime: Some(lobby_runtime_meta::Runtime::Docker(rt)),
							}
						}
					}
					"#
				))
			}

			Ok(())
		}
	}

	#[derive(Debug, Default)]
	pub struct BackendMessagePlugin;

	impl schemac::CompilePlugin for BackendMessagePlugin {
		fn module_pass(
			&self,
			file_contents: &mut String,
			meta: &schemac::ModuleMeta,
		) -> Result<(), io::Error> {
			// Generate message
			for msg in &meta.messages {
				if msg.name == "Message" {
					// Front matter
					let config = if let Some(comment) = &msg.comment {
						if comment.starts_with("/// /") {
							let frontmatter = comment
								.replace("/// /", "")
								.replace("\\[", "[")
								.replace("\\]", "]");
							let res = toml::from_str::<config::Message>(&frontmatter);

							match res {
								Ok(config) => Some(config),
								Err(err) => {
									eprintln!("{}:\n{}", meta.name.join("."), err);
									panic!("toml frontmatter error");
								}
							}
						} else {
							None
						}
					} else {
						None
					};

					if let Some(config) = config {
						let param_configs = config
							.parameters
							.iter()
							.map(|x| {
								format!(
								"::chirp_types::message::MessageSubjectParameter{{wildcard:{}}}",
								x.wildcard
							)
							})
							.collect::<Vec<_>>()
							.join(", ");

						let param_args = config
							.parameters
							.iter()
							.map(|x| format!("{}: impl std::fmt::Display", x.name))
							.collect::<Vec<String>>()
							.join(", ");

						let param_values = config
							.parameters
							.iter()
							.map(|x| format!("format!(\"{{}}\", {})", x.name))
							.collect::<Vec<_>>()
							.join(", ");

						let topic_config = "Some(::chirp_types::message::MessageTopic {})";

						let tail_ttl = if let Some(tail_ttl) = config.tail_ttl {
							format!("Some({})", tail_ttl)
						} else {
							"None".to_owned()
						};
						let history = config.history.to_string();

						file_contents.push_str(&formatdoc!(
							r#"
							impl ::chirp_types::message::Message for {message_msg_name} {{
								const NAME: &'static str = "{name}";
								const PARAMETERS: &'static [::chirp_types::message::MessageSubjectParameter] = &[{param_configs}];
								const TOPIC: Option<::chirp_types::message::MessageTopic> = {topic_config};
								const TAIL_TTL: Option<i64> = {tail_ttl};
								const HISTORY: bool = {history};

								const PERF_LABEL_SUBSCRIBE: &'static str = "subscribe-{name}";
								const PERF_LABEL_TAIL: &'static str = "tail-{name}";
								const PERF_LABEL_TAIL_READ: &'static str = "tail-read-{name}";
								const PERF_LABEL_TAIL_ANCHOR: &'static str = "tail-anchor-{name}";
								const PERF_LABEL_TAIL_ALL: &'static str = "tail-all-{name}";
								const PERF_LABEL_WRITE_STREAM: &'static str = "write-stream-{name}";
								const PERF_LABEL_WRITE_TAIL: &'static str = "write-tail-{name}";
								const PERF_LABEL_PUBLISH: &'static str = "publish-{name}";
							}}

							pub fn build_params({param_args}) -> Vec<String> {{
								vec![{param_values}]
							}}
							"#,
							message_msg_name = msg.name,
							name = config.name,
						));
					}
				}
			}

			Ok(())
		}
	}
}

mod config {
	use serde::Deserialize;

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct Message {
		pub name: String,
		pub tail_ttl: Option<u64>,
		#[serde(default)]
		pub history: bool,
		#[serde(default)]
		pub deduplicate: bool,
		pub parameters: Vec<MessageSubjectParameter>,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct MessageSubjectParameter {
		pub name: String,
		#[serde(default)]
		pub wildcard: bool,
	}
}
