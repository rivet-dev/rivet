use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
	braced, bracketed, parenthesized,
	parse::{discouraged::Speculative, Parse, ParseStream},
	punctuated::{Pair, Punctuated},
	spanned::Spanned,
	Token,
};

const TUPLE: char = 't';
const MAP: char = 'm';
const ARRAY: char = 'a';
#[allow(dead_code)]
pub type SimpleTuple<T> = SimpleCollection<TUPLE, T>;
pub type SimpleBlock<T> = SimpleCollection<MAP, T>;
pub type SimpleMap<T> = SimpleBlock<FieldValue<T>>;
#[allow(dead_code)]
pub type SimpleArray<T> = SimpleCollection<ARRAY, T>;

type Comma = Token![,];
type Colon = Token![:];

/// A collection without a label.
/// Example:
/// ```rust
/// {
///     key: "value",
/// },
/// ```
/// or
/// ```rust
/// ["foo", 1]
/// ```
#[derive(Clone)]
pub struct SimpleCollection<const C: char, T: Parse + ToTokens> {
	scope_token: CollectionToken,
	values: Punctuated<T, Comma>,
}

impl<const C: char, T: Parse + ToTokens> Parse for SimpleCollection<C, T> {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let content;
		let scope_token = match C {
			TUPLE => CollectionToken::Paren(parenthesized!(content in input)),
			MAP => CollectionToken::Brace(braced!(content in input)),
			ARRAY => CollectionToken::Bracket(bracketed!(content in input)),
			_ => panic!("unimplemented SimpleCollection type {:?}", C),
		};

		let values = content.parse_terminated(T::parse)?;

		Ok(SimpleCollection {
			scope_token,
			values,
		})
	}
}

impl<const C: char, T: Parse + ToTokens> ToTokens for SimpleCollection<C, T> {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		match self.scope_token {
			CollectionToken::Paren(p) => p.surround(tokens, |tokens| {
				self.values.to_tokens(tokens);
			}),
			CollectionToken::Bracket(b) => b.surround(tokens, |tokens| {
				self.values.to_tokens(tokens);
			}),
			CollectionToken::Brace(b) => b.surround(tokens, |tokens| {
				self.values.to_tokens(tokens);
			}),
		}
	}
}

impl<const C: char, T: Parse + ToTokens> SimpleCollection<C, T> {
	pub fn iter(&self) -> syn::punctuated::Iter<T> {
		self.values.iter()
	}
}

impl<const C: char, T: Parse + ToTokens> IntoIterator for SimpleCollection<C, T> {
	type Item = T;
	type IntoIter = syn::punctuated::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.values.into_iter()
	}
}

// Implements ease of use features for map-like collections
impl<const C: char, T: Parse + ToTokens> SimpleCollection<C, FieldValue<T>> {
	pub fn get(&self, key: &str) -> Option<&T> {
		self.values
			.iter()
			.find(|arg| arg.member.to_token_stream().to_string() == key)
			.map(|arg| &arg.expr)
	}

	#[allow(dead_code)]
	pub fn has(&self, key: &str) -> bool {
		self.values
			.iter()
			.any(|arg| arg.member.to_token_stream().to_string() == key)
	}
}

#[derive(Clone)]
enum CollectionToken {
	Paren(syn::token::Paren),
	Bracket(syn::token::Bracket),
	Brace(syn::token::Brace),
}

// MARK: Derived from syn::FieldValue
/// A field-value pair in a struct literal.
#[derive(Clone)]
pub struct FieldValue<T: Parse + ToTokens> {
	// TODO: Derive syn::Member and give it a generic so the user can handle unnamed members if they'd like
	/// Name or index of the field.
	pub member: syn::Member,

	/// The colon in `Struct { x: x }`. If written in shorthand like
	/// `Struct { x }`, there is no colon.
	pub colon_token: Option<Colon>,

	/// Value of the field.
	pub expr: T,
}

impl<T: Parse + ToTokens> Parse for FieldValue<T> {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let member: syn::Member = input.parse()?;
		let colon_token = if input.peek(Token![:]) || matches!(member, syn::Member::Unnamed(_)) {
			Some(input.parse::<Token![:]>()?)
		} else if let syn::Member::Named(_) = &member {
			None
		} else {
			unreachable!()
		};

		let value: T = input.parse()?;

		Ok(FieldValue {
			member,
			colon_token,
			expr: value,
		})
	}
}

impl<T: Parse + ToTokens> ToTokens for FieldValue<T> {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		self.member.to_tokens(tokens);
		if let Some(colon_token) = &self.colon_token {
			colon_token.to_tokens(tokens);
			self.expr.to_tokens(tokens);
		}
	}
}

/// Parses anything that can be parsed as an expression, {...}, [...], or (...). Expression takes priority
// in parsing. Will throw `expected curly braces` when no variants match.
pub enum AnyArg<P, B, C>
where
	P: Parse + ToTokens + Spanned,
	B: Parse + ToTokens + Spanned,
	C: Parse + ToTokens + Spanned,
{
	Expr(syn::Expr),
	Paren(SimpleTuple<P>),
	Bracket(SimpleArray<B>),
	Block(SimpleBlock<C>),
}

impl<P, B, C> AnyArg<P, B, C>
where
	P: Parse + ToTokens + Spanned,
	B: Parse + ToTokens + Spanned,
	C: Parse + ToTokens + Spanned,
{
	pub fn expect_expr(&self) -> syn::Result<&syn::Expr> {
		match self {
			Self::Expr(v) => Ok(&v),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
		}
	}

	#[allow(dead_code)]
	pub fn expect_paren(&self) -> syn::Result<&SimpleTuple<P>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
			Self::Paren(v) => Ok(&v),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
		}
	}

	pub fn expect_bracket(&self) -> syn::Result<&SimpleArray<B>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
			Self::Bracket(v) => Ok(&v),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
		}
	}

	#[allow(dead_code)]
	pub fn expect_block(&self) -> syn::Result<&SimpleBlock<C>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected block")),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected block")),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected block")),
			Self::Block(v) => Ok(&v),
		}
	}
}

impl<P, B, C> Parse for AnyArg<P, B, C>
where
	P: Parse + ToTokens + Spanned,
	B: Parse + ToTokens + Spanned,
	C: Parse + ToTokens + Spanned,
{
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let fork = input.fork();

		if let Ok(v) = fork.parse::<syn::Expr>() {
			input.advance_to(&fork);

			Ok(Self::Expr(v))
		} else {
			let fork = input.fork();

			if let Ok(v) = fork.parse::<SimpleTuple<P>>() {
				input.advance_to(&fork);

				Ok(Self::Paren(v))
			} else {
				let fork = input.fork();

				if let Ok(v) = fork.parse::<SimpleArray<B>>() {
					input.advance_to(&fork);

					Ok(Self::Bracket(v))
				} else {
					input.parse::<SimpleBlock<C>>().map(Self::Block)
				}
			}
		}
	}
}

impl<P, B, C> ToTokens for AnyArg<P, B, C>
where
	P: Parse + ToTokens + Spanned,
	B: Parse + ToTokens + Spanned,
	C: Parse + ToTokens + Spanned,
{
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		match self {
			Self::Expr(v) => v.to_tokens(tokens),
			Self::Paren(v) => v.to_tokens(tokens),
			Self::Bracket(v) => v.to_tokens(tokens),
			Self::Block(v) => v.to_tokens(tokens),
		}
	}
}

// Same as `AnyArg` but only has Expr types.
pub enum ExprTree {
	Expr(syn::Expr),
	Paren(SimpleTuple<ExprTree>),
	Bracket(SimpleArray<ExprTree>),
	Block(SimpleMap<ExprTree>),
}

impl ExprTree {
	pub fn expect_expr(&self) -> syn::Result<&syn::Expr> {
		match self {
			Self::Expr(v) => Ok(&v),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected single expression")),
		}
	}

	#[allow(dead_code)]
	pub fn expect_paren(&self) -> syn::Result<&SimpleTuple<ExprTree>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
			Self::Paren(v) => Ok(v),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected parenthesis block")),
		}
	}

	#[allow(dead_code)]
	pub fn expect_bracket(&self) -> syn::Result<&SimpleArray<ExprTree>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
			Self::Bracket(v) => Ok(&v),
			Self::Block(v) => Err(syn::Error::new(v.span(), "Expected bracket block")),
		}
	}

	pub fn expect_block(&self) -> syn::Result<&SimpleMap<ExprTree>> {
		match self {
			Self::Expr(v) => Err(syn::Error::new(v.span(), "Expected map")),
			Self::Paren(v) => Err(syn::Error::new(v.span(), "Expected map")),
			Self::Bracket(v) => Err(syn::Error::new(v.span(), "Expected map")),
			Self::Block(v) => Ok(&v),
		}
	}
}

impl Parse for ExprTree {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let fork1 = input.fork();

		let error1 = match fork1.parse::<syn::Expr>() {
			Ok(v) => {
				input.advance_to(&fork1);

				return Ok(Self::Expr(v));
			}
			Err(err) => err,
		};

		let fork2 = input.fork();
		let error2 = match fork2.parse::<SimpleTuple<ExprTree>>() {
			Ok(v) => {
				input.advance_to(&fork2);

				return Ok(Self::Paren(v));
			}
			Err(err) => err,
		};

		let fork3 = input.fork();
		let error3 = match fork3.parse::<SimpleArray<ExprTree>>() {
			Ok(v) => {
				input.advance_to(&fork3);

				return Ok(Self::Bracket(v));
			}
			Err(err) => err,
		};

		let fork4 = input.fork();
		let error4 = match fork4.parse::<SimpleMap<ExprTree>>() {
			Ok(v) => {
				input.advance_to(&fork4);

				return Ok(Self::Block(v));
			}
			Err(err) => err,
		};

		let mut errors = vec![
			(fork1, error1),
			(fork2, error2),
			(fork3, error3),
			(fork4, error4),
		];
		// Sort by whichever fork got the furthest
		errors.sort_by_key(|(fork, _)| fork.span().end());

		// Using `syn::parse::Error::combine` doesn't work so print all errors manually
		eprintln!("\n- Other errors");
		for (_, error) in &errors {
			eprintln!("| {}", error);
		}
		eprintln!("v");

		let (_, error) = errors.pop().unwrap();

		Err(error)
	}
}

impl ToTokens for ExprTree {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		match self {
			Self::Expr(v) => v.to_tokens(tokens),
			Self::Paren(v) => v.to_tokens(tokens),
			Self::Bracket(v) => v.to_tokens(tokens),
			Self::Block(v) => v.to_tokens(tokens),
		}
	}
}

/// Takes any parsable item and re-parses it into the desired type.
pub fn re_parse<T: Parse>(input: impl ToTokens) -> syn::Result<T> {
	let mut tokens = TokenStream2::new();
	input.to_tokens(&mut tokens);

	syn::parse::<T>(TokenStream::from(tokens))
}

/// Generate a rate limit key based on the path of the endpoint function.
pub fn default_rate_limit_key(path: &syn::Path) -> String {
	path.segments
		.pairs()
		.map(|pair| {
			format!(
				"-{}",
				match pair {
					Pair::Punctuated(segment, _) => &segment.ident,
					Pair::End(segment) => &segment.ident,
				}
			)
		})
		.collect()
}

pub fn default_rate_limit_bucket(req_type: String) -> TokenStream2 {
	let count: u64 = match req_type.as_str() {
		"GET" => 128,
		"PATCH" => 32,
		"POST" | "PUT" | "DELETE" => 16,
		_ => 16,
	};
	// 15 minutes
	let bucket_duration = default_rate_limit_bucket_duration();

	quote! {
		rivet_cache::RateLimitBucketConfig {
			count: #count,
			bucket_duration_ms: #bucket_duration,
		}
	}
}

pub fn default_rate_limit_bucket_duration() -> i64 {
	1000 * 60 * 15
}
