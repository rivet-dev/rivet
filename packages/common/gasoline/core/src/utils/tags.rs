use std::fmt::Display;

use serde::Serialize;

use crate::error::{WorkflowError, WorkflowResult};

pub trait AsTags: Send + Sync {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value>;
	fn as_cjson_tags(&self) -> WorkflowResult<String>;
}

impl<T: Display + Send + Sync, U: Serialize + Send + Sync> AsTags for (T, U) {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		let (k, v) = self;
		Ok(serde_json::Value::Object(
			IntoIterator::into_iter([(
				k.to_string(),
				serde_json::to_value(v).map_err(WorkflowError::SerializeTags)?,
			)])
			.collect(),
		))
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		cjson::to_string(&self.as_tags()?).map_err(WorkflowError::CjsonSerializeTags)
	}
}

impl<T: Display + Send + Sync, U: Serialize + Send + Sync> AsTags for &[(T, U)] {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		let mut map = serde_json::Map::new();
		for (k, v) in self.iter() {
			map.insert(
				k.to_string(),
				serde_json::to_value(v).map_err(WorkflowError::SerializeTags)?,
			);
		}
		Ok(serde_json::Value::Object(map))
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		cjson::to_string(&self.as_tags()?).map_err(WorkflowError::CjsonSerializeTags)
	}
}

impl AsTags for serde_json::Value {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		match self {
			serde_json::Value::Object(_) => Ok(self.clone()),
			_ => Err(WorkflowError::InvalidTags("must be an object".to_string())),
		}
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		match self {
			serde_json::Value::Object(_) => {
				cjson::to_string(&self).map_err(WorkflowError::CjsonSerializeTags)
			}
			_ => Err(WorkflowError::InvalidTags("must be an object".to_string())),
		}
	}
}

impl<T: AsTags> AsTags for &T {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		(*self).as_tags()
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		(*self).as_cjson_tags()
	}
}
