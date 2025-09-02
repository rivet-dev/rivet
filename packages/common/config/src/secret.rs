use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Wraps a given value in a container that does not implement `Debug` or `Display` in order to
/// prevent accidentally logging the inner secret.
///
/// In order to access the secret, explicitly use `read()`.
#[derive(Clone)]
pub struct Secret<T>(T)
where
	T: Clone;

impl<T> Secret<T>
where
	T: Clone,
{
	pub fn new(v: T) -> Self {
		Self(v)
	}

	/// Read the secret.
	pub fn read(&self) -> &T {
		&self.0
	}
}

impl<T> fmt::Debug for Secret<T>
where
	T: Clone,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Secret").finish()
	}
}

impl<T> Serialize for Secret<T>
where
	T: Clone + Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de, T> Deserialize<'de> for Secret<T>
where
	T: Clone + Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		T::deserialize(deserializer).map(Secret)
	}
}

impl<T> JsonSchema for Secret<T>
where
	T: Clone + JsonSchema,
{
	fn schema_name() -> String {
		format!("Secret<{}>", T::schema_name())
	}

	fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
		T::json_schema(generator)
	}
}
