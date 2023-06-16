use std::{
	convert::{TryFrom, TryInto},
	fmt::Write,
	string::ToString,
};

use global_error::prelude::*;
use regex;
use types::rivet::common;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GlobToken {
	Char(char),
	AnySequence,
	AnyRecursiveSequence,
}

#[derive(Hash, PartialEq, Eq)]
pub struct Glob {
	pub tokens: Vec<GlobToken>,
}

impl Glob {
	pub fn new(tokens: Vec<GlobToken>) -> Self {
		Glob { tokens }
	}

	pub fn parse(input: &str) -> GlobalResult<Self> {
		let mut tokens = Vec::with_capacity(input.len() / 2); // Rough capacity estimate

		// Separate by slashes
		let mut dont_split = false;
		let segments = input
			.split(|c| {
				if dont_split {
					dont_split = false;
					false
				} else if c == '/' {
					true
				} else {
					dont_split = c == '\\';
					false
				}
			})
			.collect::<Vec<_>>();
		let len = segments.len();

		let mut previous_was_double_star = false;
		for (i, segment) in segments.into_iter().enumerate() {
			match segment {
				"**" => {
					// Don't insert multiple consecutive double star segments, they are redundant
					if !previous_was_double_star {
						previous_was_double_star = true;

						tokens.push(GlobToken::AnyRecursiveSequence);
					}
				}
				_ => {
					previous_was_double_star = false;

					let chars = segment.chars();
					let mut previous_was_star = false;

					for c in chars {
						match c {
							'*' => {
								if previous_was_star {
									panic_with!(
										GLOB_INVALID,
										error = "segments that contain two stars (**) cannot contain any other characters"
									);
								}

								tokens.push(GlobToken::AnySequence);
								previous_was_star = true;
							}
							_ => {
								tokens.push(GlobToken::Char(c));
								previous_was_star = false;
							}
						}
					}

					if i != len - 1 {
						tokens.push(GlobToken::Char('/'));
					}
				}
			}
		}

		Ok(Glob::new(tokens))
	}
}

pub trait Traefik {
	fn as_traefik(&self) -> Result<String, std::fmt::Error>;
}

impl Traefik for Glob {
	fn as_traefik(&self) -> Result<String, std::fmt::Error> {
		let mut res = String::with_capacity(self.tokens.len()); // Rough capacity estimate

		write!(&mut res, "{{glob:")?;

		for token in &self.tokens {
			match token {
				GlobToken::Char(c) => {
					write!(&mut res, "{}", regex::escape(c.encode_utf8(&mut [0u8; 4])))?
				}
				GlobToken::AnySequence => write!(&mut res, "[^/]+")?,
				GlobToken::AnyRecursiveSequence => write!(&mut res, ".*")?,
			}
		}

		write!(&mut res, "}}")?;

		Ok(res)
	}
}

impl std::fmt::Debug for Glob {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for token in &self.tokens {
			match token {
				GlobToken::Char(c) => write!(f, "{}", c)?,
				GlobToken::AnySequence => write!(f, "*")?,
				GlobToken::AnyRecursiveSequence => write!(f, "**/")?,
			}
		}

		Ok(())
	}
}

impl ToString for Glob {
	fn to_string(&self) -> String {
		self.tokens
			.iter()
			.map(|token| match token {
				GlobToken::Char(c) => c.to_string(),
				GlobToken::AnySequence => "*".to_string(),
				GlobToken::AnyRecursiveSequence => "**/".to_string(),
			})
			.collect::<String>()
	}
}

impl From<Glob> for common::Glob {
	fn from(value: Glob) -> Self {
		common::Glob {
			tokens: value.tokens.into_iter().map(Into::into).collect::<Vec<_>>(),
		}
	}
}

impl From<GlobToken> for common::glob::Token {
	fn from(value: GlobToken) -> Self {
		let kind = match value {
			GlobToken::Char(c) => common::glob::token::Kind::Char(c.to_string()),
			GlobToken::AnySequence => common::glob::token::Kind::AnySequence(()),
			GlobToken::AnyRecursiveSequence => common::glob::token::Kind::AnyRecursiveSequence(()),
		};

		common::glob::Token { kind: Some(kind) }
	}
}

impl TryFrom<common::Glob> for Glob {
	type Error = GlobalError;

	fn try_from(value: common::Glob) -> GlobalResult<Self> {
		Ok(Glob::new(
			value
				.tokens
				.into_iter()
				.map(TryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		))
	}
}

impl TryFrom<common::glob::Token> for GlobToken {
	type Error = GlobalError;

	fn try_from(value: common::glob::Token) -> GlobalResult<Self> {
		match internal_unwrap!(value.kind) {
			common::glob::token::Kind::Char(c) => {
				Ok(GlobToken::Char(*internal_unwrap!(c.chars().next())))
			}
			common::glob::token::Kind::AnySequence(_) => Ok(GlobToken::AnySequence),
			common::glob::token::Kind::AnyRecursiveSequence(_) => {
				Ok(GlobToken::AnyRecursiveSequence)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn parse() {
		{
			let glob = Glob::parse("**/*.html").unwrap();
			let mut tokens = glob.tokens.iter();

			assert_eq!(tokens.next().unwrap(), &GlobToken::AnyRecursiveSequence);
			assert_eq!(tokens.next().unwrap(), &GlobToken::AnySequence);
			// Rest
			assert_eq!(
				tokens
					.map(|token| match token {
						GlobToken::Char(c) => c.to_string(),
						GlobToken::AnySequence => "*".to_string(),
						GlobToken::AnyRecursiveSequence => "**/".to_string(),
					})
					.collect::<String>(),
				".html"
			);
		}

		{
			let glob = Glob::parse("**/*.html").unwrap();
			assert_eq!(&glob.to_string(), "**/*.html");
		}

		{
			// Sequent double stars are combined
			let glob = Glob::parse("**/**/*.html").unwrap();
			let mut tokens = glob.tokens.iter();
			assert_eq!(tokens.next().unwrap(), &GlobToken::AnyRecursiveSequence);
			assert_eq!(tokens.next().unwrap(), &GlobToken::AnySequence);
		}

		{
			let glob = Glob::parse("test-glob").unwrap();
			assert_eq!(&glob.to_string(), "test-glob");
			let mut tokens = glob.tokens.iter();
			assert_eq!(tokens.next().unwrap(), &GlobToken::Char('t'));
			assert_eq!(tokens.next().unwrap(), &GlobToken::Char('e'));
			assert_eq!(tokens.next().unwrap(), &GlobToken::Char('s'));
			assert_eq!(tokens.next().unwrap(), &GlobToken::Char('t'));
			assert_eq!(tokens.next().unwrap(), &GlobToken::Char('-'));
		}

		Glob::parse("***").unwrap_err();
		Glob::parse("**a").unwrap_err();
		Glob::parse("*.*.*.*a*").unwrap();
	}
}
