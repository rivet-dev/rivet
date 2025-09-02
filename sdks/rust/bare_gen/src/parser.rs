#![allow(dead_code)]
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;
use std::collections::BTreeMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct BARE;

pub type Length = usize;

#[derive(Debug, Clone, Copy)]
pub enum PrimativeType {
	UInt,
	U64,
	U32,
	U16,
	U8,
	Int,
	I64,
	I32,
	I16,
	I8,
	F64,
	F32,
	Str,
	Data(Option<Length>),
	Void,
	Bool,
}

#[derive(Debug, Clone)]
pub enum AnyType {
	Primative(PrimativeType),
	List {
		inner: Box<AnyType>,
		length: Option<usize>,
	},
	Struct(Vec<StructField>),
	Enum(Vec<(String, Option<usize>)>),
	Map {
		key: Box<AnyType>,
		value: Box<AnyType>,
	},
	Union(Vec<AnyType>),
	Optional(Box<AnyType>),
	TypeReference(String),
}

#[derive(Debug, Clone)]
pub struct StructField {
	pub name: String,
	pub type_r: AnyType,
}

pub fn parse_string(schema: &str) -> BTreeMap<String, AnyType> {
	let schema = BARE::parse(Rule::schema, &schema)
		.unwrap_or_else(|e| panic!("{}", e))
		.next()
		.unwrap(); // this can't fail if parsing didn't fail
	if schema.as_rule() != Rule::schema {
		unreachable!()
	}
	let mut user_type_registry: BTreeMap<String, AnyType> = BTreeMap::default();
	for user_types in schema.into_inner() {
		if user_types.as_rule() == Rule::EOI {
			break;
		}
		let mut inner = user_types.into_inner();
		let user_type_name = inner.next().unwrap();
		let user_type_type = inner.next().unwrap();
		if user_type_registry.contains_key(user_type_name.as_str()) {
			panic!("Duplicate definition: {:?}", user_type_name.as_span())
		}
		let t = parse_any_type(&user_type_registry, user_type_type);
		user_type_registry.insert(user_type_name.as_str().into(), t);
	}
	user_type_registry
}

fn parse_any_type(registry: &BTreeMap<String, AnyType>, pair: Pair<'_, Rule>) -> AnyType {
	match pair.as_rule() {
		Rule::unsigned_t => parse_unsigned_int(pair),
		Rule::signed_t => parse_signed_int(pair),
		Rule::void_t => AnyType::Primative(PrimativeType::Void),
		Rule::str_t => AnyType::Primative(PrimativeType::Str),
		Rule::bool_t => AnyType::Primative(PrimativeType::Bool),
		Rule::float_t => parse_float(pair),
		Rule::data_t => {
			let length_t = pair.into_inner().next();
			if let Some(length_t) = length_t {
				let length: usize = length_t.as_str().parse().unwrap();
				AnyType::Primative(PrimativeType::Data(Some(length)))
			} else {
				AnyType::Primative(PrimativeType::Data(None))
			}
		}
		Rule::enum_t => parse_enum(pair),
		Rule::list_t => parse_list(registry, pair),
		Rule::struct_t => parse_struct(registry, pair),
		Rule::map_t => parse_map(registry, pair),
		Rule::union_t => parse_union(registry, pair),
		Rule::optional_t => {
			let inner_type = pair.into_inner().next().unwrap();
			AnyType::Optional(Box::new(parse_any_type(registry, inner_type)))
		}
		Rule::user_type_name => {
			let user_type = pair.as_str();
			if registry.contains_key(user_type) {
				AnyType::TypeReference(user_type.into())
			} else {
				panic!("User type {user_type} has not been defined yet.");
			}
		}
		x => panic!("Unreachable: {x:?}"),
	}
}

fn parse_unsigned_int(pair: Pair<'_, Rule>) -> AnyType {
	assert!(pair.as_rule() == Rule::unsigned_t);
	AnyType::Primative(match pair.as_str() {
		"uint" => PrimativeType::UInt,
		"u64" => PrimativeType::U64,
		"u32" => PrimativeType::U32,
		"u16" => PrimativeType::U16,
		"u8" => PrimativeType::U8,
		_ => unreachable!(),
	})
}

fn parse_signed_int(pair: Pair<'_, Rule>) -> AnyType {
	assert!(pair.as_rule() == Rule::signed_t);
	AnyType::Primative(match pair.as_str() {
		"int" => PrimativeType::Int,
		"i64" => PrimativeType::I64,
		"i32" => PrimativeType::I32,
		"i16" => PrimativeType::I16,
		"i8" => PrimativeType::I8,
		_ => unreachable!(),
	})
}

fn parse_enum(pair: Pair<'_, Rule>) -> AnyType {
	let mut members: Vec<(String, Option<Length>)> = Vec::new();
	for enum_value in pair.into_inner() {
		assert!(enum_value.as_rule() == Rule::enum_value);
		let mut e = enum_value.into_inner();
		let enum_value_name = e.next().unwrap();
		let value: Option<usize> = e.next().and_then(|e| Some(e.as_str().parse().unwrap()));
		members.push((enum_value_name.as_str().into(), value));
	}
	AnyType::Enum(members)
}

fn parse_list(registry: &BTreeMap<String, AnyType>, pair: Pair<'_, Rule>) -> AnyType {
	let mut list = pair.into_inner();
	let list_type = list.next().unwrap();
	let inner = parse_any_type(registry, list_type);
	let length: Option<usize> = list
		.next()
		.and_then(|e: Pair<'_, Rule>| Some(e.as_str().parse().unwrap()));
	AnyType::List {
		inner: Box::new(inner),
		length,
	}
}

fn parse_struct(registry: &BTreeMap<String, AnyType>, pair: Pair<'_, Rule>) -> AnyType {
	let mut st = pair.into_inner();
	let mut fields: Vec<StructField> = Vec::new();
	while let Some(struct_t) = st.next() {
		let mut struct_field = struct_t.into_inner();
		let field_name = struct_field.next().unwrap();
		let field_type = struct_field.next().unwrap();
		let ft = parse_any_type(registry, field_type);
		fields.push(StructField {
			name: field_name.as_str().to_string(),
			type_r: ft,
		})
	}
	AnyType::Struct(fields)
}
fn parse_map(registry: &BTreeMap<String, AnyType>, pair: Pair<'_, Rule>) -> AnyType {
	let mut map = pair.into_inner();
	let key_t = map.next().unwrap();
	let value_t = map.next().unwrap();
	let key = parse_any_type(registry, key_t);
	let value = parse_any_type(registry, value_t);
	AnyType::Map {
		key: Box::new(key),
		value: Box::new(value),
	}
}
fn parse_union(registry: &BTreeMap<String, AnyType>, pair: Pair<'_, Rule>) -> AnyType {
	let union_t = pair.into_inner();
	let mut members: Vec<AnyType> = Vec::new();
	for union_member in union_t {
		let t = parse_any_type(registry, union_member);
		members.push(t);
	}
	AnyType::Union(members)
}

fn parse_float(pair: Pair<'_, Rule>) -> AnyType {
	assert!(pair.as_rule() == Rule::float_t);
	AnyType::Primative(match pair.as_str() {
		"f32" => PrimativeType::F32,
		"f64" => PrimativeType::F64,
		_ => unreachable!(),
	})
}

#[cfg(test)]
mod test {
	use std::fs::read_to_string;

	use super::*;

	#[test]
	fn basic() {
		let file = read_to_string("./src/example.bare").unwrap();
		let user_type_registry: BTreeMap<String, AnyType> = parse_string(&file);

		for (key, value) in user_type_registry.iter() {
			println!("{} = {:?}", key, value);
		}
	}
}
