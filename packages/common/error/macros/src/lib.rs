use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, LitStr, Meta, parse_macro_input};

#[proc_macro_derive(RivetError, attributes(error))]
pub fn derive_rivet_error(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	// Check if this is a struct or an enum
	match &input.data.clone() {
		syn::Data::Struct(data_struct) => derive_struct_error(input, data_struct),
		syn::Data::Enum(data_enum) => derive_enum_error(input, data_enum),
		_ => panic!("RivetError can only be derived for structs and enums"),
	}
}

fn derive_struct_error(input: DeriveInput, data_struct: &syn::DataStruct) -> TokenStream {
	let struct_name = &input.ident;
	let vis = &input.vis;

	// Extract error attributes
	let error_attr = input
		.attrs
		.iter()
		.find(|attr| attr.path().is_ident("error"))
		.expect("RivetError requires #[error(...)] attribute");

	let args = match &error_attr.meta {
		Meta::List(meta_list) => {
			let tokens = &meta_list.tokens;
			syn::parse2::<ErrorArgs>(tokens.clone())
				.expect("Failed to parse error attribute arguments")
		}
		_ => panic!("error attribute must be in the form #[error(...)]"),
	};

	// Generate the schema creation
	let group = &args.group;
	let code = &args.code;
	let description = &args.description;

	// Generate the output based on whether we have fields
	let output = match &data_struct.fields {
		Fields::Named(fields) => {
			let field_names = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

			if let Some(formatted) = &args.formatted_desc {
				quote! {
					impl #struct_name {
						#vis fn build(self) -> ::anyhow::Error {
							use ::rivet_error::{RivetError, RivetErrorSchema, RivetErrorSchemaWithMeta, MacroMarker};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchemaWithMeta<#struct_name> = RivetErrorSchemaWithMeta {
								schema: RivetErrorSchema {
									group: #group,
									code: #code,
									default_message: #description,
									meta_type: Some(stringify!(#struct_name)),
									_macro_marker: MacroMarker { _private: () },
								},
								message_fn: |meta: &#struct_name| -> String {
									::rivet_error::indoc::formatdoc! {
										#formatted,
										#(#field_names = meta.#field_names),*
									}
								},
								_phantom: ::std::marker::PhantomData,
							};

							SCHEMA.build_with(self)
						}
					}
				}
			} else {
				quote! {
					impl #struct_name {
						#vis fn build(self) -> ::anyhow::Error {
							use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchema = RivetErrorSchema {
								group: #group,
								code: #code,
								default_message: #description,
								meta_type: Some(stringify!(#struct_name)),
								_macro_marker: MacroMarker { _private: () },
							};

							let meta_json = ::serde_json::to_value(&self)
								.ok()
								.and_then(|v| ::serde_json::value::to_raw_value(&v).ok());

							let error = RivetError {
								schema: &SCHEMA,
								meta: meta_json,
								message: None,
							};
							::anyhow::Error::new(error)
						}
					}
				}
			}
		}
		Fields::Unnamed(fields) => {
			let field_count = fields.unnamed.len();
			let field_names = (0..field_count)
				.map(|i| syn::Ident::new(&format!("field{}", i), proc_macro2::Span::call_site()))
				.collect::<Vec<_>>();

			if let Some(formatted) = &args.formatted_desc {
				let struct_meta_fields = field_names
					.iter()
					.zip(fields.unnamed.iter())
					.map(|(field_name, field)| {
						let field_type = &field.ty;
						quote! { #field_name: #field_type }
					})
					.collect::<Vec<_>>();
				let meta_fields = field_names
					.iter()
					.enumerate()
					.map(|(i, field_name)| {
						let idx = syn::Index::from(i);
						quote! { #field_name: self.#idx }
					})
					.collect::<Vec<_>>();

				quote! {
					impl #struct_name {
						#vis fn build(self) -> ::anyhow::Error {
							use ::rivet_error::{RivetError, RivetErrorSchema, RivetErrorSchemaWithMeta, MacroMarker};

							#[derive(::serde::Serialize)]
							struct StructMeta {
								#(#struct_meta_fields),*
							}

							let meta = StructMeta {
								#(#meta_fields),*
							};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchemaWithMeta<StructMeta> = RivetErrorSchemaWithMeta {
								schema: RivetErrorSchema {
									group: #group,
									code: #code,
									default_message: #description,
									meta_type: Some(stringify!(#struct_name)),
									_macro_marker: MacroMarker { _private: () },
								},
								message_fn: |meta: &StructMeta| -> String {
									::rivet_error::indoc::formatdoc! {
										#formatted,
										#(meta.#field_names),*
									}
								},
								_phantom: ::std::marker::PhantomData,
							};

							SCHEMA.build_with(meta)
						}
					}
				}
			} else {
				let json_fields = field_names
					.iter()
					.map(|field_name| {
						let field_name_str = field_name.to_string();

						quote! { #field_name_str: #field_name }
					})
					.collect::<Vec<_>>();

				quote! {
					impl #struct_name {
						#vis fn build(self) -> ::anyhow::Error {
							use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchema = RivetErrorSchema {
								group: #group,
								code: #code,
								default_message: #description,
								meta_type: Some(stringify!(#struct_name)),
								_macro_marker: MacroMarker { _private: () },
							};

							let meta_value = ::serde_json::json!({
								#(#json_fields),*
							});

							let meta_json = ::serde_json::value::to_raw_value(&meta_value).ok();

							let error = RivetError {
								schema: &SCHEMA,
								meta: meta_json,
								message: None,
							};
							::anyhow::Error::new(error)
						}
					}
				}
			}
		}
		Fields::Unit => {
			quote! {
				impl #struct_name {
					#vis fn build(self) -> ::anyhow::Error {
						use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

						#[allow(non_upper_case_globals)]
						static SCHEMA: RivetErrorSchema = RivetErrorSchema {
							group: #group,
							code: #code,
							default_message: #description,
							meta_type: None,
							_macro_marker: MacroMarker { _private: () },
						};

						SCHEMA.build()
					}
				}
			}
		}
	};

	// Write error documentation
	if let Err(e) = write_error_doc(&args.group, &args.code, &args.description) {
		panic!(
			"Failed to write error documentation for {}.{}: {}",
			args.group, args.code, e
		);
	}

	// eprintln!("\n\n{output}\n");

	TokenStream::from(output)
}

fn derive_enum_error(input: DeriveInput, data_enum: &syn::DataEnum) -> TokenStream {
	let enum_name = &input.ident;
	let vis = &input.vis;

	// Extract group name from enum-level error attribute
	let error_attr = input
		.attrs
		.iter()
		.find(|attr| attr.path().is_ident("error"))
		.expect("RivetError on enum requires #[error(\"group\")] attribute");

	let group = match &error_attr.meta {
		Meta::List(meta_list) => {
			let tokens = &meta_list.tokens;
			let group_str = syn::parse2::<LitStr>(tokens.clone())
				.expect("Failed to parse enum error attribute arguments");
			group_str.value()
		}
		_ => panic!("error attribute for enum must be in the form #[error(\"group\")]"),
	};

	let mut variant_matches = Vec::new();

	// Process each variant
	for variant in &data_enum.variants {
		let variant_name = &variant.ident;

		// Extract error attributes from variant
		let variant_error_attr = variant
			.attrs
			.iter()
			.find(|attr| attr.path().is_ident("error"))
			.expect(&format!(
				"Variant {} requires #[error(...)] attribute",
				variant_name
			));

		let (code, description, formatted_desc) = match &variant_error_attr.meta {
			Meta::List(meta_list) => {
				let tokens = &meta_list.tokens;
				parse_variant_error_args(tokens).expect(&format!(
					"Failed to parse variant error attributes for {}",
					variant_name
				))
			}
			_ => panic!(
				"error attribute for variant must be in the form #[error(\"code\", \"description\")]"
			),
		};

		// Write error documentation
		if let Err(e) = write_error_doc(&group, &code, &description) {
			panic!(
				"Failed to write error documentation for {}.{}: {}",
				group, code, e
			);
		}

		// Handle variants with fields
		match &variant.fields {
			Fields::Named(fields) => {
				let field_names = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
				let field_patterns = quote! { { #(#field_names),* } };

				if let Some(formatted) = &formatted_desc {
					variant_matches.push(quote! {
						#enum_name::#variant_name #field_patterns => {
							use ::rivet_error::{RivetError, RivetErrorSchema, RivetErrorSchemaWithMeta, MacroMarker};

							#[derive(Serialize)]
							struct VariantMeta #fields

							let meta = VariantMeta {
								#(#field_names: #field_names),*
							};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchemaWithMeta<VariantMeta> = RivetErrorSchemaWithMeta {
								schema: RivetErrorSchema {
									group: #group,
									code: #code,
									default_message: #description,
									meta_type: Some(stringify!(#enum_name::#variant_name)),
									_macro_marker: MacroMarker { _private: () },
								},
								message_fn: |meta: &VariantMeta| -> String {
									::rivet_error::indoc::formatdoc! {
										#formatted,
										#(#field_names = meta.#field_names),*
									}
								},
								_phantom: ::std::marker::PhantomData,
							};

							SCHEMA.build_with(meta)
						}
					});
				} else {
					let json_fields = field_names
						.iter()
						.map(|field_name| {
							let field_name_str = field_name.as_ref().map(|x| x.to_string());

							quote! { #field_name_str: #field_name }
						})
						.collect::<Vec<_>>();

					variant_matches.push(quote! {
						#enum_name::#variant_name #field_patterns => {
							use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchema = RivetErrorSchema {
								group: #group,
								code: #code,
								default_message: #description,
								meta_type: Some(stringify!(#enum_name::#variant_name)),
								_macro_marker: MacroMarker { _private: () },
							};

							let meta_value = ::serde_json::json!({
								#(#json_fields),*
							});

							let meta_json = ::serde_json::value::to_raw_value(&meta_value).ok();

							let error = RivetError {
								schema: &SCHEMA,
								meta: meta_json,
								message: None,
							};
							::anyhow::Error::new(error)
						}
					});
				}
			}
			Fields::Unnamed(fields) => {
				let field_count = fields.unnamed.len();
				let field_names = (0..field_count)
					.map(|i| {
						syn::Ident::new(&format!("field{}", i), proc_macro2::Span::call_site())
					})
					.collect::<Vec<_>>();
				let field_patterns = quote! { ( #(#field_names),* ) };

				if let Some(formatted) = &formatted_desc {
					let meta_fields = field_names
						.iter()
						.zip(fields.unnamed.iter())
						.map(|(field_name, field)| quote! { #field_name: #field })
						.collect::<Vec<_>>();

					variant_matches.push(quote! {
						#enum_name::#variant_name #field_patterns => {
							use ::rivet_error::{RivetError, RivetErrorSchema, RivetErrorSchemaWithMeta, MacroMarker};

							#[derive(Serialize)]
							struct VariantMeta {
								#(#meta_fields),*
							}

							let meta = VariantMeta {
								#(#field_names: #field_names),*
							};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchemaWithMeta<VariantMeta> = RivetErrorSchemaWithMeta {
								schema: RivetErrorSchema {
									group: #group,
									code: #code,
									default_message: #description,
									meta_type: Some(stringify!(#enum_name::#variant_name)),
									_macro_marker: MacroMarker { _private: () },
								},
								message_fn: |meta: &VariantMeta| -> String {
									::rivet_error::indoc::formatdoc! {
										#formatted,
										#(meta.#field_names),*
									}
								},
								_phantom: ::std::marker::PhantomData,
							};

							SCHEMA.build_with(meta)
						}
					});
				} else {
					let json_fields = field_names
						.iter()
						.map(|field_name| {
							let field_name_str = field_name.to_string();

							quote! { #field_name_str: #field_name }
						})
						.collect::<Vec<_>>();

					variant_matches.push(quote! {
						#enum_name::#variant_name #field_patterns => {
							use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

							#[allow(non_upper_case_globals)]
							static SCHEMA: RivetErrorSchema = RivetErrorSchema {
								group: #group,
								code: #code,
								default_message: #description,
								meta_type: Some(stringify!(#enum_name::#variant_name)),
								_macro_marker: MacroMarker { _private: () },
							};

							let meta_value = ::serde_json::json!({
								#(#json_fields),*
							});

							let meta_json = ::serde_json::value::to_raw_value(&meta_value).ok();

							let error = RivetError {
								schema: &SCHEMA,
								meta: meta_json,
								message: None,
							};
							::anyhow::Error::new(error)
						}
					});
				}
			}
			Fields::Unit => {
				// Handle unit variants
				variant_matches.push(quote! {
					#enum_name::#variant_name => {
						use ::rivet_error::{RivetError, RivetErrorSchema, MacroMarker};

						#[allow(non_upper_case_globals)]
						static SCHEMA: RivetErrorSchema = RivetErrorSchema {
							group: #group,
							code: #code,
							default_message: #description,
							meta_type: None,
							_macro_marker: MacroMarker { _private: () },
						};

						SCHEMA.build()
					}
				});
			}
		}
	}

	let output = quote! {
		impl #enum_name {
			#vis fn build(self) -> ::anyhow::Error {
				match self {
					#(#variant_matches),*
				}
			}
		}
	};

	TokenStream::from(output)
}

fn parse_variant_error_args(
	tokens: &proc_macro2::TokenStream,
) -> syn::Result<(String, String, Option<String>)> {
	struct VariantErrorArgs {
		code: String,
		description: String,
		formatted_desc: Option<String>,
	}

	impl syn::parse::Parse for VariantErrorArgs {
		fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
			let code = input.parse::<LitStr>()?.value();
			input.parse::<syn::Token![,]>()?;

			let description = input.parse::<LitStr>()?.value();

			let mut formatted_desc = None;
			if input.peek(syn::Token![,]) {
				input.parse::<syn::Token![,]>()?;
				if input.peek(LitStr) {
					formatted_desc = Some(input.parse::<LitStr>()?.value());
				}
			}

			Ok(VariantErrorArgs {
				code,
				description,
				formatted_desc,
			})
		}
	}

	let args = syn::parse2::<VariantErrorArgs>(tokens.clone())?;
	Ok((args.code, args.description, args.formatted_desc))
}

struct ErrorArgs {
	group: String,
	code: String,
	description: String,
	formatted_desc: Option<String>,
}

impl syn::parse::Parse for ErrorArgs {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let group = input.parse::<LitStr>()?.value();
		input.parse::<syn::Token![,]>()?;

		let code = input.parse::<LitStr>()?.value();
		input.parse::<syn::Token![,]>()?;

		let description = input.parse::<LitStr>()?.value();

		let mut formatted_desc = None;
		// Check if there's a formatted description
		if input.peek(syn::Token![,]) {
			input.parse::<syn::Token![,]>()?;
			if input.peek(LitStr) {
				formatted_desc = Some(input.parse::<LitStr>()?.value());
			}
		}

		Ok(ErrorArgs {
			group,
			code,
			description,
			formatted_desc,
		})
	}
}

fn write_error_doc(group: &str, code: &str, message: &str) -> std::io::Result<()> {
	use std::fs;
	use std::io::Write;

	let workspace_root = find_workspace_root()?;
	let errors_dir = if std::env::var("RIVET_ERROR_OUTPUT_DIR").is_ok() {
		// If custom dir is specified, errors go directly there
		workspace_root
	} else {
		// Otherwise use the standard out/errors path
		workspace_root.join("out/errors")
	};
	fs::create_dir_all(&errors_dir)?;

	let filename = format!("{group}.{code}.json");
	let filepath = errors_dir.join(filename);

	// Create JSON structure
	let error_doc = serde_json::json!({
		"group": group,
		"code": code,
		"message": message
	});

	let content = serde_json::to_string_pretty(&error_doc)?;

	let mut file = fs::File::create(filepath)?;
	file.write_all(content.as_bytes())?;

	Ok(())
}

fn find_workspace_root() -> std::io::Result<std::path::PathBuf> {
	use std::path::Path;

	// Check if a custom output directory is specified via env var
	if let Ok(custom_dir) = std::env::var("RIVET_ERROR_OUTPUT_DIR") {
		return Ok(Path::new(&custom_dir).to_path_buf());
	}

	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
		.map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;

	let mut current = Path::new(&manifest_dir);

	loop {
		if current.join("Cargo.toml").exists() {
			let content = std::fs::read_to_string(current.join("Cargo.toml"))?;
			if content.contains("[workspace]") {
				return Ok(current.to_path_buf());
			}
		}

		match current.parent() {
			Some(parent) => current = parent,
			None => {
				return Ok(Path::new(&manifest_dir).to_path_buf());
			}
		}
	}
}
