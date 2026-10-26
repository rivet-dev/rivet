use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use ciborium::Value;
use serde::de::{self, DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};

use crate::{actor::Actor, context::Ctx};

pub const TUPLE_ARITY_MAX: usize = 128;
pub(crate) type BoxActionFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send>>;

/// An action a client can invoke on an actor. Arguments and `Output` must be JSON-compatible;
/// they are carried through the JSON value model to match the TypeScript runtime.
pub trait Action: serde::Serialize + DeserializeOwned + Send + Sync + 'static {
	type Output: serde::Serialize + DeserializeOwned + Send + 'static;

	const NAME: &'static str;
}

pub fn encode_positional<T: Serialize>(value: &T) -> Result<Vec<u8>> {
	encode_varargs(value, "action args")
}

pub(crate) fn encode_varargs<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>> {
	let value = to_json_value(value, label)?;
	let value = positional_value(value);
	encode_json_as_cbor(&value, label)
}

/// Serialize `value` into the JSON-shaped value model shared with the TypeScript runtime.
/// Values with no JSON representation are rejected: `serde_json` rejects out-of-range integers
/// and non-string map keys, and `reject_non_finite_floats` rejects `NaN`/infinity.
pub(crate) fn to_json_value<T: Serialize>(value: &T, label: &str) -> Result<serde_json::Value> {
	reject_non_finite_floats(value)
		.with_context(|| format!("encode {label} as a json-compatible value"))?;
	serde_json::to_value(value)
		.with_context(|| format!("encode {label} as a json-compatible value"))
}

/// Encode a JSON-shaped value as CBOR bytes for the wire.
pub(crate) fn encode_json_as_cbor(value: &serde_json::Value, label: &str) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded)
		.with_context(|| format!("encode {label} as cbor"))?;
	Ok(encoded)
}

/// Convert a decoded CBOR envelope back into a JSON-shaped value, the inverse of
/// [`encode_json_as_cbor`].
pub(crate) fn cbor_to_json(value: Value) -> Result<serde_json::Value> {
	use serde_json::Value as JsonValue;

	Ok(match value {
		Value::Null => JsonValue::Null,
		Value::Bool(value) => JsonValue::Bool(value),
		Value::Integer(value) => integer_to_json(i128::from(value))?,
		Value::Float(value) => JsonValue::Number(
			serde_json::Number::from_f64(value)
				.context("cbor float cannot be represented as json")?,
		),
		Value::Bytes(value) => JsonValue::Array(value.into_iter().map(JsonValue::from).collect()),
		Value::Text(value) => JsonValue::String(value),
		Value::Array(values) => JsonValue::Array(
			values
				.into_iter()
				.map(cbor_to_json)
				.collect::<Result<Vec<_>>>()?,
		),
		Value::Map(entries) => {
			let mut object = serde_json::Map::new();
			for (key, value) in entries {
				let Value::Text(key) = key else {
					anyhow::bail!("cbor map key cannot be represented as a json object key");
				};
				object.insert(key, cbor_to_json(value)?);
			}
			JsonValue::Object(object)
		}
		Value::Tag(_, value) => cbor_to_json(*value)?,
		_ => anyhow::bail!("cbor value cannot be represented as json"),
	})
}

fn integer_to_json(value: i128) -> Result<serde_json::Value> {
	if let Ok(value) = i64::try_from(value) {
		return Ok(serde_json::Value::Number(value.into()));
	}
	if let Ok(value) = u64::try_from(value) {
		return Ok(serde_json::Value::Number(value.into()));
	}
	anyhow::bail!("cbor integer cannot be represented as json number")
}

/// Reject non-finite floats (`NaN`/infinity), which `serde_json::to_value` would silently
/// coerce to `null`.
fn reject_non_finite_floats<T: Serialize>(value: &T) -> Result<()> {
	value
		.serialize(finite::Checker)
		.map_err(anyhow::Error::from)
}

/// A `serde::Serializer` that produces no output and errors on non-finite floats.
mod finite {
	use std::fmt;

	use serde::Serialize;
	use serde::ser::{
		self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
		SerializeTupleStruct, SerializeTupleVariant,
	};

	#[derive(Debug)]
	pub(super) struct Error;

	impl fmt::Display for Error {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			f.write_str(
				"value contains a non-finite float (NaN or infinity), which is not json-compatible",
			)
		}
	}

	impl std::error::Error for Error {}

	impl ser::Error for Error {
		fn custom<T: fmt::Display>(_msg: T) -> Self {
			Error
		}
	}

	pub(super) struct Checker;

	pub(super) struct Nested;

	impl ser::Serializer for Checker {
		type Ok = ();
		type Error = Error;
		type SerializeSeq = Nested;
		type SerializeTuple = Nested;
		type SerializeTupleStruct = Nested;
		type SerializeTupleVariant = Nested;
		type SerializeMap = Nested;
		type SerializeStruct = Nested;
		type SerializeStructVariant = Nested;

		fn serialize_f32(self, value: f32) -> Result<(), Error> {
			if value.is_finite() {
				Ok(())
			} else {
				Err(Error)
			}
		}

		fn serialize_f64(self, value: f64) -> Result<(), Error> {
			if value.is_finite() {
				Ok(())
			} else {
				Err(Error)
			}
		}

		fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Error> {
			value.serialize(self)
		}

		fn serialize_newtype_struct<T: Serialize + ?Sized>(
			self,
			_name: &'static str,
			value: &T,
		) -> Result<(), Error> {
			value.serialize(self)
		}

		fn serialize_newtype_variant<T: Serialize + ?Sized>(
			self,
			_name: &'static str,
			_index: u32,
			_variant: &'static str,
			value: &T,
		) -> Result<(), Error> {
			value.serialize(self)
		}

		fn serialize_seq(self, _len: Option<usize>) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_tuple(self, _len: usize) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_tuple_variant(
			self,
			_name: &'static str,
			_index: u32,
			_variant: &'static str,
			_len: usize,
		) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_map(self, _len: Option<usize>) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_struct_variant(
			self,
			_name: &'static str,
			_index: u32,
			_variant: &'static str,
			_len: usize,
		) -> Result<Nested, Error> {
			Ok(Nested)
		}

		fn serialize_bool(self, _value: bool) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_i8(self, _value: i8) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_i16(self, _value: i16) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_i32(self, _value: i32) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_i64(self, _value: i64) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_i128(self, _value: i128) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_u8(self, _value: u8) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_u16(self, _value: u16) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_u32(self, _value: u32) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_u64(self, _value: u64) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_u128(self, _value: u128) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_char(self, _value: char) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_str(self, _value: &str) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_bytes(self, _value: &[u8]) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_none(self) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_unit(self) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Error> {
			Ok(())
		}

		fn serialize_unit_variant(
			self,
			_name: &'static str,
			_index: u32,
			_variant: &'static str,
		) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeSeq for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeTuple for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeTupleStruct for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeTupleVariant for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeMap for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
			key.serialize(Checker)
		}

		fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeStruct for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_field<T: Serialize + ?Sized>(
			&mut self,
			_key: &'static str,
			value: &T,
		) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}

	impl SerializeStructVariant for Nested {
		type Ok = ();
		type Error = Error;

		fn serialize_field<T: Serialize + ?Sized>(
			&mut self,
			_key: &'static str,
			value: &T,
		) -> Result<(), Error> {
			value.serialize(Checker)
		}

		fn end(self) -> Result<(), Error> {
			Ok(())
		}
	}
}

pub fn decode_positional<T: DeserializeOwned>(args: &[u8]) -> Result<T> {
	let value = if args.is_empty() {
		Value::Array(Vec::new())
	} else {
		ciborium::from_reader(Cursor::new(args)).context("decode action args from cbor")?
	};
	let value = match value {
		Value::Null => Value::Array(Vec::new()),
		value => value,
	};

	match decode_value::<T>(&value) {
		Ok(value) => Ok(value),
		Err(first_error) => match &value {
			Value::Array(values) if values.is_empty() => decode_value(&Value::Null)
				.or_else(|_| Err(first_error).context("decode positional action args as unit")),
			Value::Array(values) if values.len() == 1 => decode_value(&values[0])
				.or_else(|_| Err(first_error).context("decode positional action args as newtype")),
			_ => Err(first_error).context("decode positional action args"),
		},
	}
}

/// Normalize a serialized value into the positional argument array, matching the TypeScript
/// runtime: an array stays positional, unit becomes no arguments, anything else is one argument.
fn positional_value(value: serde_json::Value) -> serde_json::Value {
	use serde_json::Value;

	match value {
		Value::Array(values) => Value::Array(values),
		Value::Null => Value::Array(Vec::new()),
		other => Value::Array(vec![other]),
	}
}

fn decode_value<T: DeserializeOwned>(value: &Value) -> Result<T> {
	crate::event::deserialize_cbor_value(value.clone())
		.map_err(|error| anyhow::anyhow!(error.to_string()))
		.context("decode positional action args from cbor")
}

pub trait Handles<A: Action>: Actor + Sized {
	type Future: Future<Output = Result<A::Output>> + Send + 'static;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, action: A) -> Self::Future;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEntry<A: Actor> {
	pub name: &'static str,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> ActionEntry<A> {
	pub const fn new(name: &'static str) -> Self {
		Self {
			name,
			_p: PhantomData,
		}
	}
}

pub trait ActionSet<A: Actor>: Send + Sync + 'static {
	fn entries() -> Vec<ActionEntry<A>>;
	fn dispatch(actor: Arc<A>, ctx: Ctx<A>, name: &str, args: &[u8]) -> Option<BoxActionFuture>;
}

impl<A: Actor> ActionSet<A> for () {
	fn entries() -> Vec<ActionEntry<A>> {
		Vec::new()
	}

	fn dispatch(
		_actor: Arc<A>,
		_ctx: Ctx<A>,
		_name: &str,
		_args: &[u8],
	) -> Option<BoxActionFuture> {
		None
	}
}

macro_rules! impl_action_set {
	($($action:ident),+) => {
		impl<Act, $($action),+> ActionSet<Act> for ($($action,)+)
		where
			Act: Actor + $(Handles<$action> +)+,
			$($action: Action,)+
		{
			fn entries() -> Vec<ActionEntry<Act>> {
				vec![$(ActionEntry::new(<$action as Action>::NAME)),+]
			}

			fn dispatch(
				actor: Arc<Act>,
				ctx: Ctx<Act>,
				name: &str,
				args: &[u8],
			) -> Option<BoxActionFuture> {
				$(
					if name == <$action as Action>::NAME {
						let args = args.to_vec();
						return Some(Box::pin(async move {
							let action = decode_positional::<$action>(&args).with_context(|| {
								format!("decode action '{}' args", <$action as Action>::NAME)
							})?;
							let output = <Act as Handles<$action>>::handle(actor, ctx, action).await?;
							encode_cbor(&output, "action response")
						}));
					}
				)+
				None
			}
		}
	};
}

macro_rules! impl_action_sets {
	(@accum [$($action:ident),*]) => {};
	(@accum [$($action:ident),*] $next:ident $(, $rest:ident)*) => {
		impl_action_set!($($action,)* $next);
		impl_action_sets!(@accum [$($action,)* $next] $($rest),*);
	};
	($first:ident $(, $rest:ident)*) => {
		impl_action_sets!(@accum [] $first $(, $rest)*);
	};
}

macro_rules! with_action_type_params {
	($callback:ident) => {
		$callback!(
			A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, A16, A17, A18,
			A19, A20, A21, A22, A23, A24, A25, A26, A27, A28, A29, A30, A31, A32, A33, A34, A35,
			A36, A37, A38, A39, A40, A41, A42, A43, A44, A45, A46, A47, A48, A49, A50, A51, A52,
			A53, A54, A55, A56, A57, A58, A59, A60, A61, A62, A63, A64, A65, A66, A67, A68, A69,
			A70, A71, A72, A73, A74, A75, A76, A77, A78, A79, A80, A81, A82, A83, A84, A85, A86,
			A87, A88, A89, A90, A91, A92, A93, A94, A95, A96, A97, A98, A99, A100, A101, A102,
			A103, A104, A105, A106, A107, A108, A109, A110, A111, A112, A113, A114, A115, A116,
			A117, A118, A119, A120, A121, A122, A123, A124, A125, A126, A127
		);
	};
}

with_action_type_params!(impl_action_sets);

fn encode_cbor<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>> {
	let value = to_json_value(value, label)?;
	encode_json_as_cbor(&value, label)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw;

impl<'de> Deserialize<'de> for Raw {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let _ = de::IgnoredAny::deserialize(deserializer)?;
		Err(de::Error::custom(
			"rivetkit::action::Raw cannot be deserialized; use Action::raw_args() or Action::decode_as(...) instead",
		))
	}
}


