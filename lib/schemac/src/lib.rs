use std::{
	collections::{HashMap, HashSet},
	fmt, fs, io,
	path::{Path, PathBuf},
};

use regex::Regex;

lazy_static::lazy_static! {
	static ref PROTO_IMPORT_RE: Regex =
		Regex::new(r#"import\s+"(.*?)"\s*;"#).unwrap();
	static ref RUST_MESSAGE_RE: Regex = Regex::new(
		r#"((?:(?:///[^\n]+)\n)+)?(?:#\[allow\(clippy::derive_partial_eq_without_eq\)\]\s*#\[derive\([^\)]*::prost::Message\)]\s*pub struct )(\w+)(\s*\{[^\}]*\})"#,
	)
	.unwrap();
	static ref RUST_MESSAGE_FIELD_RE: Regex = Regex::new(
		r#"#\[prost\(.*tags?\s*=\s*"(.+)".*\)]\s*pub (\w+):\s*(.+),\n"#,
	)
	.unwrap();
}

/// Contains metadata about a given module generated from a Protobuf.
#[derive(Clone, Debug)]
pub struct ModuleMeta {
	pub name: Vec<String>,
	pub messages: Vec<MessageMeta>,
}

#[derive(Clone, Debug)]
pub struct MessageMeta {
	pub comment: Option<String>,
	pub name: String,
	pub fields: Vec<MessageFieldMeta>,
}

#[derive(Clone, Debug)]
pub struct MessageFieldMeta {
	pub tags: Vec<u32>,
	pub name: String,
	pub ty: String,
}

impl MessageFieldMeta {
	pub fn tag(&self) -> u32 {
		*self.tags.first().unwrap()
	}
}

/// A compiler plugin used to manipulate file contents.
pub trait CompilePlugin: fmt::Debug {
	fn module_pass(&self, file_contents: &mut String, meta: &ModuleMeta) -> io::Result<()>;
}

pub struct CompileOpts {
	root_path: PathBuf,
	input_paths: Vec<PathBuf>,
	plugins: Vec<Box<dyn CompilePlugin>>,
	service_generator: Option<Box<dyn prost_build::ServiceGenerator>>,
	extern_paths: Vec<(String, String)>,
	type_attributes: Vec<(String, String)>,
	skip_cargo_instructions: bool,
}

impl fmt::Debug for CompileOpts {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_struct("CompileOpts")
			.field("root_path", &self.root_path)
			.field("input_paths", &self.input_paths)
			.field("plugins", &self.plugins)
			.field("extern_paths", &self.extern_paths)
			.field("type_attributes", &self.type_attributes)
			.finish()
	}
}

impl Default for CompileOpts {
	fn default() -> Self {
		CompileOpts {
			root_path: PathBuf::default(),
			input_paths: Vec::new(),
			plugins: Vec::new(),
			service_generator: None,
			extern_paths: Vec::new(),
			type_attributes: Vec::new(),
			skip_cargo_instructions: false,
		}
	}
}

impl CompileOpts {
	pub fn root(mut self, root: &Path) -> Self {
		self.root_path = root.to_path_buf();

		self
	}

	pub fn include(mut self, files: &[&Path]) -> Self {
		for file in files {
			self.input_paths.push(file.to_path_buf());
		}

		self
	}

	pub fn include_dir(mut self, dir_path: &Path) -> io::Result<Self> {
		for entry in fs::read_dir(dir_path)? {
			let entry = entry?;
			let file_name_raw = entry.file_name();
			let file_name = file_name_raw.to_str().unwrap();
			let file_type = entry.file_type()?;

			// Add file
			if file_type.is_file() && file_name.ends_with(".proto") {
				self.input_paths.push(entry.path());
			}

			// Recursively add folder
			self = if file_type.is_dir() {
				self.include_dir(&entry.path())?
			} else {
				self
			};
		}

		Ok(self)
	}

	pub fn plugin(mut self, plugin: Box<dyn CompilePlugin>) -> Self {
		self.plugins.push(plugin);

		self
	}

	pub fn service_generator(
		mut self,
		service_generator: Box<dyn prost_build::ServiceGenerator>,
	) -> Self {
		self.service_generator = Some(service_generator);

		self
	}

	pub fn extern_path(mut self, proto_path: &str, rust_path: &str) -> Self {
		self.extern_paths
			.push((proto_path.to_owned(), rust_path.to_owned()));

		self
	}

	pub fn type_attribute(mut self, path: &str, attribute: &str) -> Self {
		self.type_attributes
			.push((path.to_owned(), attribute.to_owned()));

		self
	}

	pub fn skip_cargo_instructions(mut self) -> Self {
		self.skip_cargo_instructions = true;

		self
	}
}

/// Represents a module inside the protobuf module. We use this to combine multiple outputted
/// protobuf files in to one schema file.
struct SchemaNode {
	children: HashMap<String, SchemaNode>,
	code: Option<String>,
}

impl Default for SchemaNode {
	fn default() -> Self {
		SchemaNode {
			children: HashMap::new(),
			code: None,
		}
	}
}

impl SchemaNode {
	fn insert(&mut self, module_name: &[String], code: String) {
		assert_ne!(module_name.len(), 0);

		// Get the entry
		let entry = self.children.entry(module_name[0].clone()).or_default();
		if module_name.len() == 1 {
			// Insert code
			assert!(entry.code.is_none());
			entry.code = Some(code);
		} else {
			// Recursively insert module
			entry.insert(&module_name[1..], code);
		}
	}

	fn gen_code(&self, buf: &mut String) {
		// Add code
		if let Some(code) = &self.code {
			buf.push_str(&code);
		}

		// Generate child modules in alphabetical order
		let mut children = self
			.children
			.iter()
			.collect::<Vec<(&String, &SchemaNode)>>();
		children.sort_by_key(|p| p.0);
		for (name, node) in children {
			buf.push_str("pub mod ");
			buf.push_str(&name);
			buf.push_str(" {");
			node.gen_code(buf);
			buf.push_str("}");
		}
	}
}

pub fn compile(opts: CompileOpts) -> io::Result<String> {
	println!("> Compiling:\n{:#?}", opts);

	if !opts.skip_cargo_instructions {
		// Aggregate import paths
		let mut all_paths = HashSet::new();
		for path in &opts.input_paths {
			list_all_proto_paths(&opts.root_path, path, &mut all_paths)?;
		}
	}

	// Validate proto folder exists
	let protoc_out_dir = tempfile::tempdir()?;

	// Compile proto files
	println!("  * Running prost compiler");
	let mut build_config = prost_build::Config::new();
	build_config.protoc_arg("--experimental_allow_proto3_optional");
	build_config.out_dir(protoc_out_dir.path());
	if let Some(service_generator) = opts.service_generator {
		build_config.service_generator(service_generator);
	}
	for (proto_path, rust_path) in &opts.extern_paths {
		build_config.extern_path(proto_path, rust_path);
	}
	for (path, attribute) in &opts.type_attributes {
		build_config.type_attribute(path, attribute);
	}

	let res = build_config.compile_protos(&opts.input_paths, &[opts.root_path.clone()]);
	match res {
		Err(err) => {
			let err_kind = err.kind();

			if let Some(inner) = err.into_inner() {
				eprintln!("{}", inner);
				return Err(io::Error::new(io::ErrorKind::Other, "protoc failed"));
			} else {
				return Err(io::Error::new(err_kind, "failed to get inner error"));
			}
		}
		_ => {}
	}

	// Build schema graph
	println!("  * Building schema graph");
	let mut schema_module = SchemaNode::default();
	for entry in fs::read_dir(&protoc_out_dir)? {
		let entry = entry?;

		let file_name_raw = entry.file_name();
		let file_name = file_name_raw.to_str().unwrap();

		// Register code
		if entry.file_type()?.is_file() && file_name.ends_with(".rs") {
			// Compile module name
			let mut module_name = file_name
				.split(".")
				.map(|s| s.to_owned())
				.collect::<Vec<String>>();
			assert_eq!(module_name.pop(), Some("rs".to_owned()));

			// Read file
			println!("	* Reading module source: {}", entry.path().display());
			let mut file_contents = fs::read_to_string(entry.path())?;

			// Analyze source
			let mut messages = RUST_MESSAGE_RE
				.captures_iter(&file_contents)
				.map(|cap_msg| {
					let comment = cap_msg.get(1).map(|m| m.as_str().to_owned());
					let name = cap_msg.get(2).unwrap().as_str().to_owned();
					let body = cap_msg.get(3).unwrap(); // TODO: parse this

					// Parse message fields
					let mut fields = RUST_MESSAGE_FIELD_RE
						.captures_iter(body.as_str())
						.map(|cap_field| {
							// Parse the Proto tags to integers
							let mut tags = cap_field
								.get(1)
								.unwrap()
								.as_str()
								.split(",")
								.map(|s| s.trim())
								.map(|s| s.parse::<u32>().unwrap())
								.collect::<Vec<u32>>();
							tags.sort();

							MessageFieldMeta {
								tags,
								name: cap_field.get(2).unwrap().as_str().to_owned(),
								ty: cap_field.get(3).unwrap().as_str().to_owned(),
							}
						})
						.collect::<Vec<MessageFieldMeta>>();
					fields.sort_by_key(|f| f.tag());

					MessageMeta {
						comment,
						name,
						fields,
					}
				})
				.collect::<Vec<MessageMeta>>();
			messages.sort_by_cached_key(|m| m.name.clone());

			// TODO: Handle submodules with separate ModuleMeta objects correctly; we may want to
			// switch to syn for this

			let meta = ModuleMeta {
				name: module_name,
				messages,
			};

			// Run plugins
			for plugin in &opts.plugins {
				println!("  * Running plugin: {:?}", plugin);
				plugin.module_pass(&mut file_contents, &meta)?;
			}

			// Save file
			schema_module.insert(&meta.name, file_contents);
		}
	}

	// Generate code
	let mut buf = String::new();
	schema_module.gen_code(&mut buf);

	// // Format code with rustfmt
	// let mut cmd = Command::new("rustfmt")
	//	 .arg("+stable")
	//	 .arg("--edition=2021")
	//	 .stdin(Stdio::piped())
	//	 .stdout(Stdio::piped())
	//	 .spawn()?;
	// cmd.stdin.as_mut().unwrap().write_all(buf.as_bytes())?;

	// // let mut buf_formatted = String::new();
	// // cmd.stdout
	// //	 .as_mut()
	// //	 .unwrap()
	// //	 .read_to_string(&mut buf_formatted)?;

	// // let exit_status = cmd.wait()?;
	// // assert!(exit_status.success(), "failed to format schema file");

	// let output = cmd.wait_with_output()?;
	// assert!(output.status.success(), "failed to format schema file");
	// let buf_formatted = String::from_utf8(output.stdout).unwrap();

	Ok(buf)
}

fn list_all_proto_paths(
	root: &Path,
	proto_path: &Path,
	all_paths: &mut HashSet<PathBuf>,
) -> io::Result<()> {
	// Save path
	all_paths.insert(proto_path.to_owned());

	// Read the file
	println!("Parsing imports {}...", proto_path.display());
	let proto_source = if proto_path.exists() {
		fs::read_to_string(&proto_path)?
	} else {
		println!("Proto at {} does not exist.", proto_path.display());
		return Ok(());
	};

	// Match imports
	for capture in PROTO_IMPORT_RE.captures_iter(&proto_source) {
		let import_path = capture.get(1).unwrap().as_str();

		// Ignore stdlib
		if import_path.starts_with("google/") {
			continue;
		}

		let full_path = root.join(import_path);

		// Recursively parse imports
		list_all_proto_paths(root, &full_path, all_paths)?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn build_all_proto() {
		let current_dir = std::env::current_dir().unwrap();
		let project_root = current_dir.parent().unwrap().parent().unwrap();
		let proto_dir = project_root.join("proto");

		let output = compile(
			CompileOpts::default()
				.root(&project_root)
				.include_dir(&proto_dir)
				.unwrap(),
		)
		.unwrap();

		println!("Output:\n{}", output);
	}
}
