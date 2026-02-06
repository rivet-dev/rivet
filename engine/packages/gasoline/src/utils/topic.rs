use std::fmt::Display;

pub trait AsTopic: Send + Sync {
	fn as_topic(&self) -> String;
}

impl AsTopic for String {
	fn as_topic(&self) -> String {
		self.clone()
	}
}

impl AsTopic for &str {
	fn as_topic(&self) -> String {
		self.to_string()
	}
}

impl<T: Display + Send + Sync, U: Display + Send + Sync> AsTopic for (T, U) {
	fn as_topic(&self) -> String {
		format!("{}:{}", self.0, self.1)
	}
}

impl AsTopic for () {
	fn as_topic(&self) -> String {
		String::new()
	}
}

impl<T: AsTopic> AsTopic for &T {
	fn as_topic(&self) -> String {
		(*self).as_topic()
	}
}
