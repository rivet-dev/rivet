//! Recursive-descent AST parser for udb key paths.
//!
//! Grammar:
//!
//! ```text
//! path        := "/"? (segment ("/" segment)*)?
//! segment     := ".." | "." | typed_value
//! typed_value := (TYPE ":")? value
//! value       := nested | atom
//! nested      := "[" (typed_value ("/" typed_value)*)? "]"
//! atom        := (escaped | not_special)*
//! ```
//!
//! `TYPE` is a non-empty identifier (any chars except special). Recognized
//! types are `u64`, `i64`, `f64`, `uuid`, `id`, `str`, `nested`, `bytes`, `b`.
//! `escaped` is `\\` followed by any char. `not_special` is any char except
//! `[`, `]`, `\\`, and `/` (plus `:` while scanning a type prefix). `/`
//! separates path segments at the top level and nested-value items inside a
//! `[...]`.

use std::str::FromStr;

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::util::udb::{SimpleTuple, SimpleTupleSegment, SimpleTupleValue};

#[derive(Debug)]
pub struct ParsedPath {
	pub tuple: SimpleTuple,
	pub relative: bool,
	pub back_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct TypedValueAst {
	ty: Option<String>,
	value: ValueAst,
}

#[derive(Debug, Clone, PartialEq)]
enum ValueAst {
	Atom(String),
	Nested(Vec<TypedValueAst>),
}

enum SegmentAst {
	Back,
	Current,
	Value(TypedValueAst),
}

pub fn parse_path(input: &str) -> Result<ParsedPath> {
	let mut c = Cursor::new(input);
	let relative = !input.starts_with('/');
	if !relative {
		c.bump();
	}

	let mut segments = Vec::new();
	let mut back_count = 0;
	let mut normal_seen = false;

	while !c.eof() {
		match parse_segment(&mut c)? {
			SegmentAst::Back => {
				if normal_seen {
					bail!("invalid path: '..' cannot go after other segments");
				}
				back_count += 1;
			}
			SegmentAst::Current => {}
			SegmentAst::Value(tv) => {
				normal_seen = true;
				segments.push(build_segment(&tv)?);
			}
		}

		match c.peek() {
			Some('/') => {
				c.bump();
			}
			Some(ch) => bail!("unexpected `{ch}` at position {}", c.pos),
			None => break,
		}
	}

	Ok(ParsedPath {
		tuple: SimpleTuple { segments },
		relative,
		back_count,
	})
}

fn parse_segment(c: &mut Cursor) -> Result<SegmentAst> {
	if c.rest().starts_with("..") && c.at_segment_boundary(c.pos + 2) {
		c.pos += 2;
		return Ok(SegmentAst::Back);
	}
	if c.rest().starts_with('.') && c.at_segment_boundary(c.pos + 1) {
		c.pos += 1;
		return Ok(SegmentAst::Current);
	}

	let tv = parse_typed_value(c)?;
	Ok(SegmentAst::Value(tv))
}

fn parse_typed_value(c: &mut Cursor) -> Result<TypedValueAst> {
	let ty = scan_type_prefix(c);
	let value = if c.peek() == Some('[') {
		c.bump();
		let mut items = Vec::new();
		if c.peek() != Some(']') {
			loop {
				items.push(parse_typed_value(c)?);
				match c.peek() {
					Some('/') => {
						c.bump();
					}
					Some(']') => break,
					Some(ch) => bail!(
						"expected `/` or `]` in nested value at position {} (got `{ch}`)",
						c.pos
					),
					None => bail!("unterminated nested value"),
				}
			}
		}
		c.bump();
		ValueAst::Nested(items)
	} else {
		ValueAst::Atom(read_atom(c))
	};

	Ok(TypedValueAst { ty, value })
}

fn scan_type_prefix(c: &mut Cursor) -> Option<String> {
	let start = c.pos;
	while let Some(ch) = c.peek() {
		match ch {
			'\\' => {
				c.bump();
				c.bump();
			}
			':' => {
				let ty = c.s[start..c.pos].trim().to_string();
				c.bump();
				if ty.is_empty() {
					return None;
				}
				return Some(ty);
			}
			'/' | '[' | ']' => break,
			_ => {
				c.bump();
			}
		}
	}
	c.pos = start;
	None
}

fn read_atom(c: &mut Cursor) -> String {
	let mut out = String::new();
	while let Some(ch) = c.peek() {
		match ch {
			'\\' => {
				c.bump();
				if let Some(next) = c.bump() {
					out.push(next);
				}
			}
			'/' | '[' | ']' => break,
			_ => {
				out.push(ch);
				c.bump();
			}
		}
	}
	out
}

fn build_segment(tv: &TypedValueAst) -> Result<SimpleTupleSegment> {
	let value = build_value(tv.ty.as_deref(), &tv.value)?;
	Ok(SimpleTupleSegment::new(value))
}

fn build_value(ty: Option<&str>, v: &ValueAst) -> Result<SimpleTupleValue> {
	match v {
		ValueAst::Nested(items) => {
			if let Some(t) = ty {
				if t != "nested" {
					bail!("type `{t}` cannot be applied to a nested value");
				}
			}
			let mut out = Vec::with_capacity(items.len());
			for item in items {
				out.push(build_value(item.ty.as_deref(), &item.value)?);
			}
			Ok(SimpleTupleValue::Nested(out))
		}
		ValueAst::Atom(s) => {
			if ty == Some("nested") {
				bail!("type `nested` requires a `[...]` value");
			}
			build_atom(ty, s.trim())
		}
	}
}

fn build_atom(ty: Option<&str>, value: &str) -> Result<SimpleTupleValue> {
	match ty {
		Some("u64") => value
			.parse::<u64>()
			.map(SimpleTupleValue::U64)
			.with_context(|| format!("could not parse `{value}` as u64")),
		Some("i64") => value
			.parse::<i64>()
			.map(SimpleTupleValue::I64)
			.with_context(|| format!("could not parse `{value}` as i64")),
		Some("f64") => value
			.parse::<f64>()
			.map(SimpleTupleValue::F64)
			.with_context(|| format!("could not parse `{value}` as f64")),
		Some("uuid") => Uuid::from_str(value)
			.map(SimpleTupleValue::Uuid)
			.with_context(|| format!("could not parse `{value}` as UUID")),
		Some("id") => rivet_util::Id::from_str(value)
			.map(SimpleTupleValue::Id)
			.with_context(|| format!("could not parse `{value}` as ID")),
		Some("str") => Ok(SimpleTupleValue::String(value.to_string())),
		Some("bytes") | Some("b") => hex::decode(value.as_bytes())
			.map(SimpleTupleValue::Bytes)
			.with_context(|| format!("could not parse `{value}` as hex encoded bytes")),
		Some(t) => bail!("unknown type: `{t}`"),
		None => Ok(auto_detect(value)),
	}
}

fn auto_detect(value: &str) -> SimpleTupleValue {
	if let Ok(v) = value.parse::<u64>() {
		SimpleTupleValue::U64(v)
	} else if let Ok(v) = value.parse::<i64>() {
		SimpleTupleValue::I64(v)
	} else if let Ok(v) = value.parse::<f64>() {
		SimpleTupleValue::F64(v)
	} else if let Ok(v) = Uuid::from_str(value) {
		SimpleTupleValue::Uuid(v)
	} else if let Ok(v) = rivet_util::Id::from_str(value) {
		SimpleTupleValue::Id(v)
	} else if let Some(v) = universaldb::utils::keys::key_from_str(value) {
		SimpleTupleValue::U64(v as u64)
	} else {
		SimpleTupleValue::String(value.to_string())
	}
}

struct Cursor<'a> {
	s: &'a str,
	pos: usize,
}

impl<'a> Cursor<'a> {
	fn new(s: &'a str) -> Self {
		Self { s, pos: 0 }
	}

	fn rest(&self) -> &'a str {
		&self.s[self.pos..]
	}

	fn peek(&self) -> Option<char> {
		self.rest().chars().next()
	}

	fn bump(&mut self) -> Option<char> {
		let ch = self.peek()?;
		self.pos += ch.len_utf8();
		Some(ch)
	}

	fn eof(&self) -> bool {
		self.pos >= self.s.len()
	}

	fn at_segment_boundary(&self, pos: usize) -> bool {
		pos >= self.s.len() || self.s.as_bytes()[pos] == b'/'
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use super::*;

	fn parse(s: &str) -> ParsedPath {
		parse_path(s).expect("expected ok")
	}

	fn values(p: &ParsedPath) -> Vec<SimpleTupleValue> {
		p.tuple.segments.iter().map(|s| s.value().clone()).collect()
	}

	#[test]
	fn absolute_vs_relative() {
		let p = parse("/foo/bar");
		assert!(!p.relative);
		assert_eq!(p.back_count, 0);
		assert_eq!(
			values(&p),
			vec![
				SimpleTupleValue::String("foo".into()),
				SimpleTupleValue::String("bar".into()),
			]
		);

		let p = parse("foo/bar");
		assert!(p.relative);
		assert_eq!(p.back_count, 0);
		assert_eq!(
			values(&p),
			vec![
				SimpleTupleValue::String("foo".into()),
				SimpleTupleValue::String("bar".into()),
			]
		);
	}

	#[test]
	fn empty_path() {
		let p = parse("");
		assert!(p.relative);
		assert!(values(&p).is_empty());

		let p = parse("/");
		assert!(!p.relative);
		assert!(values(&p).is_empty());
	}

	#[test]
	fn back_and_current() {
		let p = parse("../../foo");
		assert!(p.relative);
		assert_eq!(p.back_count, 2);
		assert_eq!(values(&p), vec![SimpleTupleValue::String("foo".into())]);

		let p = parse("./foo");
		assert_eq!(p.back_count, 0);
		assert_eq!(values(&p), vec![SimpleTupleValue::String("foo".into())]);

		// '..' after a normal segment is invalid.
		assert!(parse_path("foo/..").is_err());
	}

	#[test]
	fn auto_detect_u64() {
		assert_eq!(values(&parse("/42")), vec![SimpleTupleValue::U64(42)]);
	}

	#[test]
	fn auto_detect_i64() {
		assert_eq!(values(&parse("/-3")), vec![SimpleTupleValue::I64(-3)]);
	}

	#[test]
	fn auto_detect_f64() {
		assert_eq!(values(&parse("/1.5")), vec![SimpleTupleValue::F64(1.5)]);
	}

	#[test]
	fn auto_detect_uuid() {
		let raw = "550e8400-e29b-41d4-a716-446655440000";
		assert_eq!(
			values(&parse(&format!("/{raw}"))),
			vec![SimpleTupleValue::Uuid(Uuid::from_str(raw).unwrap())]
		);
	}

	#[test]
	fn auto_detect_string_fallback() {
		assert_eq!(
			values(&parse("/hello")),
			vec![SimpleTupleValue::String("hello".into())]
		);
	}

	#[test]
	fn typed_prefix_str_keeps_value_literal() {
		assert_eq!(
			values(&parse("/str:42")),
			vec![SimpleTupleValue::String("42".into())]
		);
		assert_eq!(
			values(&parse("/str:hello world")),
			vec![SimpleTupleValue::String("hello world".into())]
		);
	}

	#[test]
	fn typed_prefix_u64_ok() {
		assert_eq!(values(&parse("/u64:123")), vec![SimpleTupleValue::U64(123)]);
	}

	#[test]
	fn typed_prefix_u64_invalid_value() {
		assert!(parse_path("/u64:notanumber").is_err());
	}

	#[test]
	fn unknown_type_errors() {
		// `foobar:baz` parses `foobar` as a type prefix; type is unknown so this errors.
		let err = parse_path("/foobar:baz").expect_err("should error on unknown type");
		assert!(
			err.to_string().contains("unknown type"),
			"unexpected error: {err}"
		);
	}

	#[test]
	fn escaped_colon_is_string() {
		// `foobar\:baz` escapes the colon so the whole token is parsed as one atom
		// and falls through to a String value.
		assert_eq!(
			values(&parse(r"/foobar\:baz")),
			vec![SimpleTupleValue::String("foobar:baz".into())]
		);
	}

	#[test]
	fn second_colon_is_literal() {
		// Only the first unescaped `:` is the type separator; later ones are atom chars.
		assert_eq!(
			values(&parse("/str:a:b")),
			vec![SimpleTupleValue::String("a:b".into())]
		);
	}

	#[test]
	fn nested_value() {
		assert_eq!(
			values(&parse("/[1/hello]")),
			vec![SimpleTupleValue::Nested(vec![
				SimpleTupleValue::U64(1),
				SimpleTupleValue::String("hello".into()),
			])]
		);
	}

	#[test]
	fn nested_with_typed_items() {
		assert_eq!(
			values(&parse("/[u64:1/str:hello]")),
			vec![SimpleTupleValue::Nested(vec![
				SimpleTupleValue::U64(1),
				SimpleTupleValue::String("hello".into()),
			])]
		);
	}

	#[test]
	fn nested_unterminated_errors() {
		assert!(parse_path("/[1/2").is_err());
	}

	#[test]
	fn nested_of_nested() {
		assert_eq!(
			values(&parse("/[[1/2]/3]")),
			vec![SimpleTupleValue::Nested(vec![
				SimpleTupleValue::Nested(vec![SimpleTupleValue::U64(1), SimpleTupleValue::U64(2),]),
				SimpleTupleValue::U64(3),
			])]
		);
	}

	#[test]
	fn comma_is_literal_in_nested() {
		// `,` is no longer a separator. It's part of the atom verbatim.
		assert_eq!(
			values(&parse("/[a,b]")),
			vec![SimpleTupleValue::Nested(vec![SimpleTupleValue::String(
				"a,b".into()
			)])]
		);
	}

	#[test]
	fn empty_nested() {
		assert_eq!(
			values(&parse("/[]")),
			vec![SimpleTupleValue::Nested(vec![])]
		);
	}

	#[test]
	fn escaped_slash_in_atom() {
		// `\\/` keeps the slash inside the atom, so we get a single segment containing `a/b`.
		assert_eq!(
			values(&parse(r"/a\/b")),
			vec![SimpleTupleValue::String("a/b".into())]
		);
	}

	#[test]
	fn bytes_type() {
		assert_eq!(
			values(&parse("/bytes:deadbeef")),
			vec![SimpleTupleValue::Bytes(vec![0xde, 0xad, 0xbe, 0xef])]
		);
		assert!(parse_path("/bytes:zz").is_err());
	}

	#[test]
	fn nested_type_requires_brackets() {
		assert!(parse_path("/nested:foo").is_err());
	}

	#[test]
	fn typed_prefix_disallows_nested_value() {
		assert!(parse_path("/u64:[1/2]").is_err());
	}

	#[test]
	fn multiple_segments_mixed_types() {
		assert_eq!(
			values(&parse("/u64:1/str:hello/world")),
			vec![
				SimpleTupleValue::U64(1),
				SimpleTupleValue::String("hello".into()),
				SimpleTupleValue::String("world".into()),
			]
		);
	}

	#[test]
	fn relative_with_back_count() {
		let p = parse("../foo/bar");
		assert!(p.relative);
		assert_eq!(p.back_count, 1);
		assert_eq!(
			values(&p),
			vec![
				SimpleTupleValue::String("foo".into()),
				SimpleTupleValue::String("bar".into()),
			]
		);
	}
}
