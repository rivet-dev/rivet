use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	spanned::Spanned,
	GenericArgument, Ident, Item, ItemFn, ItemStruct, LitStr, PathArguments, ReturnType, Type,
};

struct Config {
	max_retries: usize,
	timeout: u64,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			max_retries: 5,
			timeout: 30,
		}
	}
}

struct MessageConfig {
	tail_ttl: u64,
}

impl Default for MessageConfig {
	fn default() -> Self {
		MessageConfig { tail_ttl: 90 }
	}
}

#[proc_macro_attribute]
pub fn workflow(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as OptionalIdent)
		.ident
		.map(|x| x.to_string())
		.unwrap_or_else(|| "Workflow".to_string());
	let item_fn = parse_macro_input!(item as ItemFn);

	if let Err(err) = parse_empty_config(&item_fn.attrs) {
		return err.into_compile_error().into();
	}

	let ctx_ty = syn::parse_str("&mut WorkflowCtx").unwrap();
	let TraitFnOutput {
		ctx_ident,
		input_ident,
		input_type,
		output_type,
	} = match parse_trait_fn(&ctx_ty, "Workflow", &item_fn) {
		Ok(x) => x,
		Err(err) => return err,
	};

	let struct_ident = Ident::new(&name, proc_macro2::Span::call_site());
	let fn_name = item_fn.sig.ident.to_string();
	let fn_body = item_fn.block;
	let vis = item_fn.vis;

	let expanded = quote! {
		#vis struct #struct_ident;

		impl chirp_workflow::workflow::WorkflowInput for #input_type {
			type Workflow = #struct_ident;
		}

		#[async_trait::async_trait]
		impl chirp_workflow::workflow::Workflow for #struct_ident {
			type Input = #input_type;
			type Output = #output_type;

			const NAME: &'static str = #fn_name;

			async fn run(#ctx_ident: #ctx_ty, #input_ident: &Self::Input) -> GlobalResult<Self::Output> {
				#fn_body
			}
		}
	};

	// eprintln!("\n\n{expanded}\n");

	TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn activity(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as Ident).to_string();
	let item_fn = parse_macro_input!(item as ItemFn);

	let config = match parse_config(&item_fn.attrs) {
		Ok(x) => x,
		Err(err) => return err.into_compile_error().into(),
	};

	let ctx_ty = syn::parse_str("&ActivityCtx").unwrap();
	let TraitFnOutput {
		ctx_ident,
		input_ident,
		input_type,
		output_type,
	} = match parse_trait_fn(&ctx_ty, "Activity", &item_fn) {
		Ok(x) => x,
		Err(err) => return err,
	};

	let struct_ident = Ident::new(&name, proc_macro2::Span::call_site());
	let fn_name = item_fn.sig.ident.to_string();
	let fn_body = item_fn.block;
	let vis = item_fn.vis;

	let max_retries = config.max_retries;
	let timeout = config.timeout;

	let expanded = quote! {
		#vis struct #struct_ident;

		impl chirp_workflow::activity::ActivityInput for #input_type {
			type Activity = #struct_ident;
		}

		#[async_trait::async_trait]
		impl chirp_workflow::activity::Activity for #struct_ident {
			type Input = #input_type;
			type Output = #output_type;

			const NAME: &'static str = #fn_name;
			const MAX_RETRIES: usize = #max_retries;
			const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(#timeout);

			async fn run(#ctx_ident: #ctx_ty, #input_ident: &Self::Input) -> GlobalResult<Self::Output> {
				#fn_body
			}
		}
	};

	// eprintln!("\n\n{expanded}\n");

	TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn operation(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as OptionalIdent)
		.ident
		.map(|x| x.to_string())
		.unwrap_or_else(|| "Operation".to_string());
	let item_fn = parse_macro_input!(item as ItemFn);

	let config = match parse_config(&item_fn.attrs) {
		Ok(x) => x,
		Err(err) => return err.into_compile_error().into(),
	};

	let ctx_ty = syn::parse_str("&OperationCtx").unwrap();
	let TraitFnOutput {
		ctx_ident,
		input_ident,
		input_type,
		output_type,
	} = match parse_trait_fn(&ctx_ty, "Operation", &item_fn) {
		Ok(x) => x,
		Err(err) => return err,
	};

	let struct_ident = Ident::new(&name, proc_macro2::Span::call_site());
	let fn_name = item_fn.sig.ident.to_string();
	let fn_body = item_fn.block;
	let vis = item_fn.vis;

	let timeout = config.timeout;

	let expanded = quote! {
		#vis struct #struct_ident;

		impl chirp_workflow::operation::OperationInput for #input_type {
			type Operation = #struct_ident;
		}

		#[async_trait::async_trait]
		impl chirp_workflow::operation::Operation for #struct_ident {
			type Input = #input_type;
			type Output = #output_type;

			const NAME: &'static str = #fn_name;
			const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(#timeout);

			async fn run(#ctx_ident: #ctx_ty, #input_ident: &Self::Input) -> GlobalResult<Self::Output> {
				#fn_body
			}
		}
	};

	// eprintln!("\n\n{expanded}\n");

	TokenStream::from(expanded)
}

struct TraitFnOutput {
	ctx_ident: syn::Ident,
	input_ident: syn::Ident,
	input_type: Box<syn::Type>,
	output_type: syn::Type,
}

fn parse_trait_fn(
	ctx_ty: &syn::Type,
	trait_name: &str,
	item_fn: &syn::ItemFn,
) -> Result<TraitFnOutput, TokenStream> {
	// Check if is async
	if item_fn.sig.asyncness.is_none() {
		return Err(error(item_fn.sig.span(), "function must be async"));
	}

	let mut arg_names = vec![];
	let mut arg_types = vec![];

	for input in &item_fn.sig.inputs {
		if let syn::FnArg::Typed(arg) = input {
			match arg.pat.as_ref() {
				syn::Pat::Ident(ident) => {
					let arg_name = ident.ident.to_string();
					arg_names.push(arg_name);
					arg_types.push((*arg.ty).clone());
				}
				_ => {
					return Err(error(arg.pat.span(), "unsupported input parameter pattern"));
				}
			}
		} else {
			return Err(error(input.span(), "unsupported input parameter type"));
		}
	}

	if arg_types.len() != 2 || &arg_types[0] != ctx_ty {
		return Err(error(
			item_fn.sig.span(),
			&format!(
				"{} function must have exactly two parameters: ctx: {} and input: &YourInputType",
				trait_name,
				ctx_ty.to_token_stream().to_string()
			),
		));
	}

	let input_type = if let syn::Type::Reference(syn::TypeReference { elem, .. }) = &arg_types[1] {
		elem.clone()
	} else {
		return Err(error(arg_types[1].span(), "input type must be a reference"));
	};

	let output_type = match &item_fn.sig.output {
		ReturnType::Type(_, ty) => match ty.as_ref() {
			Type::Path(path) => {
				let segment = path.path.segments.last().unwrap();
				if segment.ident == "GlobalResult" {
					match &segment.arguments {
						PathArguments::AngleBracketed(args) => {
							if let Some(GenericArgument::Type(ty)) = args.args.first() {
								ty.clone()
							} else {
								return Err(error(args.span(), "unsupported Result type"));
							}
						}
						_ => {
							return Err(error(segment.arguments.span(), "unsupported Result type"))
						}
					}
				} else {
					return Err(error(
						path.span(),
						&format!("{} function must return a GlobalResult type", trait_name),
					));
				}
			}
			_ => return Err(error(ty.span(), "unsupported output type")),
		},
		_ => {
			return Err(error(
				item_fn.sig.output.span(),
				&format!("{} function must have a return type", trait_name),
			));
		}
	};

	Ok(TraitFnOutput {
		ctx_ident: Ident::new(&arg_names[0], proc_macro2::Span::call_site()),
		input_ident: Ident::new(&arg_names[1], proc_macro2::Span::call_site()),
		input_type,
		output_type,
	})
}

#[proc_macro_attribute]
pub fn signal(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as LitStr);
	if !name
		.value()
		.chars()
		.all(|c| c.is_ascii_lowercase() || c == '_')
	{
		return error(name.span(), "invalid signal name, must be [a-z_]");
	}

	let item = parse_macro_input!(item as Item);
	let (ident, attrs) = match item {
		Item::Struct(ref item_struct) => (&item_struct.ident, &item_struct.attrs),
		Item::Enum(ref item_enum) => (&item_enum.ident, &item_enum.attrs),
		_ => return error(item.span(), "expected struct or enum"),
	};

	// If also a message, don't derive serde traits
	let also_message = attrs
		.iter()
		.filter_map(|attr| attr.path().segments.last())
		.any(|seg| seg.ident == "message");
	let serde_derive = if also_message {
		quote! {}
	} else {
		quote! { #[derive(serde::Serialize, serde::Deserialize)] }
	};

	let expanded = quote! {
		#serde_derive
		#item

		impl chirp_workflow::signal::Signal for #ident {
			const NAME: &'static str = #name;
		}

		#[async_trait::async_trait]
		impl chirp_workflow::listen::Listen for #ident {
			async fn listen(ctx: &mut chirp_workflow::prelude::ListenCtx) -> chirp_workflow::prelude::WorkflowResult<Self> {
				let row = ctx.listen_any(&[<Self as chirp_workflow::signal::Signal>::NAME]).await?;
				Self::parse(&row.signal_name, &row.body)
			}

			fn parse(_name: &str, body: &serde_json::value::RawValue) -> chirp_workflow::prelude::WorkflowResult<Self> {
				serde_json::from_str(body.get()).map_err(WorkflowError::DeserializeSignalBody)
			}
		}
	};

	TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn message(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as LitStr);
	if !name
		.value()
		.chars()
		.all(|c| c.is_ascii_lowercase() || c == '_')
	{
		return error(name.span(), "invalid message name, must be [a-z_]");
	}

	let item_struct = parse_macro_input!(item as ItemStruct);

	// If also a signal, don't derive serde traits
	let also_signal = item_struct
		.attrs
		.iter()
		.filter_map(|attr| attr.path().segments.last())
		.any(|seg| seg.ident == "signal");
	let serde_derive = if also_signal {
		quote! {}
	} else {
		quote! { #[derive(serde::Serialize, serde::Deserialize)] }
	};

	let config = match parse_msg_config(&item_struct.attrs) {
		Ok(x) => x,
		Err(err) => return err.into_compile_error().into(),
	};

	let struct_ident = &item_struct.ident;
	let tail_ttl = config.tail_ttl;

	let expanded = quote! {
		#serde_derive
		#[derive(Debug)]
		#item_struct

		impl chirp_workflow::message::Message for #struct_ident {
			const NAME: &'static str = #name;
			const TAIL_TTL: std::time::Duration = std::time::Duration::from_secs(#tail_ttl);
		}
	};

	TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn workflow_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let input = syn::parse_macro_input!(item as syn::ItemFn);

	let test_ident = &input.sig.ident;
	let body = &input.block;

	// Check if is async
	if input.sig.asyncness.is_none() {
		return error(
			input.sig.span(),
			"the async keyword is missing from the function declaration",
		);
	}

	// Parse args
	let ctx_input = match input.sig.inputs.first().unwrap() {
		syn::FnArg::Receiver(recv) => {
			return error(recv.span(), "cannot have receiver argument");
		}
		syn::FnArg::Typed(input) => input,
	};
	let ctx_ident = match &*ctx_input.pat {
		syn::Pat::Ident(ident) => ident.ident.clone(),
		_ => {
			return error(ctx_input.span(), "expected identifier as argument");
		}
	};

	let result = quote! {
		#[test]
		fn #test_ident() {
			async fn test_body(#ctx_ident: chirp_workflow::prelude::TestCtx) {
				#body
			}

			// Build runtime
			let _ = chirp_workflow::prelude::__rivet_runtime::RunConfig {
				pretty_logs: true,
				..Default::default()
			}
			.run(
				chirp_workflow::prelude::tracing::Instrument::instrument(
					async move {
						// Build context
						let ctx = chirp_workflow::prelude::TestCtx::from_env(stringify!(#test_ident))
							.await;

						// Run test
						tracing::info!("test starting");
						test_body(ctx).await;
						tracing::info!("test finished");
					},
					chirp_workflow::prelude::tracing::info_span!(stringify!(#test_ident))
				)
			);
		}
	};

	result.into()
}

fn error(span: proc_macro2::Span, msg: &str) -> TokenStream {
	syn::Error::new(span, msg).to_compile_error().into()
}

fn parse_config(attrs: &[syn::Attribute]) -> syn::Result<Config> {
	let mut config = Config::default();

	for attr in attrs {
		let syn::Meta::NameValue(name_value) = &attr.meta else {
			continue;
		};

		let ident = name_value.path.require_ident()?;

		// Verify config property
		if ident == "max_retries" {
			config.max_retries =
				syn::parse::<syn::LitInt>(name_value.value.to_token_stream().into())?
					.base10_parse()?;
		} else if ident == "timeout" {
			config.timeout = syn::parse::<syn::LitInt>(name_value.value.to_token_stream().into())?
				.base10_parse()?;
		} else if ident != "doc" {
			return Err(syn::Error::new(
				ident.span(),
				format!("Unknown config property `{ident}`"),
			));
		}
	}

	Ok(config)
}

fn parse_msg_config(attrs: &[syn::Attribute]) -> syn::Result<MessageConfig> {
	let mut config = MessageConfig::default();

	for attr in attrs {
		let syn::Meta::NameValue(name_value) = &attr.meta else {
			continue;
		};

		let ident = name_value.path.require_ident()?;

		// Verify config property
		if ident == "tail_ttl" {
			config.tail_ttl = syn::parse::<syn::LitInt>(name_value.value.to_token_stream().into())?
				.base10_parse()?;
		} else if ident != "doc" {
			return Err(syn::Error::new(
				ident.span(),
				format!("Unknown config property `{ident}`"),
			));
		}
	}

	Ok(config)
}

fn parse_empty_config(attrs: &[syn::Attribute]) -> syn::Result<()> {
	for attr in attrs {
		let syn::Meta::NameValue(name_value) = &attr.meta else {
			continue;
		};

		let ident = name_value.path.require_ident()?;

		if ident != "doc" {
			return Err(syn::Error::new(
				ident.span(),
				format!("Unknown config property `{ident}`"),
			));
		}
	}

	Ok(())
}

struct OptionalIdent {
	ident: Option<Ident>,
}

impl Parse for OptionalIdent {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		if input.is_empty() {
			Ok(OptionalIdent { ident: None })
		} else {
			let ident: Ident = input.parse()?;
			Ok(OptionalIdent { ident: Some(ident) })
		}
	}
}
