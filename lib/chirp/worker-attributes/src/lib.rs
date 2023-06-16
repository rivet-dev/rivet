use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
	parse::{Parse, ParseStream},
	spanned::Spanned,
	Token,
};

struct Attr {
	name: Option<syn::Expr>,
	timeout: Option<syn::Expr>,
}

impl Parse for Attr {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut pairs = input
			.parse_terminated(syn::MetaNameValue::parse, Token![,])?
			.into_iter()
			.map(|kv| {
				Ok((
					kv.path
						.get_ident()
						.ok_or_else(|| syn::Error::new(kv.path.span(), "expected ident"))?
						.to_string(),
					kv.value,
				))
			})
			.collect::<Result<HashMap<_, _>, syn::Error>>()?;

		Ok(Attr {
			name: pairs.remove("name"),
			timeout: pairs.remove("timeout"),
		})
	}
}

#[proc_macro_attribute]
pub fn worker(attr: TokenStream, item: TokenStream) -> TokenStream {
	let attr = Into::<TokenStream2>::into(attr);
	let attr_span = attr.span();

	// Validate attribute args
	let args = match syn::parse2::<Attr>(attr) {
		Ok(args) => args,
		Err(err) => return err.to_compile_error().into(),
	};
	let name = if let Some(name) = args.name {
		name
	} else {
		return error(attr_span, "expected `name = ...` argument");
	};
	let timeout = args
		.timeout
		.as_ref()
		.map(ToTokens::to_token_stream)
		.unwrap_or_else(|| quote! { 60 });

	// Parse body
	let input = syn::parse_macro_input!(item as syn::ItemFn);

	// Validate signature
	let fn_name = &input.sig.ident;
	let body = &input.block;
	if input.sig.asyncness.is_none() {
		return error(
			input.sig.span(),
			"the async keyword is missing from the function declaration",
		);
	}
	if input.sig.inputs.len() != 1 {
		return error(
			input.sig.inputs.span(),
			"the worker function must have only one parameter",
		);
	}
	if fn_name != "worker" {
		return error(fn_name.span(), "worker function must be named `worker`");
	}

	// Derive req type
	let (req_ident, req_type): (&syn::Ident, &syn::Type) = match &input.sig.inputs[0] {
		syn::FnArg::Typed(pat) => {
			if !pat.attrs.is_empty() {
				return error(pat.span(), "attributes will be ignored");
			}

			// Extract identifier for the request
			let ident = match &*pat.pat {
				syn::Pat::Ident(ident) => &ident.ident,
				_ => {
					return error(pat.pat.span(), "invalid argument format");
				}
			};

			// Extract the inner request type
			let ty = match &*pat.ty {
				syn::Type::Path(path) => {
					let final_segment = path.path.segments.last().unwrap();
					if final_segment.ident != "OperationContext" {
						return error(
							final_segment.ident.span(),
							"argument must be a `OperationContext`",
						);
					}

					// Read the generic type
					match &final_segment.arguments {
						syn::PathArguments::AngleBracketed(args) => {
							if args.args.len() != 1 {
								return error(
									final_segment.span(),
									"must have exactly 1 generic argument",
								);
							}

							// Match the correct generic type
							match &args.args[0] {
								syn::GenericArgument::Type(ty) => ty,
								arg => {
									return error(arg.span(), "generic argument must be a type");
								}
							}
						}
						_ => {
							return error(final_segment.span(), "invalid generic args");
						}
					}
				}
				_ => {
					return error(pat.ty.span(), "unsupported type");
				}
			};

			(ident, ty)
		}
		_ => {
			return error(input.sig.inputs[0].span(), "invalid function argument");
		}
	};

	// Derive res type
	let res_type: &syn::Type = match &input.sig.output {
		syn::ReturnType::Type(_, ty) => {
			match &**ty {
				syn::Type::Path(path) => {
					let final_segment = path.path.segments.last().unwrap();
					if final_segment.ident != "Result" && final_segment.ident != "GlobalResult" {
						return error(
							final_segment.ident.span(),
							"return value must be a `Result`",
						);
					}

					// Read the generic type
					match &final_segment.arguments {
						syn::PathArguments::AngleBracketed(args) => {
							if final_segment.ident == "Result" && args.args.len() != 2 {
								return error(
									final_segment.span(),
									"must have exactly 2 generic arguments",
								);
							} else if final_segment.ident == "GlobalResult" && args.args.len() != 1
							{
								return error(
									final_segment.span(),
									"must have exactly 1 generic argument",
								);
							}

							// Match the correct generic type
							match &args.args[0] {
								syn::GenericArgument::Type(ty) => ty,
								arg => {
									return error(arg.span(), "generic argument must be a type");
								}
							}
						}
						_ => {
							return error(final_segment.span(), "invalid generic args");
						}
					}
				}
				_ => return error(ty.span(), "invalid return type"),
			}
		}
		_ => return error(input.sig.output.span(), "invalid return type"),
	};

	// Build main function that starts the manager with the worker
	let result = quote! {
		#[derive(Clone)]
		pub struct Worker;

		#[::chirp_worker::prelude::async_trait]
		impl ::chirp_worker::Worker for Worker {
			type Request = #req_type;
			type Response = #res_type;

			const NAME: &'static str = #name;
			const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(#timeout);

			async fn handle<'a>(
				&self,
				#req_ident: &::chirp_worker::prelude::OperationContext<Self::Request>,
			) -> GlobalResult<Self::Response>
			where
				Self::Response: 'a,
			{
				#body
			}
		}
	};

	result.into()
}

#[proc_macro_attribute]
pub fn worker_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
			async fn test_body(#ctx_ident: ::chirp_worker::TestCtx) {
				#body
			}

			// Build runtime
			let _ = ::chirp_worker::prelude::__rivet_runtime::RunConfig {
				pretty_logs: true,
				..Default::default()
			}
			.run(
				::chirp_worker::prelude::tracing::Instrument::instrument(
						async move {
						// Build context
						let ctx = ::chirp_worker::TestCtx::from_env(stringify!(#test_ident))
							.await
							.expect("create test context");

						// Run test
						tracing::info!("test starting");
						test_body(ctx).await;
						tracing::info!("test finished");
					},
					::chirp_worker::prelude::tracing::info_span!(stringify!(#test_ident))
				)
			);

			// // Log panic immediately
			// let rt = std::sync::Mutex::new(Some(rt));
			// let default_panic = std::panic::take_hook();
			// std::panic::set_hook(Box::new(move |info| {
			// 	tracing::error!(?info, "test panicked");
			// 	default_panic(info);

			// NOTE: This does some weird things with logging when there's a
			// panic
			// 	// Force the runtime to exit early
			// 	/*
			// 	rt
			// 		.lock()
			// 		.expect("failed to lock rt")
			// 		.take()
			// 		.expect("missing rt")
			// 		.shutdown_timeout(std::time::Duration::from_secs(1));
			// 	*/
			// }));
		}
	};

	result.into()
}

fn error(span: proc_macro2::Span, msg: &str) -> proc_macro::TokenStream {
	syn::Error::new(span, msg).to_compile_error().into()
}
