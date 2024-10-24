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
pub fn operation(attr: TokenStream, item: TokenStream) -> TokenStream {
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
			"the handle function must have only one parameter",
		);
	}
	if fn_name != "handle" {
		return error(fn_name.span(), "operation function must be named `handle`");
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

	let result = quote! {
		// Used by op macro
		pub type __Request = #req_type;

		#[derive(Clone)]
		pub struct Operation;

		#[rivet_operation::prelude::async_trait]
		impl rivet_operation::Operation for Operation {
			type Request = #req_type;
			type Response = #res_type;

			const NAME: &'static str = #name;
			const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(#timeout);

			async fn handle(
				#req_ident: rivet_operation::OperationContext<Self::Request>,
			) -> GlobalResult<Self::Response> {
				#body
			}
		}
	};

	result.into()
}

fn error(span: proc_macro2::Span, msg: &str) -> proc_macro::TokenStream {
	syn::Error::new(span, msg).to_compile_error().into()
}
