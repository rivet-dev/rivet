use anyhow::Result;
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, EncodeError, Sink, Versioned};

mod generated {
	include!(concat!(env!("OUT_DIR"), "/combined_imports.rs"));
}

pub use generated::counter_v1::{Actual, Data as CounterRecord, Desired};

// These examples own their generated schema. Production Engine records belong in rivet-data.
pub const VERSION: u16 = 1;

pub struct CounterData(CounterRecord);

impl Versioned for CounterData {
	type Latest = CounterRecord;

	const LATEST: u16 = VERSION;

	fn wrap_latest(value: CounterRecord) -> Self {
		Self(value)
	}

	fn unwrap_latest(self) -> Result<CounterRecord> {
		Ok(self.0)
	}

	fn decode_version(version: u16, _body: Bytes) -> Result<Self, DecodeError> {
		Err(DecodeError::with_detail(
			DecodeErrorKind::UnsupportedVersion,
			0,
			format!("unsupported counter schema version {version}"),
		))
	}

	fn encode_version<S: Sink>(&self, version: u16, out: &mut S) -> Result<(), EncodeError> {
		if version != VERSION {
			return Err(EncodeError::UnsupportedVersion(version));
		}
		self.0.encode(out);
		Ok(())
	}
}
