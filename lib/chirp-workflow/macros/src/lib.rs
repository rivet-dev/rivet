use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	spanned::Spanned,
	GenericArgument, Ident, ItemFn, ItemStruct, LitStr, PathArguments, ReturnType, Type,
};

struct Config {
	max_retries: u32,
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
	} = parse_trait_fn(&ctx_ty, "Workflow", &item_fn);

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
		impl chirp_workflow::prelude::Workflow for #struct_ident {
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
	} = parse_trait_fn(&ctx_ty, "Activity", &item_fn);

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

		// NOTE: This would normally be an impl on the trait `ActivityInput` but this has conflicts with other
		// generic implementations on `Executable` so we implement executable on all of the input structs
		// instead
		#[async_trait::async_trait]
		impl chirp_workflow::prelude::Executable for #input_type {
			type Output = <#struct_ident as chirp_workflow::prelude::Activity>::Output;

			async fn execute(self, ctx: &mut chirp_workflow::prelude::WorkflowCtx) -> GlobalResult<Self::Output> {
				ctx.activity(self).await
			}
		}

		#[async_trait::async_trait]
		impl chirp_workflow::prelude::Activity for #struct_ident {
			type Input = #input_type;
			type Output = #output_type;

			const NAME: &'static str = #fn_name;
			const MAX_RETRIES: u32 = #max_retries;
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
	} = parse_trait_fn(&ctx_ty, "Operation", &item_fn);

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
		impl chirp_workflow::prelude::Operation for #struct_ident {
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

fn parse_trait_fn(ctx_ty: &syn::Type, trait_name: &str, item_fn: &syn::ItemFn) -> TraitFnOutput {
	// Check if is async
	if item_fn.sig.asyncness.is_none() {
		panic!("the async keyword is missing from the function declaration");
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
				_ => panic!("Unsupported input parameter pattern"),
			}
		} else {
			panic!("Unsupported input parameter type");
		}
	}

	if arg_types.len() != 2 || &arg_types[0] != ctx_ty {
		panic!(
			"{} function must have exactly two parameters: ctx: {:?} and input: YourInputType",
			trait_name,
			ctx_ty.to_token_stream().to_string(),
		);
	}

	let input_type = if let syn::Type::Reference(syn::TypeReference { elem, .. }) = &arg_types[1] {
		elem.clone()
	} else {
		panic!("Input type must be a reference");
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
								panic!("Unsupported Result type");
							}
						}
						_ => panic!("Unsupported Result type"),
					}
				} else {
					panic!("{} function must return a GlobalResult type", trait_name,);
				}
			}
			_ => panic!("Unsupported output type"),
		},
		_ => panic!("{} function must have a return type", trait_name),
	};

	TraitFnOutput {
		ctx_ident: Ident::new(&arg_names[0], proc_macro2::Span::call_site()),
		input_ident: Ident::new(&arg_names[1], proc_macro2::Span::call_site()),
		input_type,
		output_type,
	}
}

#[proc_macro_attribute]
pub fn signal(attr: TokenStream, item: TokenStream) -> TokenStream {
	let name = parse_macro_input!(attr as LitStr);
	let item_struct = parse_macro_input!(item as ItemStruct);

	let struct_ident = &item_struct.ident;

	let expanded = quote! {
		#[derive(serde::Serialize, serde::Deserialize)]
		#item_struct

		impl Signal for #struct_ident {
			const NAME: &'static str = #name;
		}

		#[async_trait::async_trait]
		impl Listen for #struct_ident {
			async fn listen(ctx: &mut chirp_workflow::prelude::WorkflowCtx) -> chirp_workflow::prelude::WorkflowResult<Self> {
				let row = ctx.listen_any(&[Self::NAME]).await?;
				Self::parse(&row.signal_name, row.body)
			}

			fn parse(_name: &str, body: serde_json::Value) -> chirp_workflow::prelude::WorkflowResult<Self> {
				serde_json::from_value(body).map_err(WorkflowError::DeserializeActivityOutput)
			}
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

fn error(span: proc_macro2::Span, msg: &str) -> proc_macro::TokenStream {
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
		} else {
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

		return Err(syn::Error::new(
			ident.span(),
			format!("Unknown config property `{ident}`"),
		));
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
