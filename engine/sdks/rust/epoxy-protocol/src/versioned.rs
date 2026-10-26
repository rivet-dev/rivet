use anyhow::{Result, ensure};
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, Message, Versioned};

use crate::{PROTOCOL_VERSION, generated::v5};

/// Implements [`Versioned`] for a single-version envelope `enum $name { V5($latest) }`.
macro_rules! impl_versioned_single {
	($name:ident, $latest:ty) => {
		impl Versioned for $name {
			type Latest = $latest;

			const LATEST: u16 = PROTOCOL_VERSION;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V5(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				let Self::V5(latest) = self;
				Ok(latest)
			}

			fn decode_version(
				version: u16,
				_body: Bytes,
			) -> std::result::Result<Self, DecodeError> {
				Err(unsupported_version(version))
			}

			fn encode_version<S: vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), vbare::EncodeError> {
				if version != Self::LATEST {
					return Err(vbare::EncodeError::UnsupportedVersion(version));
				}
				let Self::V5(latest) = self;
				latest.encode(out);
				Ok(())
			}
		}
	};
}

/// The error for a version that a single-version envelope cannot decode.
fn unsupported_version(version: u16) -> DecodeError {
	DecodeError::with_detail(
		DecodeErrorKind::UnsupportedVersion,
		0,
		format!("invalid Epoxy version: {version}"),
	)
}

pub enum CommittedValue {
	V5(v5::CommittedValue),
}

impl_versioned_single!(CommittedValue, v5::CommittedValue);

pub enum AcceptedValue {
	V5(v5::AcceptedValue),
}

impl_versioned_single!(AcceptedValue, v5::AcceptedValue);

pub enum Request {
	V5(v5::Request),
}

impl_versioned_single!(Request, v5::Request);

pub fn decode_request(bytes: &[u8], version: u16) -> Result<v5::Request> {
	Request::deserialize(bytes, version)
}

pub fn encode_request(value: v5::Request, version: u16) -> Result<Vec<u8>> {
	Request::serialize(&value, version)
}

pub fn decode_response(bytes: &[u8], version: u16) -> Result<v5::Response> {
	ensure!(
		version == PROTOCOL_VERSION,
		"unsupported Epoxy version: {version}"
	);
	Ok(v5::Response::decode(Bytes::copy_from_slice(bytes))?)
}

pub fn encode_response(value: v5::Response, version: u16) -> Result<Vec<u8>> {
	ensure!(
		version == PROTOCOL_VERSION,
		"unsupported Epoxy version: {version}"
	);
	Ok(value.encode_to_vec())
}

pub enum ControlCommand {
	V5(v5::ControlCommand),
}

impl_versioned_single!(ControlCommand, v5::ControlCommand);

pub enum ControlResponse {
	V5(v5::ControlResponse),
}

impl_versioned_single!(ControlResponse, v5::ControlResponse);

pub enum ReplicaCommand {
	V5(v5::ReplicaCommand),
}

impl_versioned_single!(ReplicaCommand, v5::ReplicaCommand);

pub enum ReplicaCommandResponse {
	V5(v5::ReplicaCommandResponse),
}

impl_versioned_single!(ReplicaCommandResponse, v5::ReplicaCommandResponse);

pub enum Ballot {
	V5(v5::Ballot),
}

impl_versioned_single!(Ballot, v5::Ballot);
