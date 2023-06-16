use std::fmt::Debug;

/// A type that can be serialized in to a key that can be used in the cache.
pub trait CacheKey: Clone + Debug + PartialEq {
	fn cache_key(&self) -> String;
}

impl<'a> CacheKey for &'a str {
	fn cache_key(&self) -> String {
		self.replace("\\", "\\\\").replace(":", "\\")
	}
}

impl CacheKey for String {
	fn cache_key(&self) -> String {
		self.as_str().cache_key()
	}
}

impl<V0: CacheKey> CacheKey for (V0,) {
	fn cache_key(&self) -> String {
		self.0.cache_key()
	}
}

impl<V0: CacheKey, V1: CacheKey> CacheKey for (V0, V1) {
	fn cache_key(&self) -> String {
		format!("{}:{}", self.0.cache_key(), self.1.cache_key())
	}
}

impl<V0: CacheKey, V1: CacheKey, V2: CacheKey> CacheKey for (V0, V1, V2) {
	fn cache_key(&self) -> String {
		format!(
			"{}:{}:{}",
			self.0.cache_key(),
			self.1.cache_key(),
			self.2.cache_key()
		)
	}
}

impl<V0: CacheKey, V1: CacheKey, V2: CacheKey, V3: CacheKey> CacheKey for (V0, V1, V2, V3) {
	fn cache_key(&self) -> String {
		format!(
			"{}:{}:{}:{}",
			self.0.cache_key(),
			self.1.cache_key(),
			self.2.cache_key(),
			self.3.cache_key()
		)
	}
}

macro_rules! impl_to_string {
	($type_name:ty) => {
		impl CacheKey for $type_name {
			fn cache_key(&self) -> String {
				self.to_string()
			}
		}
	};
}

impl_to_string!(uuid::Uuid);
impl_to_string!(bool);
impl_to_string!(char);
impl_to_string!(u8);
impl_to_string!(u16);
impl_to_string!(u32);
impl_to_string!(u64);
impl_to_string!(u128);
impl_to_string!(usize);
impl_to_string!(i8);
impl_to_string!(i16);
impl_to_string!(i32);
impl_to_string!(i64);
impl_to_string!(i128);
impl_to_string!(isize);
