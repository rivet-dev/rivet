use std::fmt;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
pub enum RuntimeProtocolKind {
	Envoy = 0,
	Ups = 1,
	Epoxy = 3,
}

impl fmt::Display for RuntimeProtocolKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			RuntimeProtocolKind::Envoy => f.write_str("envoy"),
			RuntimeProtocolKind::Ups => f.write_str("ups"),
			RuntimeProtocolKind::Epoxy => f.write_str("epoxy"),
		}
	}
}

#[derive(PartialEq, Eq)]
pub struct RuntimeProtocol {
	pub kind: RuntimeProtocolKind,
	compiled_version: u16,
	override_version: Option<u16>,
}

impl RuntimeProtocol {
	pub fn new(kind: RuntimeProtocolKind, compiled_version: u16) -> Self {
		RuntimeProtocol {
			kind,
			compiled_version,
			override_version: None,
		}
	}

	pub fn version(&self) -> u16 {
		if let Some(override_version) = self.override_version {
			return override_version;
		}

		// Default fallback
		self.compiled_version()
	}

	pub fn set_override_version(&mut self, override_version: u16) {
		self.override_version = Some(override_version);
	}

	pub fn compiled_version(&self) -> u16 {
		self.compiled_version
	}
}

#[derive(PartialEq, Eq)]
pub struct RuntimeProtocols {
	pub envoy: RuntimeProtocol,
	pub ups: RuntimeProtocol,
	pub epoxy: RuntimeProtocol,
}

impl Default for RuntimeProtocols {
	/// Placeholder versions for a process that does not carry its compiled protocol versions.
	///
	/// Version 0 is not a real schema version, so a call site that reaches the wire with a
	/// placeholder fails instead of quietly negotiating the wrong format. Only the binary that
	/// depends on the protocol crates can supply real versions.
	fn default() -> Self {
		RuntimeProtocols {
			envoy: RuntimeProtocol::new(RuntimeProtocolKind::Envoy, 0),
			ups: RuntimeProtocol::new(RuntimeProtocolKind::Ups, 0),
			epoxy: RuntimeProtocol::new(RuntimeProtocolKind::Epoxy, 0),
		}
	}
}

impl RuntimeProtocols {
	/// Returns every protocol in a fixed order.
	pub fn iter(&self) -> std::array::IntoIter<&RuntimeProtocol, 3> {
		[&self.envoy, &self.ups, &self.epoxy].into_iter()
	}

	/// Returns every protocol in a fixed order, allowing each to be modified in place.
	pub fn iter_mut(&mut self) -> std::array::IntoIter<&mut RuntimeProtocol, 3> {
		[
			&mut self.envoy,
			&mut self.ups,
			&mut self.epoxy,
		]
		.into_iter()
	}
}

impl<'a> IntoIterator for &'a RuntimeProtocols {
	type Item = &'a RuntimeProtocol;

	type IntoIter = std::array::IntoIter<&'a RuntimeProtocol, 3>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a> IntoIterator for &'a mut RuntimeProtocols {
	type Item = &'a mut RuntimeProtocol;

	type IntoIter = std::array::IntoIter<&'a mut RuntimeProtocol, 3>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}
