use anyhow::*;
use serde::{Deserialize, Serialize};
use versioned_data_util::OwnedVersionedData;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestDataV1 {
	id: u32,
	name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestDataV2 {
	id: u32,
	name: String,
	description: String,
}

#[derive(Clone)]
enum TestData {
	V1(TestDataV1),
	V2(TestDataV2),
}

impl OwnedVersionedData for TestData {
	type Latest = TestDataV2;

	fn latest(latest: TestDataV2) -> Self {
		TestData::V2(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let TestData::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(TestData::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(TestData::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			TestData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			TestData::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestDataNoConvertersV1 {
	id: u32,
	name: String,
}

#[derive(Clone)]
enum TestDataNoConverters {
	V1(TestDataNoConvertersV1),
}

impl OwnedVersionedData for TestDataNoConverters {
	type Latest = TestDataNoConvertersV1;

	fn latest(latest: TestDataNoConvertersV1) -> Self {
		TestDataNoConverters::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let TestDataNoConverters::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(TestDataNoConverters::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			TestDataNoConverters::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl TestData {
	fn v1_to_v2(self) -> Result<Self> {
		match self {
			TestData::V1(v1) => Ok(TestData::V2(TestDataV2 {
				id: v1.id,
				name: v1.name,
				description: "default".to_string(),
			})),
			v2 => Ok(v2),
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		match self {
			TestData::V2(v2) => Ok(TestData::V1(TestDataV1 {
				id: v2.id,
				name: v2.name,
			})),
			v1 => Ok(v1),
		}
	}
}

#[test]
fn test_v2_to_v1_to_v2() {
	let data = TestDataV2 {
		id: 456,
		name: "test".to_string(),
		description: "will be stripped".to_string(),
	};

	let payload = TestData::V2(data.clone()).serialize(1).unwrap();

	let deserialized = TestData::deserialize(&payload, 1).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "test");
	assert_eq!(deserialized.description, "default");
}

#[test]
fn test_v2_to_v2() {
	let data = TestDataV2 {
		id: 456,
		name: "test".to_string(),
		description: "data".to_string(),
	};

	let payload = TestData::V2(data.clone()).serialize(2).unwrap();

	let deserialized = TestData::deserialize(&payload, 2).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "test");
	assert_eq!(deserialized.description, "data");
}

#[test]
fn test_unsupported_version() {
	assert!(TestData::deserialize(&[], 99).is_err());
}

#[test]
fn test_serialize() {
	let data = TestData::V2(TestDataV2 {
		id: 456,
		name: "serialize_test".to_string(),
		description: "will be stripped".to_string(),
	});

	// Test serializing to V1 (should convert V2 -> V1)
	let result = data.clone().serialize(1).unwrap();
	let deserialized: TestDataV1 = serde_bare::from_slice(&result).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "serialize_test");

	// Test serializing to V2
	let result = data.serialize(2).unwrap();
	let deserialized: TestDataV2 = serde_bare::from_slice(&result).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "serialize_test");
	assert_eq!(deserialized.description, "will be stripped");
}

#[test]
fn test_embedded_v2_to_v1_to_v2() {
	let data = TestDataV2 {
		id: 456,
		name: "test".to_string(),
		description: "will be stripped".to_string(),
	};

	let payload = TestData::V2(data.clone())
		.serialize_with_embedded_version(1)
		.unwrap();

	// First 2 bytes should be the version (2 in little-endian)
	assert_eq!(payload[0], 1u8);
	assert_eq!(payload[1], 0u8);

	let deserialized = TestData::deserialize_with_embedded_version(&payload).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "test");
	assert_eq!(deserialized.description, "default");
}

#[test]
fn test_embedded_v2_to_v2() {
	let data = TestDataV2 {
		id: 456,
		name: "test".to_string(),
		description: "data".to_string(),
	};

	let payload = TestData::V2(data.clone())
		.serialize_with_embedded_version(2)
		.unwrap();

	// First 2 bytes should be the version (2 in little-endian)
	assert_eq!(payload[0], 2u8);
	assert_eq!(payload[1], 0u8);

	let deserialized = TestData::deserialize_with_embedded_version(&payload).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "test");
	assert_eq!(deserialized.description, "data");
}

#[test]
fn test_no_converters() {
	let data = TestDataNoConvertersV1 {
		id: 456,
		name: "test".to_string(),
	};

	let payload = TestDataNoConverters::V1(data.clone()).serialize(1).unwrap();

	let deserialized = TestDataNoConverters::deserialize(&payload, 1).unwrap();
	assert_eq!(deserialized.id, 456);
	assert_eq!(deserialized.name, "test");
}
