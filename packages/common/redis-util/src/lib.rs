use redis::{from_redis_value, FromRedisValue, RedisError, Value};

macro_rules! invalid_type_error {
	($v:expr, $det:expr) => {{
		RedisError::from((
			redis::ErrorKind::TypeError,
			"Response was of incompatible type",
			format!("{:?} (response was {:?})", $det, $v),
		))
	}};
}

#[derive(Debug)]
pub struct RedisResult<T>(Result<T, String>);

impl<T> std::ops::Deref for RedisResult<T> {
	type Target = Result<T, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: FromRedisValue> FromRedisValue for RedisResult<T> {
	fn from_redis_value(v: &Value) -> Result<RedisResult<T>, RedisError> {
		match *v {
			Value::Bulk(ref items) => {
				let mut items = items.into_iter();

				let status_raw = items
					.next()
					.ok_or_else(|| invalid_type_error!(items, "Missing status at first item."))?;
				let status = String::from_redis_value(status_raw)?;
				match status.as_str() {
					"ok" => {
						let data_raw = items.next().unwrap_or_else(|| &redis::Value::Nil);
						let data = FromRedisValue::from_redis_value(data_raw)?;
						Ok(RedisResult(Ok(data)))
					}
					"err" => {
						let err_raw = items.next().ok_or_else(|| {
							invalid_type_error!(items, "Missing error at second item.")
						})?;
						let err = String::from_redis_value(err_raw)?;
						Ok(RedisResult(Err(err)))
					}
					status @ _ => Err(invalid_type_error!(status, "Status was not `ok` or `err`.")),
				}
			}
			_ => Err(invalid_type_error!(v, "Not bulk data")),
		}
	}
}

#[derive(Debug)]
pub struct SearchResult {
	pub count: u64,
	pub entries: Vec<SearchResultEntry>,
}

impl FromRedisValue for SearchResult {
	fn from_redis_value(root: &Value) -> redis::RedisResult<SearchResult> {
		let root = root
			.as_sequence()
			.ok_or_else(|| invalid_type_error!(root, "Not a sequence."))?;
		let mut root_iter = root.iter();

		let count = root_iter
			.next()
			.ok_or_else(|| invalid_type_error!(root, "Missing count."))?;

		let mut entries = Vec::with_capacity((root.len() - 1) / 2);
		while let (Some(key), Some(data)) = (root_iter.next(), root_iter.next()) {
			let data = data
				.as_sequence()
				.ok_or_else(|| invalid_type_error!(data, "Data not a sequence."))?;
			let mut data_iter = data.iter();

			let mut data = Vec::with_capacity(data.len() / 2);
			while let (Some(property), Some(value)) = (data_iter.next(), data_iter.next()) {
				data.push(SearchResultEntryData {
					property: from_redis_value(property)?,
					value: from_redis_value(value)?,
				});
			}

			entries.push(SearchResultEntry {
				key: from_redis_value(key)?,
				data,
			});
		}

		Ok(SearchResult {
			count: from_redis_value(count)?,
			entries,
		})
	}
}

#[derive(Debug)]
pub struct SearchResultEntry {
	/// Key for the corresponding entry.
	pub key: String,

	/// Data returned by `RETURN` clause.
	///
	/// This will be a raw JSON string if querying JSON.
	pub data: Vec<SearchResultEntryData>,
}

#[derive(Debug)]
pub struct SearchResultEntryData {
	/// Property of the value returned for the entry.
	pub property: String,

	/// The data that was returned.
	pub value: String,
}

#[derive(Debug)]
pub struct SearchResultNoContent {
	pub count: u64,
	pub keys: Vec<String>,
}

impl FromRedisValue for SearchResultNoContent {
	fn from_redis_value(root: &Value) -> redis::RedisResult<SearchResultNoContent> {
		let root = root
			.as_sequence()
			.ok_or_else(|| invalid_type_error!(root, "Not a sequence."))?;
		let mut root_iter = root.iter();

		let count = root_iter
			.next()
			.ok_or_else(|| invalid_type_error!(root, "Missing count."))?;

		let keys = root_iter
			.map(|entry| from_redis_value::<String>(entry))
			.collect::<Result<Vec<_>, _>>()?;

		Ok(SearchResultNoContent {
			count: from_redis_value(count)?,
			keys,
		})
	}
}

lazy_static::lazy_static! {
	static ref ESCAPE_SEARCH_QUERY: regex::Regex = regex::Regex::new(r"[^\w_]").unwrap();
	static ref ESCAPE_SEARCH_QUERY_WITH_SPACES: regex::Regex = regex::Regex::new(r"[^\w_ ]").unwrap();
}

/// See https://redis.io/docs/stack/search/reference/escaping/#the-rules-of-text-field-tokenization
pub fn escape_search_query(query: impl ToString) -> String {
	ESCAPE_SEARCH_QUERY
		.replace_all(&query.to_string(), "\\$0")
		.to_string()
}

// Doesn't escape spaces, they are replaced with wildcards
pub fn double_escape_search_query_with_spaces(query: impl ToString) -> String {
	ESCAPE_SEARCH_QUERY_WITH_SPACES
		.replace_all(&query.to_string(), "\\\\$0")
		.to_string()
}
