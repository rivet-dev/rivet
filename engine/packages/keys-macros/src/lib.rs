//! Derives for formal keys: a key is a plain struct, and its attribute lists the components it
//! packs to. Each derive generates `new` and the address-space traits of its family in
//! `rivet_keys::keys::codec`, and nothing else.
//!
//! A component list such as `key = [ACTOR, actor_id]` names fields and tags: an identifier that
//! names a field of the struct packs that field, and any other identifier packs the tag constant
//! of that name from the family's tag space in `rivet_keys::keys::tags`. Every field except a
//! `#[group]` field (and a UPS subject's `#[subject]` buffer) appears exactly once.
//!
//! Each family takes only the options that mean something for it, and the derive refuses the
//! rest, a repeated option and a second attribute instead of ignoring them. Generated locals
//! start with `__`, so a field named like one of them (`out`, `depth`, `parts`) is not shadowed.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
	Data, DeriveInput, Error, Expr, Fields, Ident, Result, Token, Type, bracketed,
	parse::{Parse, ParseStream},
	parse_macro_input,
	punctuated::Punctuated,
};

/// A Depot volume address.
///
/// `#[volume(key = [..], name = TAG, placement = expr)]`: `name` names one of several volumes
/// that share a partition key, and `placement` is an expression over the fields, bound by
/// reference, that places the volume; without it the volume is placed by its key's hash. A field
/// marked `#[group]` holds the volume group, packed with the `GROUPED` tag in front of
/// everything else.
#[proc_macro_derive(VolumeKey, attributes(volume, group))]
pub fn derive_volume_key(input: TokenStream) -> TokenStream {
	expand(input, Family::Volume)
}

/// A typed application key inside a Depot volume.
///
/// `#[kv(key = [..], placement = expr, value = codec)]`, where `codec` is
/// `vbare(Wrapper, VERSION)`, `vbare_selected(Wrapper)`, `bytes` or `unit`. Without a `value` the
/// key has no value codec, as a range prefix does.
#[proc_macro_derive(VolumeKvKey, attributes(kv, group))]
pub fn derive_volume_kv_key(input: TokenStream) -> TokenStream {
	expand(input, Family::Kv)
}

/// A physical SlateDB key, `#[slatedb(key = [..], value = codec)]`, or a raw scan prefix,
/// `#[slatedb(prefix = [..], next = Type)]`. `next` restricts the scan to rows whose following
/// component starts with that type's marker.
#[proc_macro_derive(SlatedbKey, attributes(slatedb))]
pub fn derive_slatedb_key(input: TokenStream) -> TokenStream {
	expand(input, Family::Slatedb)
}

/// An exact-match UPS subject, `#[ups(subject = [..])]`. The struct holds the packed subject in
/// a `Vec<u8>` field marked `#[subject]`, filled by `new`.
#[proc_macro_derive(UpsSubject, attributes(ups, subject))]
pub fn derive_ups_subject(input: TokenStream) -> TokenStream {
	expand(input, Family::Ups)
}

/// An S3 object key, `#[s3(key = [..], value = codec)]`.
#[proc_macro_derive(S3Key, attributes(s3))]
pub fn derive_s3_key(input: TokenStream) -> TokenStream {
	expand(input, Family::S3)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
	Volume,
	Kv,
	Slatedb,
	Ups,
	S3,
}

impl Family {
	fn attribute(self) -> &'static str {
		match self {
			Family::Volume => "volume",
			Family::Kv => "kv",
			Family::Slatedb => "slatedb",
			Family::Ups => "ups",
			Family::S3 => "s3",
		}
	}

	/// The options this family's attribute takes. `name` exists only for volume addresses,
	/// `placement` only for partition keys, `prefix` and `next` only for SlateDB scan prefixes,
	/// and a UPS subject has nothing but its `subject`.
	fn options(self) -> &'static [&'static str] {
		match self {
			Family::Volume => &["key", "name", "placement"],
			Family::Kv => &["key", "placement", "value"],
			Family::Slatedb => &["key", "prefix", "next", "value"],
			Family::Ups => &["subject"],
			Family::S3 => &["key", "value"],
		}
	}

	fn tag_space(self) -> Ident {
		let space = match self {
			Family::Volume | Family::Kv => "partition",
			Family::Slatedb => "slatedb",
			Family::Ups => "ups",
			Family::S3 => "s3",
		};
		Ident::new(space, Span::call_site())
	}
}

fn expand(input: TokenStream, family: Family) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	match expand_key(&input, family) {
		Ok(tokens) => tokens.into(),
		Err(error) => error.to_compile_error().into(),
	}
}

/// One component of a key's packed form.
enum Part {
	Field(Ident, Type),
	Tag(Ident),
}

/// A value codec.
enum Value {
	Vbare(Type, Expr),
	VbareSelected(Type),
	Bytes,
	Unit,
}

/// The parsed key attribute.
#[derive(Default)]
struct Spec {
	/// Every option the attribute names, in order, so each family can refuse the ones it does
	/// not take.
	options: Vec<Ident>,
	key: Option<Vec<Ident>>,
	prefix: Option<Vec<Ident>>,
	name: Option<Vec<Ident>>,
	placement: Option<Expr>,
	value: Option<Value>,
	next: Option<Type>,
}

impl Parse for Spec {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut spec = Spec::default();
		while !input.is_empty() {
			let option: Ident = input.parse()?;
			if spec.options.iter().any(|seen| *seen == option) {
				return Err(Error::new(
					option.span(),
					format!("duplicate key option `{option}`"),
				));
			}
			spec.options.push(option.clone());
			match option.to_string().as_str() {
				"key" | "subject" => {
					input.parse::<Token![=]>()?;
					spec.key = Some(parse_idents(input)?);
				}
				"prefix" => {
					input.parse::<Token![=]>()?;
					spec.prefix = Some(parse_idents(input)?);
				}
				"name" => {
					input.parse::<Token![=]>()?;
					let names = if input.peek(syn::token::Bracket) {
						parse_idents(input)?
					} else {
						vec![input.parse()?]
					};
					spec.name = Some(names);
				}
				"placement" => {
					input.parse::<Token![=]>()?;
					spec.placement = Some(input.parse()?);
				}
				"next" => {
					input.parse::<Token![=]>()?;
					spec.next = Some(input.parse()?);
				}
				"value" => {
					input.parse::<Token![=]>()?;
					spec.value = Some(parse_value(input)?);
				}
				other => {
					return Err(Error::new(
						option.span(),
						format!("unknown key option `{other}`"),
					));
				}
			}
			if !input.is_empty() {
				input.parse::<Token![,]>()?;
			}
		}
		Ok(spec)
	}
}

fn parse_idents(input: ParseStream) -> Result<Vec<Ident>> {
	let content;
	bracketed!(content in input);
	let idents = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
	Ok(idents.into_iter().collect())
}

fn parse_value(input: ParseStream) -> Result<Value> {
	let kind: Ident = input.parse()?;
	match kind.to_string().as_str() {
		"vbare" => {
			let content;
			syn::parenthesized!(content in input);
			let wrapper: Type = content.parse()?;
			content.parse::<Token![,]>()?;
			let version: Expr = content.parse()?;
			Ok(Value::Vbare(wrapper, version))
		}
		"vbare_selected" => {
			let content;
			syn::parenthesized!(content in input);
			Ok(Value::VbareSelected(content.parse()?))
		}
		"bytes" => Ok(Value::Bytes),
		"unit" => Ok(Value::Unit),
		other => Err(Error::new(
			kind.span(),
			format!("unknown value codec `{other}`"),
		)),
	}
}

/// The fields of the key struct, split by role.
struct KeyFields {
	/// Every field in declaration order, which is the order `new` takes them in.
	all: Vec<(Ident, Type)>,
	group: Option<Ident>,
	subject: Option<Ident>,
}

fn key_fields(input: &DeriveInput) -> Result<KeyFields> {
	let Data::Struct(data) = &input.data else {
		return Err(Error::new_spanned(input, "formal keys are structs"));
	};
	let mut fields = KeyFields {
		all: Vec::new(),
		group: None,
		subject: None,
	};
	match &data.fields {
		Fields::Named(named) => {
			for field in &named.named {
				let ident = field.ident.clone().expect("named field");
				if field.attrs.iter().any(|attr| attr.path().is_ident("group")) {
					if fields.group.is_some() {
						return Err(Error::new(ident.span(), "a key belongs to one group"));
					}
					fields.group = Some(ident.clone());
				}
				if field
					.attrs
					.iter()
					.any(|attr| attr.path().is_ident("subject"))
				{
					fields.subject = Some(ident.clone());
					continue;
				}
				fields.all.push((ident, field.ty.clone()));
			}
		}
		Fields::Unit => {}
		Fields::Unnamed(_) => {
			return Err(Error::new_spanned(
				input,
				"formal keys have named fields or none",
			));
		}
	}
	Ok(fields)
}

fn resolve_parts(idents: &[Ident], fields: &KeyFields) -> Result<Vec<Part>> {
	let mut parts = Vec::new();
	let mut used = Vec::new();
	for ident in idents {
		if Some(ident) == fields.group.as_ref() {
			return Err(Error::new(
				ident.span(),
				"the `#[group]` field is packed in front of the key and cannot be a component",
			));
		}
		match fields.all.iter().find(|(field, _)| field == ident) {
			Some((field, ty)) => {
				if used.contains(field) {
					return Err(Error::new(ident.span(), "a field is packed once"));
				}
				used.push(field.clone());
				parts.push(Part::Field(field.clone(), ty.clone()));
			}
			None => parts.push(Part::Tag(ident.clone())),
		}
	}
	for (field, _) in &fields.all {
		if Some(field) != fields.group.as_ref() && !used.contains(field) {
			return Err(Error::new(
				field.span(),
				"every field of a formal key is one of its components",
			));
		}
	}
	Ok(parts)
}

fn expand_key(input: &DeriveInput, family: Family) -> Result<TokenStream2> {
	let mut attributes = input
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident(family.attribute()));
	let attribute = attributes.next().ok_or_else(|| {
		Error::new_spanned(
			&input.ident,
			format!("missing #[{}(..)] attribute", family.attribute()),
		)
	})?;
	if let Some(second) = attributes.next() {
		return Err(Error::new_spanned(
			second,
			format!("a key has one #[{}(..)] attribute", family.attribute()),
		));
	}
	let spec: Spec = attribute.parse_args()?;
	validate_options(&spec, family)?;
	let fields = key_fields(input)?;

	if fields.group.is_some() && !matches!(family, Family::Volume | Family::Kv) {
		return Err(Error::new_spanned(
			&input.ident,
			"only volume keys belong to a group",
		));
	}

	let name = &input.ident;
	let constructor = constructor(name, &fields, family, &spec)?;
	let body = match family {
		Family::Volume | Family::Kv => binary_key(name, &fields, family, &spec)?,
		Family::Slatedb => match &spec.prefix {
			Some(prefix) => slatedb_prefix(name, &fields, prefix, &spec)?,
			None => binary_key(name, &fields, family, &spec)?,
		},
		Family::Ups => quote! {},
		Family::S3 => s3_key(name, &fields, &spec)?,
	};
	let value = value_codec(name, &spec);

	Ok(quote! {
		#constructor
		#body
		#value
	})
}

/// Refuses options the family does not take and combinations that would generate a key other
/// than the one the attribute reads as.
fn validate_options(spec: &Spec, family: Family) -> Result<()> {
	for option in &spec.options {
		if !family.options().contains(&option.to_string().as_str()) {
			return Err(Error::new(
				option.span(),
				format!(
					"`{option}` is not an option of #[{}(..)]",
					family.attribute()
				),
			));
		}
	}

	let option = |name: &str| spec.options.iter().find(|option| *option == name);
	if family == Family::Slatedb {
		match (option("key"), option("prefix")) {
			(Some(_), Some(prefix)) => {
				return Err(Error::new(
					prefix.span(),
					"a SlateDB key has either `key` or `prefix`",
				));
			}
			(None, None) => {
				return Err(Error::new(
					Span::call_site(),
					"a SlateDB key needs `key = [..]` or `prefix = [..]`",
				));
			}
			(Some(_), None) | (None, Some(_)) => {}
		}
		if let (Some(next), None) = (option("next"), option("prefix")) {
			return Err(Error::new(next.span(), "`next` only follows a `prefix`"));
		}
		if let (Some(value), Some(_)) = (option("value"), option("prefix")) {
			return Err(Error::new(
				value.span(),
				"a scan prefix holds no value; only a `key` has a value codec",
			));
		}
	}
	Ok(())
}

/// `new`, taking every field in declaration order. A UPS subject packs its subject here.
fn constructor(
	name: &Ident,
	fields: &KeyFields,
	family: Family,
	spec: &Spec,
) -> Result<TokenStream2> {
	let params = fields.all.iter().map(|(field, ty)| quote! { #field: #ty });
	let names = fields.all.iter().map(|(field, _)| field);

	if family != Family::Ups {
		return Ok(quote! {
			impl #name {
				pub fn new(#(#params),*) -> Self {
					Self { #(#names),* }
				}
			}
		});
	}

	let subject = fields.subject.as_ref().ok_or_else(|| {
		Error::new_spanned(
			name,
			"a UPS subject holds its bytes in a `#[subject]` field",
		)
	})?;
	let idents = spec
		.key
		.as_ref()
		.ok_or_else(|| Error::new_spanned(name, "missing `subject = [..]`"))?;
	let parts = resolve_parts(idents, fields)?;
	let space = family.tag_space();
	let pushes = parts.iter().map(|part| {
		binary_push(
			part,
			&space,
			quote! { &mut __key.#subject },
			quote! { __key },
		)
	});
	Ok(quote! {
		impl #name {
			pub fn new(#(#params),*) -> Self {
				// Subjects are a few tags and an identifier, so one allocation holds them.
				let mut __key = Self {
					#(#names,)*
					#subject: ::std::vec::Vec::with_capacity(32),
				};
				#(#pushes)*
				__key
			}
		}

		impl ::std::convert::AsRef<[u8]> for #name {
			fn as_ref(&self) -> &[u8] {
				&self.#subject
			}
		}
	})
}

/// Packs one component of a binary key from `owner`'s field into `out`.
fn binary_push(part: &Part, space: &Ident, out: TokenStream2, owner: TokenStream2) -> TokenStream2 {
	match part {
		Part::Field(field, _) => quote! {
			::rivet_keys::keys::codec::BinaryComponent::pack_into(&#owner.#field, #out);
		},
		Part::Tag(tag) => quote! {
			::rivet_keys::keys::codec::BinaryComponent::pack_into(
				&(::rivet_keys::keys::tags::#space::#tag as u64),
				#out,
			);
		},
	}
}

/// Binds every field by reference, for a placement expression.
fn field_bindings(fields: &KeyFields) -> TokenStream2 {
	let names = fields.all.iter().map(|(field, _)| field);
	quote! {
		#[allow(unused_variables)]
		let Self { #(#names,)* .. } = self;
	}
}

/// `KeyAddress`, `SubspaceRange` and tuple unpacking for a tuple-encoded key, and for a volume
/// key its volume address.
fn binary_key(
	name: &Ident,
	fields: &KeyFields,
	family: Family,
	spec: &Spec,
) -> Result<TokenStream2> {
	let idents = spec
		.key
		.as_ref()
		.ok_or_else(|| Error::new_spanned(name, "missing `key = [..]`"))?;
	let parts = resolve_parts(idents, fields)?;
	let space = family.tag_space();

	let pack_group = fields.group.as_ref().map(|group| {
		quote! { ::rivet_keys::keys::codec::pack_group(&self.#group, __out); }
	});
	let pack_placement = spec.placement.as_ref().map(|placement| {
		let bindings = field_bindings(fields);
		quote! {
			let __placement: ::rivet_keys::keys::codec::Placement = {
				#bindings
				#placement
			};
			::rivet_keys::keys::codec::pack_placement(&__placement, __out);
		}
	});
	let pushes = parts
		.iter()
		.map(|part| binary_push(part, &space, quote! { __out }, quote! { self }));

	let read_group = fields.group.as_ref().map(|group| {
		quote! {
			::rivet_keys::keys::codec::expect_tag(
				&mut __remaining,
				__depth,
				::rivet_keys::keys::tags::partition::GROUPED,
			)?;
			let #group = <::rivet_keys::keys::codec::GroupId as ::rivet_keys::keys::codec::BinaryComponent>::unpack_from(&mut __remaining, __depth)?;
		}
	});
	let skip_placement = spec.placement.as_ref().map(|_| {
		quote! { ::rivet_keys::keys::codec::skip_placement(&mut __remaining, __depth)?; }
	});
	let reads = parts.iter().map(|part| match part {
		Part::Field(field, ty) => quote! {
			let #field = <#ty as ::rivet_keys::keys::codec::BinaryComponent>::unpack_from(&mut __remaining, __depth)?;
		},
		Part::Tag(tag) => quote! {
			::rivet_keys::keys::codec::expect_tag(
				&mut __remaining,
				__depth,
				::rivet_keys::keys::tags::#space::#tag,
			)?;
		},
	});
	let names = fields.all.iter().map(|(field, _)| field);

	let volume = match family {
		Family::Volume => Some(volume_address(name, spec)),
		Family::Kv | Family::Slatedb | Family::Ups | Family::S3 => None,
	};

	Ok(quote! {
		impl ::rivet_keys::keys::codec::KeyAddress for #name {
			type Packed = ::rivet_keys::__private::bytes::Bytes;
			type Input = [u8];

			fn pack(&self) -> Self::Packed {
				::rivet_keys::keys::codec::pack_bytes(|__out| {
					#pack_group
					#pack_placement
					#(#pushes)*
				})
			}

			fn unpack(__input: &[u8]) -> ::rivet_keys::__private::anyhow::Result<Self> {
				let __key: Self = ::rivet_keys::__private::foundationdb_tuple::unpack(__input)?;
				// Repacking derives any placement again, so a key with a wrong one is refused.
				::rivet_keys::__private::anyhow::ensure!(
					::rivet_keys::keys::codec::KeyAddress::pack(&__key).as_ref() == __input,
					"noncanonical binary key"
				);
				Ok(__key)
			}
		}

		impl ::rivet_keys::keys::codec::SubspaceRange for #name {
			fn range(&self) -> ::std::ops::Range<::std::vec::Vec<u8>> {
				let mut start =
					::rivet_keys::keys::codec::KeyAddress::pack(self).to_vec();
				let mut end = start.clone();
				start.push(0x00);
				end.push(0xff);
				start..end
			}
		}

		impl<'de> ::rivet_keys::__private::foundationdb_tuple::TupleUnpack<'de> for #name {
			fn unpack(
				__input: &'de [u8],
				__depth: ::rivet_keys::__private::foundationdb_tuple::TupleDepth,
			) -> ::rivet_keys::__private::foundationdb_tuple::PackResult<(&'de [u8], Self)> {
				let mut __remaining = __input;
				#read_group
				#skip_placement
				#(#reads)*
				Ok((__remaining, Self { #(#names),* }))
			}
		}

		#volume
	})
}

/// `volume_key` and `from_volume_key` of a volume address.
fn volume_address(name: &Ident, spec: &Spec) -> TokenStream2 {
	match &spec.name {
		Some(tags) => {
			let pushes = tags.iter().map(|tag| {
				quote! {
					::rivet_keys::keys::codec::BinaryComponent::pack_into(
						&(::rivet_keys::keys::tags::partition::#tag as u64),
						&mut volume_name,
					);
				}
			});
			let volume_name = quote! {
				let mut volume_name = ::std::vec::Vec::new();
				#(#pushes)*
			};
			quote! {
				impl #name {
					pub fn volume_key(&self) -> ::rivet_keys::keys::VolumeKey {
						#volume_name
						::rivet_keys::keys::VolumeKey::named(
							::rivet_keys::keys::codec::KeyAddress::pack(self).to_vec(),
							volume_name,
						)
					}

					pub fn from_volume_key(
						key: &::rivet_keys::keys::VolumeKey,
					) -> ::rivet_keys::__private::anyhow::Result<Self> {
						#volume_name
						::rivet_keys::__private::anyhow::ensure!(
							key.name.as_deref() == Some(volume_name.as_slice()),
							"this formal volume address has a different name"
						);
						<Self as ::rivet_keys::keys::codec::KeyAddress>::unpack(&key.partition_key)
					}
				}
			}
		}
		None => quote! {
			impl #name {
				pub fn volume_key(&self) -> ::rivet_keys::keys::VolumeKey {
					::rivet_keys::keys::VolumeKey::unnamed(
						::rivet_keys::keys::codec::KeyAddress::pack(self).to_vec(),
					)
				}

				pub fn from_volume_key(
					key: &::rivet_keys::keys::VolumeKey,
				) -> ::rivet_keys::__private::anyhow::Result<Self> {
					::rivet_keys::__private::anyhow::ensure!(
						key.name.is_none(),
						"this formal volume address has no name"
					);
					<Self as ::rivet_keys::keys::codec::KeyAddress>::unpack(&key.partition_key)
				}
			}
		},
	}
}

/// `PrefixAddress` of a SlateDB scan prefix.
fn slatedb_prefix(
	name: &Ident,
	fields: &KeyFields,
	idents: &[Ident],
	spec: &Spec,
) -> Result<TokenStream2> {
	let parts = resolve_parts(idents, fields)?;
	let pushes = parts.iter().map(|part| match part {
		Part::Field(field, _) => quote! {
			::rivet_keys::keys::codec::PrefixComponent::pack_prefix_into(&self.#field, __out);
		},
		Part::Tag(tag) => quote! {
			::rivet_keys::keys::codec::BinaryComponent::pack_into(
				&(::rivet_keys::keys::tags::slatedb::#tag as u64),
				__out,
			);
		},
	});
	let next = spec.next.as_ref().map(|next| {
		quote! { <#next as ::rivet_keys::keys::codec::PrefixStart>::pack_start(__out); }
	});
	Ok(quote! {
		impl ::rivet_keys::keys::codec::PrefixAddress for #name {
			fn pack(&self) -> ::rivet_keys::__private::bytes::Bytes {
				::rivet_keys::keys::codec::pack_bytes(|__out| {
					#(#pushes)*
					#next
				})
			}
		}
	})
}

/// `KeyAddress` of an S3 object key.
fn s3_key(name: &Ident, fields: &KeyFields, spec: &Spec) -> Result<TokenStream2> {
	let idents = spec
		.key
		.as_ref()
		.ok_or_else(|| Error::new_spanned(name, "missing `key = [..]`"))?;
	let parts = resolve_parts(idents, fields)?;
	let pushes = parts.iter().map(|part| match part {
		Part::Field(field, _) => quote! {
			__parts.push(::rivet_keys::keys::codec::TextComponent::to_text(&self.#field));
		},
		Part::Tag(tag) => quote! {
			__parts.push(::rivet_keys::keys::tags::s3::#tag.to_owned());
		},
	});
	let reads = parts.iter().map(|part| {
		let next = quote! {
			__parts
				.next()
				.ok_or_else(|| ::rivet_keys::__private::anyhow::anyhow!("missing key component"))?
		};
		match part {
			Part::Field(field, ty) => quote! {
				let #field = <#ty as ::rivet_keys::keys::codec::TextComponent>::from_text(&#next)?;
			},
			Part::Tag(tag) => quote! {
				::rivet_keys::__private::anyhow::ensure!(
					__parts.next().as_deref() == Some(::rivet_keys::keys::tags::s3::#tag),
					"wrong key tag"
				);
			},
		}
	});
	let names = fields.all.iter().map(|(field, _)| field);
	Ok(quote! {
		impl ::rivet_keys::keys::codec::KeyAddress for #name {
			type Packed = ::rivet_keys::__private::object_store::path::Path;
			type Input = ::rivet_keys::__private::object_store::path::Path;

			fn pack(&self) -> Self::Packed {
				let mut __parts = ::std::vec::Vec::<::std::string::String>::new();
				#(#pushes)*
				::rivet_keys::__private::object_store::path::Path::from_iter(__parts)
			}

			fn unpack(__input: &Self::Input) -> ::rivet_keys::__private::anyhow::Result<Self> {
				let mut __parts = ::rivet_keys::keys::codec::s3_parts(__input)?.into_iter();
				#(#reads)*
				::rivet_keys::__private::anyhow::ensure!(__parts.next().is_none(), "trailing S3 path components");
				let __key = Self { #(#names),* };
				::rivet_keys::__private::anyhow::ensure!(
					::rivet_keys::keys::codec::KeyAddress::pack(&__key) == *__input,
					"noncanonical S3 path"
				);
				Ok(__key)
			}
		}
	})
}

/// `FormalKey` with the key's value codec, when it has one.
fn value_codec(name: &Ident, spec: &Spec) -> Option<TokenStream2> {
	let codec = match spec.value.as_ref()? {
		Value::Vbare(wrapper, version) => quote! {
			::rivet_keys::keys::codec::VbareValue<#wrapper, { #version }>
		},
		Value::VbareSelected(wrapper) => quote! {
			::rivet_keys::keys::codec::SelectedVbareValue<#wrapper>
		},
		Value::Bytes => quote! { ::rivet_keys::keys::codec::BytesValue },
		Value::Unit => quote! { ::rivet_keys::keys::codec::UnitValue },
	};
	Some(quote! {
		impl ::rivet_keys::keys::codec::FormalKey for #name {
			type Codec = #codec;
		}
	})
}
