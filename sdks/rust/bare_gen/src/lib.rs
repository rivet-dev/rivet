/*!
`bare_gen` provides a simple function that generates Rust types from BARE schema files.
Generated types implicitly implement `serde::Serialize` and `serde::Deserialize`, as `serde_bare`
is used to handle encoding and decoding. Please see
[serde_bare's documentation](https://docs.rs/serde_bare/latest/serde_bare/) for information on how
the Rust data model maps to the BARE data model.

To use this macro, define a BARE schema file and populate it with type declarations.

For example:


```bare
// schema.bare
type PublicKey data[128]
type Time str # ISO 8601

type Department enum {
  ACCOUNTING
  ADMINISTRATION
  CUSTOMER_SERVICE
  DEVELOPMENT

  # Reserved for the CEO
  JSMITH = 99
}

type Address list<str>[4] # street, city, state, country

type Customer struct {
  name: str
  email: str
  address: Address
  orders: list<struct {
	orderId: i64
	quantity: i32
  }>
  metadata: map<str><data>
}

type Employee struct {
  name: str
  email: str
  address: Address
  department: Department
  hireDate: Time
  publicKey: optional<PublicKey>
  metadata: map<str><data>
}

type TerminatedEmployee void

type Person union {Customer | Employee | TerminatedEmployee}
```

Then, within a Rust source file:

```ignore

bare_gen::bare_schema("schema.bare"); // TokenStream

```

# BARE => Rust Data Mapping

In most areas, the BARE data model maps cleanly to a Rust representation. Unless otherwise
specified, the most obvious Rust data type is generated from a given BARE type. For example,
a BARE `option<type>` is mapped to Rust's `Option<type>`, BARE unions and enums are mapped to
Rust `enum`s. See below for opinions that this crate has around data types that do not map
as cleanly or require additional explanation.

## Maps

BARE maps are interpreted as `HashMap<K, V>` in Rust. As of now, this is not configurable, but
may be in the future.

## Variable Length Integers

The variable `uint` and `int` types are mapped to [`serde_bare::UInt`] and [`serde_bare::Int`]
respectively. These types wrap `u64` and `i64` (the largest possible sized values stored in BARE
variable length integers).

Arrays that have 32 or less elements are mapped directly as Rust arrays, while BARE arrays with
more than 32 elements are converted into `Vec<T>`.

*/

use std::{collections::BTreeMap, fs::read_to_string, path::Path};

use heck::{ToSnakeCase, ToUpperCamelCase};
use parser::{AnyType, PrimativeType, StructField, parse_string};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

mod parser;

fn ident_from_string(s: &String) -> Ident {
	Ident::new(s, Span::call_site())
}

/// `bare_schema` parses a BARE schema file and generates equivalent Rust code that is capable of
/// being serialized to and deserialized from bytes using the BARE encoding format. The macro takes
/// exactly one argument, a string that will be parsed as path pointing to a BARE schema file. The
/// path is treated as relative to the file location of the macro's use.
/// For details on how the BARE data model maps to the Rust data model, see the [`Serialize`
/// derive macro's documentation.](https://docs.rs/serde_bare/latest/serde_bare/)
pub fn bare_schema(schema_path: &Path) -> proc_macro2::TokenStream {
	let file = read_to_string(schema_path).unwrap();
	let mut schema_generator = SchemaGenerator {
		global_output: Default::default(),
		user_type_registry: parse_string(&file),
	};

	for (name, user_type) in &schema_generator.user_type_registry.clone() {
		schema_generator.gen_user_type(&name, &user_type);
	}

	schema_generator.complete()
}

struct SchemaGenerator {
	global_output: Vec<TokenStream>,
	user_type_registry: BTreeMap<String, AnyType>,
}

impl SchemaGenerator {
	/// Completes a generation cycle by consuming the `SchemaGenerator` and yielding a
	/// `TokenStream`.
	fn complete(self) -> TokenStream {
		let user_type_syntax = self.global_output;
		quote! {
			#[allow(unused_imports)]
			use rivet_util::serde::HashableMap;
			#[allow(unused_imports)]
			use serde::{Serialize, Deserialize};
			#[allow(unused_imports)]
			use serde_bare::{Uint, Int};

			#(#user_type_syntax)*
		}
	}

	/// `gen_user_type` is responsible for generating the token streams of a single user type at a top
	/// level. Rust does not support anonymous structs/enums/etc., so we must recursively parse any
	/// anonymous definitions and generate top-level definitions. As such, this function may generate
	/// multiple types.
	fn gen_user_type(&mut self, name: &String, t: &AnyType) {
		#[allow(unused_assignments)]
		use AnyType::*;
		let def = match t {
			Primative(p) => {
				let def = gen_primative_type_def(p);
				let ident = ident_from_string(name);
				quote! {
					pub type #ident = #def;
				}
			}
			List { inner, length } => {
				let def = self.gen_list(name, inner.as_ref(), length);
				let ident = ident_from_string(name);
				quote! {
					pub type #ident = #def;
				}
			}
			Struct(fields) => {
				self.gen_struct(name, fields);
				// `gen_struct` only has side-effects on the registry, so we return nothing
				TokenStream::new()
			}
			Map { key, value } => {
				let map_def = self.gen_map(name, key.as_ref(), value.as_ref());
				let ident = ident_from_string(name);
				quote! {
					pub type #ident = #map_def;
				}
			}
			Optional(inner) => {
				let inner_def = self.dispatch_type(name, inner);
				let ident = ident_from_string(name);
				quote! {
					pub type #ident = #inner_def;
				}
			}
			TypeReference(reference) => {
				panic!("Type reference is not valid as a top level definition: {reference}")
			}
			Enum(members) => {
				self.gen_enum(name, members);
				// `gen_enum` only has side-effects on the registry, so we return nothing
				TokenStream::new()
			}
			Union(members) => {
				self.gen_union(name, members);
				// `gen_union` only has side-effects on the registry, so we return nothing
				TokenStream::new()
			}
		};
		self.global_output.push(def);
	}

	fn dispatch_type(&mut self, name: &String, any_type: &AnyType) -> TokenStream {
		match any_type {
			AnyType::Primative(p) => gen_primative_type_def(p),
			AnyType::List { inner, length } => self.gen_list(name, inner.as_ref(), length),
			AnyType::Struct(fields) => self.gen_struct(name, fields),
			AnyType::Enum(members) => self.gen_enum(name, members),
			AnyType::Map { key, value } => self.gen_map(name, key.as_ref(), value.as_ref()),
			AnyType::Union(members) => self.gen_union(name, members),
			AnyType::Optional(inner) => self.gen_option(name, inner),
			AnyType::TypeReference(i) => {
				let ident = ident_from_string(i);
				quote! { #ident }
			}
		}
	}

	fn gen_map(&mut self, name: &String, key: &AnyType, value: &AnyType) -> TokenStream {
		let key_def = self.dispatch_type(name, key);
		let val_def = self.dispatch_type(name, value);
		quote! {
			HashableMap<#key_def, #val_def>
		}
	}

	fn gen_list(
		&mut self,
		name: &String,
		inner_type: &AnyType,
		size: &Option<usize>,
	) -> TokenStream {
		let inner_def = self.dispatch_type(name, inner_type);
		match *size {
			Some(size) if size <= 32 => quote! {
				[#inner_def; #size]
			},
			_ => quote! {
				Vec<#inner_def>
			},
		}
	}

	fn gen_struct(&mut self, name: &String, fields: &Vec<StructField>) -> TokenStream {
		// clone so we can safely drain this
		let fields_clone = fields.clone();
		let fields_gen = self.gen_struct_field(name, fields_clone);
		self.gen_anonymous(name, |ident| {
			quote! {
				#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
				pub struct #ident {
					#(#fields_gen),*
				}
			}
		})
	}

	fn gen_union(&mut self, name: &String, members: &Vec<AnyType>) -> TokenStream {
		let mut members_def: Vec<TokenStream> = Vec::with_capacity(members.len());
		for (i, member) in members.iter().enumerate() {
			// If this member is a user type alias for void, we'll not generate an inner type later
			let is_void_type = match member {
				AnyType::TypeReference(i) if self.user_type_registry.get(i).is_some() => {
					let reference = self.user_type_registry.get(i).unwrap();
					matches!(reference, AnyType::Primative(PrimativeType::Void))
				}
				_ => false,
			};

			// This is to allow the `registry` binding to not shadow the function arg, but instead
			// rebind it as it's used in the subsequent `gen_anonymous` call. We'll get move errors if
			// we don't do it this way.
			#[allow(unused_assignments)]
			let mut member_def = TokenStream::new();
			member_def = match member {
				AnyType::Struct(fields) => {
					let fields_defs = self.gen_struct_field(name, fields.clone());
					quote! {
						{
							#(#fields_defs),*
						}
					}
				}
				AnyType::TypeReference(i) if is_void_type => {
					let inner_def = ident_from_string(i);
					// The `inner_def` is always a top-level type here
					quote! {
						#inner_def
					}
				}
				_ => {
					let inner_def = self.dispatch_type(&format!("{name}Member{i}"), member);
					// The `inner_def` is always a top-level type here
					quote! {
						#inner_def(#inner_def)
					}
				}
			};
			members_def.push(member_def);
		}
		self.gen_anonymous(name, |ident| {
			quote! {
				#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
				pub enum #ident {
					#(#members_def),*
				}
			}
		})
	}

	fn gen_option(&mut self, name: &String, inner: &AnyType) -> TokenStream {
		let inner_def = self.dispatch_type(name, inner);
		quote! {
		   Option<#inner_def>
		}
	}

	fn gen_struct_field(
		&mut self,
		struct_name: &String,
		fields: Vec<StructField>,
	) -> Vec<TokenStream> {
		let mut fields_gen: Vec<TokenStream> = Vec::with_capacity(fields.len());
		for StructField { name, type_r } in fields {
			let name = name.to_snake_case();
			#[allow(unused_assignments)]
			let field_gen = self.dispatch_type(&format!("{struct_name}{name}"), &type_r);
			let ident = ident_from_string(&name);
			fields_gen.push(quote! {
				pub #ident: #field_gen
			})
		}
		fields_gen
	}

	fn gen_enum(&mut self, name: &String, members: &Vec<(String, Option<usize>)>) -> TokenStream {
		let member_defs = members.iter().map(|(name, val)| {
			let ident = ident_from_string(&name.to_upper_camel_case());
			if let Some(val) = val {
				quote! {
					#ident = #val
				}
			} else {
				quote! {
					#ident
				}
			}
		});
		self.gen_anonymous(name, |ident| {
			quote! {
				#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Clone)]
				#[repr(usize)]
				pub enum #ident {
					#(#member_defs),*
				}
			}
		})
	}

	/// `gen_anonymous` generates an identifier from the provided `name`, passed it to `inner`, pushes
	/// the result of `inner` to the `registry`, and yields a quoted version of the generated
	/// identifier. This is a common operation when generating types that are anonymous in a BARE
	/// schema but not allowed by be defined anonymously in Rust.
	fn gen_anonymous(
		&mut self,
		name: &String,
		inner: impl FnOnce(Ident) -> TokenStream,
	) -> TokenStream {
		let ident = ident_from_string(name);
		self.global_output.push(inner(ident.clone()));
		quote! {
			#ident
		}
	}
}

fn gen_primative_type_def(p: &PrimativeType) -> TokenStream {
	use PrimativeType::*;
	match p {
		UInt => quote! { Uint },
		U64 => quote! { u64 },
		U32 => quote! { u32 },
		U16 => quote! { u16 },
		U8 => quote! { u8 },
		Int => quote! { Int },
		I64 => quote! { i64 },
		I32 => quote! { i32 },
		I16 => quote! { i16 },
		I8 => quote! { i8 },
		F64 => quote! { f64 },
		F32 => quote! { f32 },
		Str => quote! { String },
		Data(s) => match s {
			Some(size) if *size <= 32 => quote! { [u8; #size] },
			_ => quote! { Vec<u8> },
		},
		Void => quote! { () },
		Bool => quote! { bool },
	}
}
