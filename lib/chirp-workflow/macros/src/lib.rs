use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
	parse_macro_input, spanned::Spanned, GenericArgument, Ident, ItemFn, ItemStruct, LitStr,
	PathArguments, ReturnType, Type,
};

struct TraitFnOpts {
	ctx_ty: syn::Type,
	trait_ty: syn::Type,
	input_trait_ty: syn::Type,
	executable: bool,
}

fn trait_fn(attr: TokenStream, item: TokenStream, opts: TraitFnOpts) -> TokenStream {
	let activity_name = parse_macro_input!(attr as Ident).to_string();
	let item_fn = parse_macro_input!(item as ItemFn);
	let fn_name = item_fn.sig.ident.to_string();
	let fn_body = item_fn.block;

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

	if arg_types.len() != 2 || arg_types[0] != opts.ctx_ty {
		panic!(
			"{} function must have exactly two parameters: ctx: {:?} and input: YourInputType",
			opts.trait_ty.to_token_stream().to_string(),
			opts.ctx_ty.to_token_stream().to_string()
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
							if let Some(GenericArgument::Type(Type::Path(path))) = args.args.first()
							{
								&path.path.segments.last().unwrap().ident
							} else {
								panic!("Unsupported Result type");
							}
						}
						_ => panic!("Unsupported Result type"),
					}
				} else {
					panic!(
						"{} function must return a GlobalResult type",
						opts.trait_ty.to_token_stream().to_string()
					);
				}
			}
			_ => panic!("Unsupported output type"),
		},
		_ => panic!(
			"{} function must have a return type",
			opts.trait_ty.to_token_stream().to_string()
		),
	};

	let trait_ty = opts.trait_ty;
	let input_trait_ty = opts.input_trait_ty;
	let struct_ident = Ident::new(&activity_name, proc_macro2::Span::call_site());
	let ctx_ident = Ident::new(&arg_names[0], proc_macro2::Span::call_site());
	let ctx_ty = opts.ctx_ty;
	let input_ident = Ident::new(&arg_names[1], proc_macro2::Span::call_site());

	let exec_impl = if opts.executable {
		quote! {
			// NOTE: This would normally be an impl on the trait `ActivityInput` but this has conflicts with other
			// generic implementations on `Executable` so we implement executable on all of the input structs
			// instead
			#[async_trait::async_trait]
			impl chirp_workflow::prelude::Executable for #input_type {
				type Output = <#struct_ident as #trait_ty>::Output;

				async fn execute(self, ctx: &mut chirp_workflow::prelude::WorkflowCtx) -> GlobalResult<Self::Output> {
					ctx.activity(self).await
				}
			}
		}
	} else {
		quote! {}
	};

	let expanded = quote! {
		pub struct #struct_ident;

		impl #input_trait_ty for #input_type {
			type #trait_ty = #struct_ident;
		}

		#exec_impl

		#[async_trait::async_trait]
		impl chirp_workflow::prelude::#trait_ty for #struct_ident {
			type Input = #input_type;
			type Output = #output_type;

			fn name() -> &'static str {
				#fn_name
			}

			async fn run(#ctx_ident: #ctx_ty, #input_ident: &Self::Input) -> GlobalResult<Self::Output> {
				#fn_body
			}
		}
	};

	// eprintln!("\n\n{expanded}\n");

	TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn workflow(attr: TokenStream, item: TokenStream) -> TokenStream {
	trait_fn(
		attr,
		item,
		TraitFnOpts {
			ctx_ty: syn::parse_str("&mut WorkflowCtx").unwrap(),
			trait_ty: syn::parse_str("Workflow").unwrap(),
			input_trait_ty: syn::parse_str("chirp_workflow::workflow::WorkflowInput").unwrap(),
			executable: false,
		},
	)
}

#[proc_macro_attribute]
pub fn activity(attr: TokenStream, item: TokenStream) -> TokenStream {
	trait_fn(
		attr,
		item,
		TraitFnOpts {
			ctx_ty: syn::parse_str("&mut ActivityCtx").unwrap(),
			trait_ty: syn::parse_str("Activity").unwrap(),
			input_trait_ty: syn::parse_str("chirp_workflow::activity::ActivityInput").unwrap(),
			executable: true,
		},
	)
}

#[proc_macro_attribute]
pub fn operation(attr: TokenStream, item: TokenStream) -> TokenStream {
	trait_fn(
		attr,
		item,
		TraitFnOpts {
			ctx_ty: syn::parse_str("&mut OperationCtx").unwrap(),
			trait_ty: syn::parse_str("Operation").unwrap(),
			input_trait_ty: syn::parse_str("chirp_workflow::operation::OperationInput").unwrap(),
			executable: false,
		},
	)
}

#[proc_macro_attribute]
pub fn signal(attr: TokenStream, item: TokenStream) -> TokenStream {
	let signal_name = parse_macro_input!(attr as LitStr);
	let item_struct = parse_macro_input!(item as ItemStruct);

	let struct_ident = &item_struct.ident;

	let expanded = quote! {
		#[derive(serde::Serialize, serde::Deserialize)]
		#item_struct

		impl Signal for #struct_ident {
			fn name() -> &'static str {
				#signal_name
			}
		}

		#[::async_trait::async_trait]
		impl Listen for #struct_ident {
			async fn listen(ctx: &mut chirp_workflow::prelude::WorkflowCtx) -> chirp_workflow::prelude::WorkflowResult<Self> {
				let row = ctx.listen_any(&[Self::name()]).await?;
				Self::parse(&row.name, &row.body)
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
