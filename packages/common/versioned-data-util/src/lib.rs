use anyhow::*;

pub trait VersionedData<'a>: Sized {
	type Latest;

	fn latest(latest: Self::Latest) -> Self;
	fn into_latest(self) -> Result<Self::Latest>;
	fn deserialize_version(payload: &'a [u8], version: u16) -> Result<Self>;
	fn serialize_version(self, version: u16) -> Result<Vec<u8>>;

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// HACK: We do this for the type checker
		if false { vec![identity] } else { Vec::new() }
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// HACK: We do this for the type checker
		if false { vec![identity] } else { Vec::new() }
	}

	fn deserialize(payload: &'a [u8], version: u16) -> Result<Self::Latest> {
		let mut data = Self::deserialize_version(payload, version)?;

		for converter in Self::deserialize_converters()
			.iter()
			.skip(version.saturating_sub(1) as usize)
		{
			data = converter(data)?;
		}

		data.into_latest()
	}

	fn serialize(self, version: u16) -> Result<Vec<u8>> {
		let mut data = self;

		for converter in Self::serialize_converters()
			.iter()
			.skip(version.saturating_sub(1) as usize)
		{
			data = converter(data)?;
		}

		Self::serialize_version(data, version)
	}

	/// Serializes data with the version encoded as the first byte.
	fn deserialize_with_embedded_version(payload: &'a [u8]) -> Result<Self::Latest> {
		if payload.len() < 2 {
			bail!("payload too short for embedded version");
		}

		let version = u16::from_le_bytes([payload[0], payload[1]]);
		let payload = &payload[2..];

		Self::deserialize(payload, version)
	}

	/// Deserializes data with the version encoded as the first byte.
	fn serialize_with_embedded_version(self, version: u16) -> Result<Vec<u8>> {
		let payload = self.serialize(version)?;
		let mut result = Vec::with_capacity(2 + payload.len());
		result.extend_from_slice(&version.to_le_bytes());
		result.extend_from_slice(&payload);
		Ok(result)
	}
}

pub trait OwnedVersionedData: Sized {
	type Latest;

	fn latest(latest: Self::Latest) -> Self;
	fn into_latest(self) -> Result<Self::Latest>;
	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self>;
	fn serialize_version(self, version: u16) -> Result<Vec<u8>>;

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// HACK: We do this for the type checker
		if false { vec![identity] } else { Vec::new() }
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// HACK: We do this for the type checker
		if false { vec![identity] } else { Vec::new() }
	}

	fn deserialize(payload: &[u8], version: u16) -> Result<Self::Latest> {
		let mut data = Self::deserialize_version(payload, version)?;

		for converter in Self::deserialize_converters()
			.iter()
			.skip(version.saturating_sub(1) as usize)
		{
			data = converter(data)?;
		}

		data.into_latest()
	}

	fn serialize(self, version: u16) -> Result<Vec<u8>> {
		let mut data = self;

		for converter in Self::serialize_converters()
			.iter()
			.skip(version.saturating_sub(1) as usize)
		{
			data = converter(data)?;
		}

		Self::serialize_version(data, version)
	}

	// See VersionedData::deserialize_with_embedded_version.
	fn deserialize_with_embedded_version(payload: &[u8]) -> Result<Self::Latest> {
		if payload.len() < 2 {
			bail!("payload too short for embedded version");
		}

		let version = u16::from_le_bytes([payload[0], payload[1]]);
		let payload = &payload[2..];

		Self::deserialize(payload, version)
	}

	// See VersionedData::serialize_with_embedded_version.
	fn serialize_with_embedded_version(self, version: u16) -> Result<Vec<u8>> {
		let payload = self.serialize(version)?;
		let mut result = Vec::with_capacity(2 + payload.len());
		result.extend_from_slice(&version.to_le_bytes());
		result.extend_from_slice(&payload);
		Ok(result)
	}
}

/// Helper for default trait methods.
fn identity<T>(v: T) -> Result<T> {
	Ok(v)
}
