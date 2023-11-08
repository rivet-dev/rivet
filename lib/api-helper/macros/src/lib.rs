extern crate proc_macro;

use std::iter::FromIterator;

use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2, TokenTree};
use proc_macro_error::{emit_warning, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
	braced, bracketed, parenthesized,
	parse::{discouraged::Speculative, Parse, ParseStream},
	punctuated::Punctuated,
	spanned::Spanned,
	Token,
};

mod utils;
use utils::*;

// How to implement new endpoint arguments:
// 1. Add argument name to this list.
// 2. Implement macro code for argument. (See `MARK: Endpoint formatter` and `MARK: Simple argument parsing`)
const ENDPOINT_ARGUMENTS: &[&str] = &[
	"body",
	"body_as_bytes",
	"body_as_stream",
	"cookie",
	"raw_remote_addr",
	"opt_cookie",
	"header",
	"opt_auth",
	"not_using_cloudflare",
	"internal_endpoint",
	"query",
	"rate_limit",
	"with_response",
	"returns_bytes",
];

struct EndpointRouter {
	routes: Punctuated<Endpoint, Token![,]>,
	cors_config: Option<syn::Expr>,
	mounts: Punctuated<Mount, Token![,]>,
}

impl Parse for EndpointRouter {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut routes = None;
		let mut cors_config = None;
		let mut mounts = None;

		// Loop through all keys in the object
		loop {
			if input.is_empty() {
				break;
			}

			let key = input.parse::<syn::Ident>()?;
			input.parse::<Token![:]>()?;

			// Parse various keys
			match key.to_string().as_str() {
				"routes" => {
					if routes.is_none() {
						let routes_content;
						braced!(routes_content in input);
						routes = Some(routes_content.parse_terminated(Endpoint::parse)?);
					} else {
						return Err(syn::Error::new(
							key.span(),
							format!("Duplicate key `{}`.", key),
						));
					}
				}
				"cors" => {
					if cors_config.is_none() {
						cors_config = Some(input.parse()?);
					} else {
						return Err(syn::Error::new(
							key.span(),
							format!("Duplicate key `{}`.", key),
						));
					}
				}
				"mounts" => {
					if mounts.is_none() {
						let mounts_content;
						bracketed!(mounts_content in input);
						mounts = Some(mounts_content.parse_terminated(Mount::parse)?);
					} else {
						return Err(syn::Error::new(
							key.span(),
							format!("Duplicate key `{}`.", key),
						));
					}
				}
				_ => {
					return Err(syn::Error::new(
						key.span(),
						format!(
							"Unexpected key `{}`. Try `routes`, `cors`, or `mounts`.",
							key
						),
					));
				}
			}

			if input.is_empty() {
				break;
			}
			input.parse::<Token![,]>()?;
		}

		// Verify keys were set
		let routes = match routes {
			Some(routes) => routes,
			None => {
				return Err(syn::Error::new(
					input.span(),
					format!("Missing key `routes`."),
				));
			}
		};
		let mounts = mounts.unwrap_or_default();

		Ok(EndpointRouter {
			routes,
			cors_config,
			mounts,
		})
	}
}

impl EndpointRouter {
	fn render(self) -> syn::Result<TokenStream2> {
		let endpoints = self
			.routes
			.into_iter()
			.map(|endpoint| endpoint.render())
			.collect::<syn::Result<Vec<_>>>()?;

		let cors = self
			.cors_config
			.map(|cors_config| {
				quote! {
					lazy_static::lazy_static! {
						static ref CORS_CONFIG: api_helper::util::CorsConfig = #cors_config;
					}

					match api_helper::util::verify_cors(request, &*CORS_CONFIG)? {
						// Set headers and immediately return empty response
						api_helper::util::CorsResponse::Preflight(headers) => {
							response.headers_mut().map(|h| h.extend(headers));

							return Ok(Some(Vec::new()));
						}
						// Set headers, continue with request
						api_helper::util::CorsResponse::Regular(headers) => {
							response.headers_mut().map(|h| h.extend(headers));
						}
						// No CORS
						api_helper::util::CorsResponse::NoCors => {}
					}
				}
			})
			.unwrap_or_else(|| quote! {});

		let mounts = self
			.mounts
			.iter()
			.map(|mount| {
				let mount_path = &mount.path;
				let mount_prefix = match &mount.prefix {
					Some(prefix) => quote! { Some(#prefix) },
					None => quote! { None },
				};

				quote! {
					.try_or_else(|| async {
						#mount_path::__inner(
							shared_client.clone(),
							pools.clone(),
							cache.clone(),
							ray_id,
							request,
							response,
							#mount_prefix,
						)
						.await
						.map(std::convert::Into::into)
					}).await?
				}
			})
			.collect::<Vec<_>>();

		Ok(quote! {
			pub struct Router;
			impl Router {
				#[doc(hidden)]
				pub async fn __inner(
					shared_client: chirp_client::SharedClientHandle,
					pools: rivet_pools::Pools,
					cache: rivet_cache::Cache,
					ray_id: uuid::Uuid,
					mut request: &mut Request<Body>,
					response: &mut http::response::Builder,
					prefix: Option<&str>,
				) -> rivet_operation::prelude::GlobalResult<Option<Vec<u8>>> {
					use std::str::FromStr;
					use api_helper::macro_util::{self, __AsyncOption};
					
					// This url doesn't actually represent the url of the request, it's just put here so that the
					// URI can be parsed by url::Url::parse
					let __url = format!(
						"{}{}",
						rivet_operation::prelude::util::env::origin_api(), request.uri()
					);
					let __route = url::Url::parse(__url.as_str())?;

					// Create path segments list
					let __path_segments = if let Some(prefix) = prefix {
						let mut path_segments = match __route.path_segments() {
							Some(segments) => segments,
							None => return Ok(None),
						};

						// Parse prefix if set (only for nested routers)
						match path_segments.next() {
							Some(segment) if segment == prefix => {
								path_segments.collect::<Vec<_>>()
							},
							_ => return Ok(None),
						}
					} else {
						__route
							.path_segments()
							.map(|segments| segments.collect::<Vec<_>>())
							.unwrap_or_default()
						};

					// Cors is handled after path segments are created so that we are sure that we are in
					// the correct mount (if nested routers)
					#cors

					let __body = __AsyncOption::None
						#(#endpoints)*
						#(#mounts)*
					;

					Ok(__body.into())
				}

				pub async fn handle(
					shared_client: chirp_client::SharedClientHandle,
					pools: rivet_pools::Pools,
					cache: rivet_cache::Cache,
					ray_id: uuid::Uuid,
					mut request: Request<Body>,
					mut response: http::response::Builder,
				) -> Result<Response<Body>, http::Error> {
					tracing::info!(method=?request.method(), uri=?request.uri(), "received request");

					// If `None`, no route was found
					let res =
						match Self::__inner(
							shared_client, pools, cache, ray_id, &mut request, &mut response, None,
						).await {
							Ok(body) => {
								body.ok_or_else(|| {
									tracing::debug!("not found: {:?}", request.uri().to_string());

									rivet_operation::prelude::GlobalError::bad_request(
										rivet_operation::prelude::formatted_error::code::API_NOT_FOUND
									)
								})
							}
							Err(err) => Err(err),
						};

					// Set JSON headers
					if let Some(mut headers) = response.headers_mut() {
						headers.insert(
							http::header::CONTENT_TYPE,
							http::HeaderValue::from_static("application/json")
						);
					}

					// Convert to hyper response
					match res {
						Ok(body) => {
							Ok(response.body(hyper::Body::from(body))?)
						},
						Err(err) => Ok(api_helper::error::handle_rejection(err, response, ray_id)?),
					}
				}
			}
		})
	}
}

/// Structure of a single router mount in the `define_router!` macro:
/// ```rust
/// {
/// 	path: api_cloud::routes::Router,
/// 	prefix: "cloud",
/// }
/// ```
struct Mount {
	path: syn::TypePath,
	prefix: Option<syn::LitStr>,
}

impl Parse for Mount {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let content;
		braced!(content in input);

		let mut path = None;
		let mut prefix = None;

		// Loop through all keys in the object
		loop {
			if content.is_empty() {
				break;
			}

			let key = content.parse::<syn::Ident>()?;
			content.parse::<Token![:]>()?;

			// Parse various keys
			match key.to_string().as_str() {
				"path" => {
					if path.is_none() {
						path = Some(content.parse::<syn::TypePath>()?);
					} else {
						return Err(syn::Error::new(
							key.span(),
							format!("Duplicate key `{}`.", key),
						));
					}
				}
				"prefix" => {
					if prefix.is_none() {
						prefix = Some(content.parse::<syn::LitStr>()?);
					} else {
						return Err(syn::Error::new(
							key.span(),
							format!("Duplicate key `{}`.", key),
						));
					}
				}
				_ => {
					return Err(syn::Error::new(
						key.span(),
						format!("Unexpected key `{}`. Try `path` or `prefix`.", key),
					));
				}
			}

			if content.is_empty() {
				break;
			}
			content.parse::<Token![,]>()?;
		}

		// Verify keys were set
		let path = match path {
			Some(path) => path,
			None => {
				return Err(syn::Error::new(
					input.span(),
					format!("Missing key `path`."),
				));
			}
		};

		Ok(Mount { path, prefix })
	}
}

struct RequestPath {
	segments: Punctuated<RequestPathSegment, Token![/]>,
}

impl Parse for RequestPath {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		Ok(RequestPath {
			segments: input.parse_terminated(RequestPathSegment::parse)?,
		})
	}
}

#[derive(Debug)]
enum RequestPathSegment {
	LitStr(syn::LitStr),
	Type(syn::Type),
}

impl Parse for RequestPathSegment {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let fork = input.fork();

		if let Ok(lit) = fork.parse::<syn::LitStr>() {
			input.advance_to(&fork);

			Ok(RequestPathSegment::LitStr(lit))
		} else {
			Ok(RequestPathSegment::Type(input.parse()?))
		}
	}
}

impl RequestPathSegment {
	fn render(self, i: usize, arg_count: &mut u32) -> TokenStream2 {
		match self {
			// Deserialize literal path segment
			RequestPathSegment::LitStr(lit) => {
				quote! {
					match __path_segments.next() {
						Some(segment) if segment == & #lit => (),
						_ => return Ok(__AsyncOption::None),
					}
				}
			}
			// Deserialize path segment into a given type
			RequestPathSegment::Type(alias) => {
				let alias_label = alias.to_token_stream().to_string();
				let arg_name = format_ident!("arg_{}", arg_count);
				*arg_count += 1;

				quote! {
					let #arg_name = if let Some(segment) = __path_segments.next() {
						if let Ok(de_segment) = #alias::from_str(segment) {
							de_segment
						} else {
							tracing::debug!("cannot deserialize path segment {} into `{}`", #i, #alias_label);
							return Ok(__AsyncOption::None);
						}
					}
					else {
						return Ok(__AsyncOption::None);
					};
				}
			}
		}
	}
}

/// Structure of a single endpoint block in the `define_router!` macro:
/// ```rust
/// "test" / "endpoint": {
/// 	GET: test::endpoint(),
/// }
/// ```
struct Endpoint {
	/// Path of endpoint.
	path_segments: Punctuated<RequestPathSegment, Token![/]>,
	/// Each `EndpointFunction` in the bracketed scope of this endpoint block.
	functions: Punctuated<EndpointFunction, Token![,]>,
}

impl Parse for Endpoint {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		// Collect path. Stepping is required to check for a colon after the path
		let path_segments = input.step(|cursor| {
			let mut tts = Vec::new();
			let mut rest = *cursor;

			while let Some((tt, next)) = rest.token_tree() {
				match &tt {
					TokenTree::Punct(punct) if punct.as_char() == ':' => {
						return Ok((tts.into_iter().collect::<TokenStream2>(), next));
					}
					_ => {
						tts.push(tt);
						rest = next;
					}
				}
			}

			Err(cursor.error("expected `:` after path"))
		})?;
		let path_segments = TokenStream::from(TokenStream2::from_iter(path_segments));
		let path_segments = syn::parse::<RequestPath>(path_segments)?.segments;

		let content;
		braced!(content in input);

		Ok(Endpoint {
			path_segments,
			functions: content.parse_terminated(EndpointFunction::parse)?,
		})
	}
}

impl Endpoint {
	fn render(self) -> syn::Result<TokenStream2> {
		// Generate a path to use for the metrics
		//
		// This path can't contain the actual variables from the real
		// request in order to maintain a low carnality.
		let metrics_path_str = self
			.path_segments
			.iter()
			.map(|segment| match segment {
				RequestPathSegment::LitStr(lit) => lit.value(),
				RequestPathSegment::Type(_) => "{}".to_string(),
			})
			.collect::<Vec<String>>()
			.join("/");
		let metrics_path_str = format!("/{metrics_path_str}");
		let metrics_path = Literal::string(&metrics_path_str);

		let mut arg_count = 0u32;
		let segment_parsing = self
			.path_segments
			.into_iter()
			.enumerate()
			.map(|(i, segment)| segment.render(i, &mut arg_count))
			.collect::<Vec<_>>();

		// Make argument names for typed path segments
		let arg_list = (0..arg_count)
			.map(|i| format_ident!("arg_{}", i).to_token_stream())
			.collect::<Vec<_>>();

		let allowed_methods = self
			.functions
			.iter()
			.map(|func| func.req_type.as_ref())
			.collect::<Vec<_>>()
			.join(", ");

		// MARK: Endpoint formatter
		let arms = self
			.functions
			.into_iter()
			.map(|func| func.render(arg_list.clone(), metrics_path.clone()))
			.collect::<syn::Result<Vec<_>>>()?;

		Ok(quote! {
			.try_or_else(|| async {
				let mut __path_segments = __path_segments.iter();
				#(#segment_parsing)*

				if __path_segments.next().is_none() {
					match request.method() {
						#(#arms)*
						_ => {
							Err(rivet_operation::prelude::err_code!(
								API_METHOD_NOT_ALLOWED,
								allowed_methods = #allowed_methods
							))
						}
					}
				} else {
					Ok(__AsyncOption::None)
				}
			}).await?
		})
	}
}

/// Structure of a single endpoint function inside of an endpoint block (`Endpoint` struct)
/// in the `define_router!` macro:
/// ```rust
/// GET: test::endpoint(query: test::TestQuery)
/// ```
struct EndpointFunction {
	/// Path of the given Rust function.
	path: syn::Path,
	/// Request type of the endpoint. (GET, PUT, POST, etc)
	req_type: String,
	/// The request `body` type.
	body: Option<syn::Expr>,
	/// Any other arguments besides the `body` type. (rate limit, queries, etc)
	args: Vec<EndpointArg>,
}

impl Parse for EndpointFunction {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let req_type_ident = input.parse::<syn::Ident>()?;
		let req_type = req_type_ident.to_string();

		match req_type.as_str() {
			"GET" | "POST" | "PUT" | "DELETE" => {}
			_ => {
				return Err(syn::Error::new(
					req_type_ident.span(),
					"Invalid endpoint request type (try GET, POST, PUT, DELETE)",
				));
			}
		};

		let _colon = input.parse::<Token![:]>()?;

		let mut path = input.parse::<syn::Path>()?;

		// Remove arguments from last path segment
		let last_segment = path.segments.pop().unwrap();
		path.segments.push(syn::PathSegment {
			ident: last_segment.value().ident.clone(),
			arguments: syn::PathArguments::None,
		});

		// Collect arguments from function call
		let content;
		let args_tt = parenthesized!(content in input);
		let args: Punctuated<EndpointArg, Token![,]> =
			content.parse_terminated(EndpointArg::parse)?;

		// Check for body
		let (mut bodies, args) = args
			.into_iter()
			.partition::<Vec<_>, _>(|arg| arg.label.to_string() == "body");
		let body = bodies
			.pop()
			.map(|arg| arg.value.expect_expr().cloned())
			.transpose()?;

		// Make sure body is set for post and put requests
		if body.is_none()
			&& !args
				.iter()
				.any(|arg| arg.label == "body_as_bytes" || arg.label == "body_as_stream")
		{
			if req_type != "GET" && req_type != "DELETE" {
				return Err(syn::Error::new(
					args_tt.span,
					"POST and PUT endpoints must have a body argument",
				));
			}
		} else if req_type == "DELETE" || req_type == "GET" {
			emit_warning!(
				args_tt.span,
				"GET and DELETE endpoints should not have a body argument",
			);
		}

		Ok(EndpointFunction {
			path,
			req_type,
			body,
			args,
		})
	}
}

impl EndpointFunction {
	fn render(
		self,
		mut arg_list: Vec<TokenStream2>,
		metrics_path: Literal,
	) -> syn::Result<TokenStream2> {
		let req_type = format_ident!("{}", self.req_type);
		let path = self.path;

		let metrics_method = Literal::string(&self.req_type);

		// Check if response builder is to be included
		if self
			.args
			.iter()
			.find(|arg| arg.label == "with_response")
			.is_some()
		{
			arg_list.insert(0, quote! { response });
		}

		// Get json body or anchor body
		let json_or_anchor_body = if let Some(body_type) = self.body {
			arg_list.push(format_ident!("body").to_token_stream());

			quote! {
				let body = macro_util::__deserialize_body::<#body_type>(&mut request).await?;
			}
		} else if self.args.iter().any(|arg| arg.label == "body_as_bytes") {
			arg_list.push(format_ident!("body").to_token_stream());

			quote! {
				let body = macro_util::__read_body_bytes(&mut request).await?;
			}
		} else if self.args.iter().any(|arg| arg.label == "body_as_stream") {
			arg_list.push(format_ident!("body").to_token_stream());

			quote! {
				macro_util::__validate_body(&mut request)?;
				let body = request.body_mut();
			}
		} else if self.req_type == "GET" {
			arg_list.push(format_ident!("query").to_token_stream());

			quote! {
				let query = macro_util::__deserialize_query::<api_helper::anchor::WatchIndexQuery>(&__route)?;
				has_watch_index = query.has_watch_index();
			}
		} else {
			quote! {}
		};

		// Parse rate limit
		let rate_limit = {
			let default_key = default_rate_limit_key(&path);
			let default_bucket = default_rate_limit_bucket(self.req_type);

			if let Some(rate_limit_rate) = self.args.iter().find(|arg| arg.label == "rate_limit") {
				// NOTE: Re-parse generic block into typed map
				let value = rate_limit_rate.value.expect_block()?;
				let args =
					re_parse::<SimpleMap<AnyArg<ExprTree, SimpleMap<syn::Expr>, ExprTree>>>(value)?;

				// Parse rate limit key
				let key = if let Some(value) = args.get("key") {
					let key = &value.expect_expr()?;
					quote! { #key }
				} else {
					quote! { #default_key }
				};

				// Parse rate limit buckets
				let buckets = if let Some(value) = args.get("buckets") {
					// Make sure there's at least 1 bucket
					value
						.expect_expr()
						.ok()
						.and_then(|expr| {
							if let syn::Expr::Array(arr) = expr {
								arr.elems.is_empty().then(|| {
									Err::<(), _>(syn::Error::new(
										expr.span(),
										"Must have at least 1 bucket",
									))
								})
							} else {
								None
							}
						})
						.transpose()?;

					let buckets = value.expect_bracket().map_err(|err| {
						syn::Error::new(
							err.span(),
							"Invalid syntax, expected array of rate limit configs",
						)
					})?;

					// Validate and format buckets
					let parsed_buckets = buckets
						.iter()
						.map(|bucket| {
							let count = bucket.get("count").ok_or_else(|| {
								syn::Error::new(bucket.span(), "Expected `count` property")
							})?;
							let bucket_duration =
								if let Some(bucket_duration) = bucket.get("duration") {
									quote! { #bucket_duration }
								} else {
									let bucket_duration = default_rate_limit_bucket_duration();
									quote! { #bucket_duration }
								};

							// TODO: Error on unknown properties

							Ok(quote! {
								rivet_cache::RateLimitBucketConfig {
									count: #count,
									bucket_duration_ms: #bucket_duration,
								}
							})
						})
						.collect::<syn::Result<Vec<_>>>()?;

					quote! { vec![ #(#parsed_buckets),* ] }
				} else {
					quote! { vec![ #default_bucket ] }
				};

				quote! {
					rivet_cache::RateLimitConfig {
						key: format!("{}-{}", rivet_operation::prelude::util::env::chirp_service_name(), #key),
						buckets: #buckets,
					}
				}
			} else {
				quote! {
					rivet_cache::RateLimitConfig {
						key: format!("{}{}", rivet_operation::prelude::util::env::chirp_service_name(), #default_key),
						buckets: vec![ #default_bucket ],
					}
				}
			}
		};

		// Get optional auth value or default
		let opt_auth = if let Some(opt_auth) = self.args.iter().find(|arg| arg.label == "opt_auth")
		{
			let value = opt_auth.value.expect_expr()?;

			quote! { #value }
		} else {
			quote! { false }
		};

		// TODO: Combine not_using_cloudflare and internal_endpoint in to an enum
		// If this endpoint is not proxied behind Cloudflare
		let not_using_cloudflare = if let Some(not_using_cloudflare) = self
			.args
			.iter()
			.find(|arg| arg.label == "not_using_cloudflare")
		{
			let value = not_using_cloudflare.value.expect_expr()?;

			quote! { #value }
		} else {
			quote! { false }
		};

		// If this endpoint is accessed directly from other services
		let internal_endpoint = if let Some(internal_endpoint) = self
			.args
			.iter()
			.find(|arg| arg.label == "internal_endpoint")
		{
			let value = internal_endpoint.value.expect_expr()?;

			quote! { #value }
		} else {
			quote! { false }
		};

		// Returns the bytes directly instead of serializing them with serde_json
		let response_body = if let Some(returns_bytes) =
			self.args.iter().find(|arg| arg.label == "returns_bytes")
		{
			let value = returns_bytes.value.expect_expr()?;
			if let syn::Expr::Lit(syn::ExprLit {
				lit: syn::Lit::Bool(syn::LitBool { value, .. }),
				..
			}) = value
			{
				if *value {
					quote! { body }
				} else {
					quote! { serde_json::to_vec(&body)? }
				}
			} else {
				return Err(syn::Error::new(value.span(), "Expected boolean"));
			}
		} else {
			quote! { serde_json::to_vec(&body)? }
		};

		// Collect arg lines
		// MARK: Simple argument parsing
		let args = self
			.args
			.into_iter()
			.enumerate()
			.map(|(i, arg)| {
				if arg.label == "query" {
					let value = arg.value.expect_expr()?;

					let arg_name = format_ident!("{}_{}", arg.label, i);
					arg_list.push(arg_name.to_token_stream());

					Ok(quote! {
						let #arg_name = macro_util::__deserialize_query::<#value>(&__route)?;
					})
				} else if arg.label == "opt_cookie" {
					let value = arg.value.expect_expr()?;

					let arg_name = format_ident!("{}_{}", arg.label, i);
					arg_list.push(arg_name.to_token_stream());

					let associated_type = arg
						.associated_type
						.map(|t| quote! { #t })
						.unwrap_or_else(|| quote! { _ });

					Ok(quote! {
						let #arg_name = macro_util::__deserialize_optional_cookie::<#associated_type>(&request, #value)?;
					})
				} else if arg.label == "cookie" {
					let value = arg.value.expect_expr()?;

					let arg_name = format_ident!("{}_{}", arg.label, i);
					arg_list.push(arg_name.to_token_stream());

					let associated_type = arg
						.associated_type
						.map(|t| quote! { #t })
						.unwrap_or_else(|| quote! { _ });

					Ok(quote! {
						let #arg_name = macro_util::__deserialize_cookie::<#associated_type>(&request, #value)?;
					})
				} else if arg.label == "header" {
					let value = arg.value.expect_expr()?;

					let arg_name = format_ident!("{}_{}", arg.label, i);
					arg_list.push(arg_name.to_token_stream());

					let associated_type = arg
						.associated_type
						.map(|t| quote! { #t })
						.unwrap_or_else(|| quote! { _ });

					Ok(quote! {
						let #arg_name = macro_util::__deserialize_header::<#associated_type, _>(&request, #value)?;
					})
				} else {
					// NOTE: This scope is not unreachable, certain arguments such as body or rate limit are
					// extracted outside of this loop and are to be skipped.
					Ok(quote! {})
				}
			})
			.collect::<syn::Result<Vec<_>>>()?;

		// Format single endpoint
		Ok(quote! {
			&http::Method::#req_type => {
				#(#args)*

				let ctx = macro_util::__with_ctx(
					shared_client.clone(),
					pools.clone(),
					cache.clone(),
					&request,
					ray_id,
					#opt_auth,
					#not_using_cloudflare,
					#internal_endpoint,
					#rate_limit,
				).await?;

				let mut has_watch_index = false;
				#json_or_anchor_body

				// Pre-request metrics
				let start = std::time::Instant::now();
				let metrics_watch = if has_watch_index {
					"1"
				} else {
					"0"
				};
				macro_util::__metrics::API_REQUEST_PENDING
					.with_label_values(&[#metrics_method, #metrics_path, metrics_watch])
					.inc();
				macro_util::__metrics::API_REQUEST_TOTAL
					.with_label_values(&[#metrics_method, #metrics_path, metrics_watch])
					.inc();

				// Decrement pending when dropped. We have to
				// spawn a task for this because the decrement
				// won't run if the future is dropped.
				let (metrics_complete_tx, metrics_complete_rx) =
					tokio::sync::oneshot::channel::<(http::StatusCode, String)>();
				tokio::task::Builder::new()
					.name("api_helper::req_metrics_drop")
					.spawn(async move {
						// Wait for the request to complete or be
						// cancelled
						let complete_msg = metrics_complete_rx.await;
						let (http_status, err_code) = match &complete_msg {
							Ok((http_status, err_code)) => (http_status.as_str(), err_code.as_str()),
							Err(_) => ("", "API_CANCELLED"),
						};

						// Update metrics
						let duration = start.elapsed().as_secs_f64();
						macro_util::__metrics::API_REQUEST_PENDING
							.with_label_values(&[
								#metrics_method,
								#metrics_path,
								metrics_watch
							])
							.dec();
						macro_util::__metrics::API_REQUEST_DURATION
							.with_label_values(&[
								#metrics_method,
								#metrics_path,
								metrics_watch,
								http_status,
								err_code,
							])
							.observe(duration);
						if !err_code.is_empty() {
							macro_util::__metrics::API_REQUEST_ERRORS
								.with_label_values(&[
									#metrics_method,
									#metrics_path,
									metrics_watch,
									http_status,
									err_code,
								])
								.inc();
						}
					})?;

				let (response, http_status, err_code) = match #path(ctx, #(#arg_list),*).await {
					Ok(body) => {
						(
							Ok(__AsyncOption::Some(
								#response_body
							)),
							http::StatusCode::OK,
							String::new(),
						)
					}
					Err(err) => {
						let http_status = err.http_status();
						let err_code = err.code().unwrap_or("?").to_string();
						(Err(err), http_status, err_code)
					}
				};

				// Complete metrics span
				if let Err(_) = metrics_complete_tx.send((http_status, err_code)) {
					tracing::warn!("metrics complete receiver dropped");
				}

				response
			},
		})
	}
}

// TODO: Replace with SimpleMap
/// A single argument inside of the `EndpointFunction` call body.
/// ```rust
/// query: test::TestQuery
/// ```
/// ### With associated type
/// ```rust
/// header: <SocketAddr>("host")
/// ```
struct EndpointArg {
	label: syn::Ident,
	associated_type: Option<syn::Path>,
	value: ExprTree,
}

impl Parse for EndpointArg {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let label = input.parse::<syn::Ident>()?;

		if !ENDPOINT_ARGUMENTS.iter().any(|arg| label == arg) {
			return Err(syn::Error::new(
				label.span(),
				format!(
					"Invalid request argument `{}` (allowed: {})",
					label,
					ENDPOINT_ARGUMENTS.join(", ")
				),
			));
		}

		let _colon = input.parse::<Token![:]>()?;
		let (associated_type, value) =
			if let Ok(generics) = input.parse::<syn::AngleBracketedGenericArguments>() {
				let args = generics.args;

				if args.len() > 1 {
					return Err(syn::Error::new(
						args.span(),
						"Only one endpoint argument type parameter is allowed",
					));
				}

				let associated_type =
					if let Some(syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
						path,
						..
					}))) = args.first()
					{
						path
					} else {
						return Err(syn::Error::new(
							args.span(),
							"Invalid endpoint argument type parameter",
						));
					};

				let content;
				parenthesized!(content in input);

				(Some(associated_type.clone()), content.parse::<ExprTree>()?)
			} else {
				(None, input.parse::<ExprTree>()?)
			};

		Ok(EndpointArg {
			label,
			associated_type,
			value,
		})
	}
}

/// Create a set of hyper routes for any given endpoints.
/// # Example:
/// ```rust
///
/// define_router! {
/// 	ctx: [shared_client, pools, cache, ray_id, request, &mut response],
/// 	routes: {
/// 		"test" / "endpoint": {
/// 			GET: tests::function(rate_limit: 32),
/// 		},
/// 		"test" / Uuid: {
/// 			POST: test_function(body: TestRequest),
/// 			PUT: test_function(body: TestRequest),
/// 		},
/// 	},
/// }
/// ```
///
/// *NOTE: This automatically generates a rate limit key based on the path of the
/// endpoint function and the api service name. (Ex: `api-portal-tests-function`)*
#[proc_macro_error]
#[proc_macro]
pub fn define_router(item: TokenStream) -> TokenStream {
	let data = syn::parse_macro_input!(item as EndpointRouter);

	match data.render() {
		Ok(result) => {
			// println!("{}", result);
			result.into()
		}
		Err(err) => err.to_compile_error().into(),
	}
}
