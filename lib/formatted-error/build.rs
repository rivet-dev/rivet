use std::path::PathBuf;

use gray_matter::{engine::TOML, Matter};
use hashbrown::HashMap;
use indoc::formatdoc;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct FrontMatter {
	name: String,
	description: String,
	description_basic: Option<String>,
	http_status: u16,
}

struct Ctx {
	matter: Matter<TOML>,
	base_path: PathBuf,
	hash_items: Vec<String>,
	const_items: Vec<String>,
	existing_keys: HashMap<String, PathBuf>,
}

impl Ctx {
	fn new(base_path: PathBuf) -> Ctx {
		Ctx {
			matter: Matter::<TOML>::new(),
			base_path,
			hash_items: Vec::new(),
			const_items: Vec::new(),
			existing_keys: HashMap::new(),
		}
	}
}

fn main() -> std::io::Result<()> {
	let error_registry_path = {
		let mut path = std::env::current_dir()?;
		path.pop();
		path.pop();
		path.push("errors");
		path
	};

	println!("cargo:rerun-if-changed={}", error_registry_path.display());

	let mut ctx = Ctx::new(error_registry_path.clone());

	visit_dir(&mut ctx, error_registry_path)?;

	let output = formatdoc!(
		"
		use hashbrown::HashMap;
		use crate::utils::render_template;
		use lazy_static::lazy_static;
		use http::StatusCode;

		#[derive(Debug, Clone)]
		pub struct FormattedError {{
			name: &'static str,
			description_template: &'static str,
			description_basic: Option<&'static str>,
			http_status: StatusCode,
			documentation: &'static str,
		}}

		impl FormattedError {{
			fn new(
				name: &'static str,
				description_template: &'static str,
				description_basic: Option<&'static str>,
				http_status: u16,
				documentation: &'static str,
			) -> FormattedError
			{{
				FormattedError {{
					name,
					description_template,
					description_basic,
					http_status: StatusCode::from_u16(http_status).expect(\"invalid HTTP status code\"),
					documentation,
				}}
			}}

			pub fn name(&self) -> &'static str {{
				&self.name
			}}

			pub fn description(&self) -> String {{
				// Renders here to replace any existing {{}}'s with question marks
				render_template(
					self.description_basic
						.unwrap_or(self.description_template),
				   & std::collections::HashMap::new(),
				)
			}}

			pub fn format_description(&self, context: &std::collections::HashMap<String, String>) -> String {{
				render_template(self.description_template, context)
			}}

			pub fn http_status(&self) -> StatusCode {{
				self.http_status
			}}

			pub fn documentation(&self) -> &'static str {{
				self.documentation
			}}
		}}

		lazy_static! {{
			static ref ERROR_REGISTRY: HashMap<&'static str, FormattedError> = IntoIterator::into_iter([
				{hash}
			]).collect();
		}}
		
		pub mod code {{
			{consts}
		}}
		",
		hash = ctx.hash_items.join("\t\t"),
		consts = ctx.const_items.join("\t")
	);

	let output_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("gen.rs");

	// Write file
	std::fs::write(&output_path, output)?;

	Ok(())
}

fn visit_dir(ctx: &mut Ctx, path: PathBuf) -> std::io::Result<()> {
	for entry in std::fs::read_dir(path)? {
		let path = entry?.path();

		if path.is_dir() {
			visit_dir(ctx, path)?;
		} else {
			let contents = std::fs::read_to_string(path.clone())?;
			println!("Deserializing {}", path.display());
			let result = ctx.matter.parse(&contents);

			let data = result.data.unwrap().deserialize::<FrontMatter>().unwrap();
			let Some(title_line) = result.content.split('\n').find(|x| x.starts_with("# ")) else {
				panic!("Markdown content has no title (in {:?}).", path.display());
			};

			// Report an error on duplicate error name
			if let Some(existing_path) = ctx.existing_keys.get(&data.name) {
				panic!(
					"Duplicate error with name {:?} (from {} and {}).",
					data.name,
					existing_path.display(),
					path.display(),
				);
			} else {
				ctx.existing_keys.insert(data.name.clone(), path.clone());
			}

			// Validate frontmatter
			if http::StatusCode::from_u16(data.http_status).is_err() {
				panic!(
					"Invalid HTTP status code {:?} for error {:?} (in {})",
					data.http_status,
					data.name,
					path.display()
				);
			}

			let clean_title = title_line.to_lowercase()[2..]
				.chars()
				.filter_map(|x| {
					if x.is_alphanumeric() {
						Some(x)
					} else if x == ' ' {
						Some('-')
					} else {
						None
					}
				})
				.collect::<String>();
			let documentation = format!("https://rivet.gg/docs/general/errors#{clean_title}");

			ctx.hash_items.push(formatdoc!(
				"
				({name:?}, FormattedError::new({name:?}, {description:?}, {description_basic:?}, {http_status}, {documentation:?})),
				",
				name = data.name,
				description = data.description,
				description_basic = data.description_basic,
				http_status = data.http_status,
				documentation = documentation,
			));

			ctx.const_items.push(formatdoc!(
				"
				pub const {name}: &str = {name:?};
				",
				name = data.name,
			));
		}
	}

	Ok(())
}
