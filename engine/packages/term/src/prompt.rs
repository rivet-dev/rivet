use std::str::FromStr;

use console::{style, Term};
use derive_builder::Builder;

use crate::{error::Result, status};

#[derive(Builder)]
#[builder(setter(into))]
pub struct Prompt {
	message: String,
	#[builder(setter(strip_option), default)]
	context: Option<String>,
	#[builder(setter(strip_option), default)]
	docs: Option<String>,
	#[builder(setter(strip_option), default)]
	docs_url: Option<String>,
	#[builder(setter(strip_option), default)]
	default_value: Option<String>,
	#[builder(default)]
	indent: usize,
	#[builder(default)]
	allow_empty: bool,
}

impl Prompt {
	fn gen_indent(&self) -> String {
		"    ".repeat(self.indent)
	}

	fn print_header(&self) {
		let i = self.gen_indent();

		eprintln!();
		eprint!("{i}");
		if let Some(context) = &self.context {
			eprint!("{} ", style(format!("[{context}]")).bold());
		}
		eprintln!("{}", style(&self.message).bold().blue());
		if let Some(docs) = &self.docs {
			eprintln!("{i}  {}", style(&docs).italic());
		}
		if let Some(docs_url) = &self.docs_url {
			eprintln!("{i}  {}", style(&docs_url).italic().underlined().cyan());
		}
		if let Some(default_value) = &self.default_value {
			eprintln!(
				"{i}  {} {}",
				style("Defaults to").italic(),
				style(&default_value).italic().bold()
			);
		}
	}

	async fn read_line(&self, term: &Term) -> Result<String> {
		self.read_line_inner(term, false).await
	}

	async fn read_line_secure(&self, term: &Term) -> Result<String> {
		self.read_line_inner(term, true).await
	}

	async fn read_line_inner(&self, term: &Term, secure: bool) -> Result<String> {
		term.flush()?;

		let input = if secure {
			tokio::task::spawn_blocking({
				let term = term.clone();
				move || term.read_secure_line()
			})
			.await??
		} else {
			tokio::task::spawn_blocking({
				let term = term.clone();
				move || term.read_line()
			})
			.await??
		};

		let input_trimmed = input.trim();

		if input_trimmed.is_empty() {
			if let Some(default_value) = self.default_value.as_ref() {
				return Ok(default_value.clone());
			}
		}

		Ok(input_trimmed.to_string())
	}

	pub async fn bool(&self, term: &Term) -> Result<bool> {
		let i = self.gen_indent();

		self.print_header();

		loop {
			eprint!("{i}  {}", style("[y/n] ").bold());
			let input = self.read_line(term).await?;

			match input.to_lowercase().as_str() {
				"y" | "yes" | "t" | "true" => return Ok(true),
				"n" | "no" | "f" | "false" => return Ok(false),
				_ => {
					status::error(format!("{i}  Invalid bool"), "Must be y or n");
				}
			}
		}
	}

	pub async fn parsed<T>(&self, term: &Term) -> Result<T>
	where
		T: FromStr,
	{
		let i = self.gen_indent();

		self.print_header();

		loop {
			eprint!("{i}  {} ", style(">").bold());
			let input = self.read_line(term).await?;

			if let Ok(parsed) = input.parse::<T>() {
				return Ok(parsed);
			} else {
				status::error(format!("{i}  Invalid input"), "");
			}
		}
	}

	pub async fn string(&self, term: &Term) -> Result<String> {
		let i = self.gen_indent();

		self.print_header();

		loop {
			eprint!("{i}  {} ", style(">").bold());
			let input = self.read_line(term).await?;

			if self.allow_empty || !input.is_empty() {
				return Ok(input);
			} else {
				status::error(format!("{i}  Empty input"), "");
			}
		}
	}

	pub async fn string_secure(&self, term: &Term) -> Result<String> {
		let i = self.gen_indent();

		self.print_header();

		loop {
			eprint!("{i}  {} ", style("[input hidden]").bold());
			let input = self.read_line_secure(term).await?;

			if self.allow_empty || !input.is_empty() {
				return Ok(input);
			} else {
				status::error(format!("{i}  Empty input"), "");
			}
		}
	}
}
