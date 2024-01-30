extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

use syn::parse::{Parse, ParseStream};
use syn::token::Return;
use syn::{braced, bracketed, Expr, Ident, Result, Token};

/// Represented in seconds.
///
/// See docs/infrastructure/TIMEOUTS.md for reasoning.
const DEFAULT_TIMEOUT: u64 = 40 * 1000;

mod kw {
	syn::custom_keyword!(JITTER);
}

#[derive(Debug)]
struct SelectWithTimeout {
	timeout: Option<(Expr, u64)>,
	jitter: Option<(Expr, u64)>,
	returning: Option<Expr>,
	rest: TokenStream2,
}

impl Parse for SelectWithTimeout {
	fn parse(input: ParseStream) -> Result<Self> {
		// Check if a bracket block exists
		let header_exists = input.fork().step(|cursor| {
			if let Some((_, _, rest)) = cursor.group(proc_macro2::Delimiter::Bracket) {
				Ok((true, rest))
			} else {
				Ok((false, *cursor))
			}
		})?;

		let (timeout, jitter, returning) = if header_exists {
			// Collect content in brackets
			let content;
			bracketed!(content in input);

			// Collect timeout and jitter
			let (timeout, jitter) = if content.fork().parse::<Expr>().is_ok() {
				// Get timeout config
				let timeout = content.parse::<Expr>()?;
				let timeout_unit = content.parse::<Ident>()?;

				// Check if first config is jitter
				if content.peek(kw::JITTER) {
					let _jitter_keyword = content.parse::<kw::JITTER>()?;

					// Collect jitter
					let jitter_scale = time_scale(timeout_unit.to_string().as_str())
						.ok_or_else(|| content.error("Invalid jitter unit"))?;

					(None, Some((timeout, jitter_scale)))
				} else {
					// Collect timeout
					let timeout_scale = time_scale(timeout_unit.to_string().as_str())
						.ok_or_else(|| content.error("Invalid timeout unit"))?;

					// Collect jitter if exists
					let jitter = if !content.is_empty() {
						let _comma = content.parse::<Token![,]>()?;

						if content.peek3(kw::JITTER) {
							// Collect jitter amount
							let jitter = content.parse::<Expr>()?;
							let jitter_scale =
								time_scale(content.parse::<Ident>()?.to_string().as_str())
									.ok_or_else(|| content.error("Invalid jitter unit"))?;

							let _jitter_keyword = content.parse::<kw::JITTER>()?;

							Some((jitter, jitter_scale))
						} else {
							None
						}
					} else {
						None
					};

					(Some((timeout, timeout_scale)), jitter)
				}
			} else {
				(None, None)
			};

			// Collect return expression
			let returning = if !content.is_empty() {
				if timeout.is_some() || jitter.is_some() {
					let _comma = content.parse::<Token![,]>()?;
				}

				let _return = content.parse::<Return>()?;

				Some(content.parse::<Expr>()?)
			} else {
				None
			};

			(timeout, jitter, returning)
		} else {
			(None, None, None)
		};

		// Collect rest of tokens in braces
		let content;
		braced!(content in input);
		let rest = content.parse::<TokenStream2>()?;

		Ok(SelectWithTimeout {
			timeout,
			jitter,
			returning,
			rest,
		})
	}
}

fn time_scale(unit: &str) -> Option<u64> {
	match unit {
		"MS" | "MSEC" | "MILLIS" => Some(1),
		"SEC" => Some(1000),
		"MIN" => Some(1000 * 60),
		"HR" | "HOUR" | "HOURS" => Some(1000 * 60 * 60),
		_ => None,
	}
}

/// Wraps `tokio::select!` content and gives it a timeout arm.
///
/// Accepts three optional config properties in header array: timeout, jitter, return expression. (order preserved)
///
/// # Examples
///
/// ## With header config
/// ```rust
/// use rivet_util as util;
///
/// util::macros::select_with_timeout!([10 MIN, 3 SEC JITTER, return None] {
/// 	event = update_sub.next() => {
/// 		let _event = event?;
///
///			// Perform some operation
///
///			Some(...)
/// 	}
/// });
/// ```
///
/// ## No header config
/// Defaults: timeout 1 minute, no jitter, return type `Default::default()`
/// ```rust
/// use rivet_util as util;
///
/// util::macros::select_with_timeout!({
/// 	event = update_sub.next() => {
/// 		let _event = event?;
/// 	}
/// });
/// ```
#[proc_macro]
pub fn select_with_timeout(item: TokenStream) -> TokenStream {
	let data = syn::parse_macro_input!(item as SelectWithTimeout);

	let rest = data.rest;
	let (timeout, timeout_scale) = data
		.timeout
		.map(|(timeout, timeout_scale)| (timeout.to_token_stream(), timeout_scale))
		.unwrap_or((quote! { #DEFAULT_TIMEOUT }, 1));
	let returning = data.returning.map_or_else(
		|| {
			quote! {Default::default()}
		},
		|expr| {
			quote! {#expr}
		},
	);

	// Add jitter via `rand::SmallRng`. Secure randomness is not important here.
	let jitter = if let Some((jitter, jitter_scale)) = data.jitter {
		quote! {
			+ {
				use rand::SeedableRng;
				rand::rngs::SmallRng::from_entropy().gen_range(0..=(#jitter * #jitter_scale))
			}
		}
	} else {
		quote! {}
	};

	let result = quote! {
		tokio::select! {
			#rest
			_ = async {tokio::time::sleep(std::time::Duration::from_millis(#timeout * #timeout_scale #jitter)).await} => #returning,
		}
	};

	result.into()
}
