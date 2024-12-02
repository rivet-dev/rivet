use serde::Serialize;

#[derive(Serialize, Debug)]
pub enum TaskEvent {
	#[serde(rename = "log")]
	Log(String),
	#[serde(rename = "result")]
	Result {
		result: Box<serde_json::value::RawValue>,
	},
}
