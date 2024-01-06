use std::{
	fmt,
	path::{Path, PathBuf},
};

use anyhow::*;
use async_recursion::async_recursion;
use tokio::{fs, io::AsyncWriteExt};

use crate::context::ProjectContext;

pub enum TemplateType {
	Api,
	Bucket,
	Database,
	Operation,
	Standalone,
	Worker,
}

impl fmt::Display for TemplateType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			TemplateType::Api => write!(f, "API"),
			TemplateType::Bucket => write!(f, "bucket"),
			TemplateType::Database => write!(f, "database"),
			TemplateType::Operation => write!(f, "operation"),
			TemplateType::Standalone => write!(f, "standalone"),
			TemplateType::Worker => write!(f, "worker"),
		}
	}
}

pub struct TemplateOpts {
	pub root: Option<String>,
	pub create_pkg: bool,
	pub pkg_name: String,
	pub template_type: TemplateType,
	pub service_name: String,
}

pub async fn generate(ctx: &mut ProjectContext, opts: TemplateOpts) -> Result<()> {
	let TemplateOpts {
		root,
		create_pkg,
		pkg_name,
		template_type,
		service_name,
	} = opts;

	// Create base path based on selected root
	let base_path = if let Some(root) = root {
		let root_path = &ctx
			.config()
			.additional_roots
			.get(&root)
			.ok_or_else(|| anyhow!("Root `{}` not found in local config", root))?
			.path;

		ctx.path().join(root_path)
	} else {
		ctx.path().to_owned()
	};

	if !create_pkg
		&& !matches!(template_type, TemplateType::Api)
		&& fs::metadata(base_path.join("svc").join("pkg").join(&pkg_name))
			.await
			.is_err()
	{
		eprintln!("{}", base_path.display());
		let relative_path = base_path
			.strip_prefix(ctx.path())
			.expect("strip path")
			.display();

		bail!(
			"Package `{}` does not exist at `{}`. Use `--create-pkg` to suppress this message.",
			pkg_name,
			relative_path
		);
	}

	// Build templating manager
	let mut hb = handlebars::Handlebars::new();
	hb.register_helper("snake", Box::new(handlebars_helpers::snake));
	hb.register_helper(
		"screaming_snake",
		Box::new(handlebars_helpers::screaming_snake),
	);
	let render_data = handlebars_helpers::RenderData {
		pkg: pkg_name.clone(),
		name: service_name.clone(),
	};

	// Get template directory
	let input_path = match template_type {
		TemplateType::Api => base_path.join("svc").join("templates").join("api"),
		TemplateType::Bucket => base_path.join("svc").join("templates").join("bucket"),
		TemplateType::Database => base_path.join("svc").join("templates").join("database"),
		TemplateType::Operation => base_path.join("svc").join("templates").join("operation"),
		TemplateType::Standalone => base_path.join("svc").join("templates").join("standalone"),
		TemplateType::Worker => base_path.join("svc").join("templates").join("worker"),
	};
	let output_path = match template_type {
		TemplateType::Api => base_path.join("svc").join("api").join(&service_name),
		TemplateType::Bucket => base_path
			.join("svc")
			.join("pkg")
			.join(&pkg_name)
			.join("buckets")
			.join(&service_name),
		TemplateType::Database => base_path
			.join("svc")
			.join("pkg")
			.join(&pkg_name)
			.join("db")
			.join(&service_name),
		TemplateType::Operation => base_path
			.join("svc")
			.join("pkg")
			.join(&pkg_name)
			.join("ops")
			.join(&service_name),
		TemplateType::Standalone => base_path
			.join("svc")
			.join("pkg")
			.join(&pkg_name)
			.join("standalone")
			.join(&service_name),
		TemplateType::Worker => base_path
			.join("svc")
			.join("pkg")
			.join(&pkg_name)
			.join("worker"),
	};

	rivet_term::status::progress(format!("Creating new {} service...", template_type), "");
	eprintln!("");

	// Generate new service
	match template_type {
		TemplateType::Worker => {
			let proto_path = base_path
				.join("svc")
				.join("pkg")
				.join(pkg_name)
				.join("types")
				.join("msg");
			let proto_file_path = proto_path.join(format!("{}.proto", service_name));

			// Check if proto exists
			if fs::metadata(&proto_file_path).await.is_ok() {
				bail!(
					"Worker protobuf definition already exists at {}",
					proto_file_path.display()
				);
			}

			// Check if worker parent already exists
			if check_service_exists(&output_path).await.is_err() {
				generate_worker_partial(
					&mut hb,
					&render_data,
					input_path,
					output_path,
					service_name,
				)
				.await?;
			} else {
				generate_dir(&mut hb, &render_data, input_path, output_path).await?;
			}

			// Generate proto file
			fs::create_dir_all(&proto_path).await?;
			generate_file(
				&mut hb,
				&render_data,
				base_path
					.join("svc")
					.join("templates")
					.join("types")
					.join("msg")
					.join("{{ name }}.proto"),
				proto_path,
			)
			.await?;
		}
		TemplateType::Operation => {
			// Check if proto exists
			let proto_path = base_path
				.join("svc")
				.join("pkg")
				.join(pkg_name)
				.join("types");
			let proto_file_path = proto_path.join(format!("{}.proto", service_name));
			if fs::metadata(&proto_file_path).await.is_ok() {
				bail!(
					"Operation protobuf definition already exists at {}",
					proto_file_path.display()
				);
			}

			check_service_exists(&output_path).await?;
			generate_dir(&mut hb, &render_data, input_path, output_path).await?;

			// Generate proto file
			fs::create_dir_all(&proto_path).await?;
			generate_file(
				&mut hb,
				&render_data,
				base_path
					.join("svc")
					.join("templates")
					.join("types")
					.join("{{ name }}.proto"),
				proto_path,
			)
			.await?;
		}
		_ => {
			check_service_exists(&output_path).await?;
			generate_dir(&mut hb, &render_data, input_path, output_path).await?;
		}
	};

	eprintln!("");
	rivet_term::status::success("Done", "");

	Ok(())
}

async fn generate_worker_partial(
	hb: &mut handlebars::Handlebars<'_>,
	render_data: &handlebars_helpers::RenderData,
	input_path: PathBuf,
	output_path: PathBuf,
	service_name: String,
) -> Result<()> {
	let snake_name = service_name.replace("-", "_");

	// Create directories
	let workers_root = output_path.join("src").join("workers");
	let tests_root = output_path.join("tests");
	fs::create_dir_all(&workers_root).await?;
	fs::create_dir_all(&tests_root).await?;

	rivet_term::status::progress("Worker parent already exists, skipping initiation...", "");

	// Check if source code exists
	let worker_file_path = workers_root.join(format!("{}.rs", snake_name));
	if fs::metadata(&worker_file_path).await.is_ok() {
		bail!(
			"Worker file already exists at {}",
			worker_file_path.display()
		);
	}

	generate_file(
		hb,
		&render_data,
		input_path
			.join("src")
			.join("workers")
			.join("{{ snake name }}.rs"),
		workers_root,
	)
	.await?;
	generate_file(
		hb,
		&render_data,
		input_path.join("tests").join("{{ snake name }}.rs"),
		tests_root,
	)
	.await?;

	// Update worker mod.rs
	{
		let worker_mod_path = output_path.join("src").join("workers").join("mod.rs");
		rivet_term::status::progress("Editing", worker_mod_path.display());
		let mut worker_mod = fs::OpenOptions::new()
			.write(true)
			.append(true)
			.open(worker_mod_path)
			.await?;

		worker_mod
			.write(format!("pub mod {};\n", snake_name).as_str().as_bytes())
			.await?;
	}

	// Update main.rs
	{
		let main_path = output_path.join("src").join("main.rs");
		rivet_term::status::progress("Editing", main_path.display());
		let main_str = fs::read_to_string(&main_path).await?;

		// Find place to insert text
		let insert_idx = main_str
			.find("worker_group!")
			.and_then(|idx| (&main_str[idx..]).find(']').map(|idx2| idx + idx2))
			.ok_or_else(|| anyhow!("Invalid main.rs file in worker: {}", main_path.display()))?
			- 3;

		fs::write(
			main_path,
			&format!(
				"{}\n\t\t\t{},{}",
				&main_str[..insert_idx],
				snake_name,
				&main_str[insert_idx..]
			),
		)
		.await?;
	}

	Ok(())
}

async fn check_service_exists(output_path: &Path) -> Result<()> {
	// Check if service directory already exists
	if fs::metadata(&output_path).await.is_ok() {
		bail!("Service already exists at {}", output_path.display());
	}

	Ok(())
}

#[async_recursion]
async fn generate_dir(
	hb: &mut handlebars::Handlebars<'_>,
	render_data: &handlebars_helpers::RenderData,
	input_path: PathBuf,
	output_path: PathBuf,
) -> Result<()> {
	// Make sure output directory exists
	fs::create_dir_all(&output_path).await?;

	// Read each file in directory to either template or recursively call this
	// function
	let mut entries = fs::read_dir(input_path.as_path()).await?;
	while let Some(entry) = entries.next_entry().await? {
		let metadata = entry.metadata().await?;
		if metadata.is_file() {
			generate_file(hb, &render_data, entry.path(), output_path.clone()).await?;
		} else if metadata.is_dir() {
			// Recursively generate next directory
			generate_dir(
				hb,
				render_data,
				input_path.join(entry.file_name()),
				output_path.join(entry.file_name()),
			)
			.await?;
		}
	}

	Ok(())
}

async fn generate_file(
	hb: &mut handlebars::Handlebars<'_>,
	render_data: &handlebars_helpers::RenderData,
	input_file_path: PathBuf,
	output_path: PathBuf,
) -> Result<()> {
	let output_file_path = output_path.join(hb.render_template(
		input_file_path.file_name().unwrap().to_str().unwrap(),
		render_data,
	)?);
	rivet_term::status::progress(
		"Templating",
		format!(
			"{} -> {}",
			input_file_path.display(),
			output_file_path.display(),
		),
	);

	// Template the file and write to output
	let template_name = input_file_path.to_string_lossy().to_string();
	hb.register_template_file(template_name.as_str(), input_file_path)?;
	let output = hb.render(template_name.as_str(), render_data)?;
	fs::write(output_file_path, output).await?;

	Ok(())
}

mod handlebars_helpers {
	use handlebars::{
		Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError,
	};
	use serde::Serialize;

	#[derive(Serialize)]
	pub struct RenderData {
		pub pkg: String,
		pub name: String,
	}

	pub fn snake(
		h: &Helper,
		_: &Handlebars,
		_: &Context,
		_: &mut RenderContext,
		out: &mut dyn Output,
	) -> HelperResult {
		let param = h.param(0).unwrap();
		let value = param
			.value()
			.as_str()
			.ok_or_else(|| RenderError::new("Could not convert value to string"))?
			.replace("-", "_");
		out.write(value.as_str())?;
		Ok(())
	}

	pub fn screaming_snake(
		h: &Helper,
		_: &Handlebars,
		_: &Context,
		_: &mut RenderContext,
		out: &mut dyn Output,
	) -> HelperResult {
		let param = h.param(0).unwrap();
		let value = param
			.value()
			.as_str()
			.ok_or_else(|| RenderError::new("Could not convert value to string"))?
			.replace("-", "_")
			.to_uppercase();
		out.write(value.as_str())?;
		Ok(())
	}
}
