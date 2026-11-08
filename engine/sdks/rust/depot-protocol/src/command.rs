//! Stable command identities and generated request and response payloads.
use rivet_data::families::{VersionFamily, VersionedFamily};
use rivet_keys::keys::tags;
use std::time::Duration;
use vbare::Versioned;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CommandTag(pub usize);

/// A command can run more than once. The client retries after a cancel, a timeout,
/// a maintainer move or an unknown outcome, so running a command again must leave
/// the same state and return an equivalent response. A conditional or counting write
/// tags its effect with an operation id and treats finding its own tag as success.
/// Each command documents how it meets this.
pub trait Command: Send + Sync + 'static {
	const TAG: CommandTag;
	/// The handler only reads a committed snapshot. It must not mutate, produce
	/// external effects, retain handles, or spawn work. Register it explicitly with
	/// `register_snapshot_read` for the existing writer-backed snapshot, or with
	/// `register_read` for read-only access. A refused read may be repeated at the request edge even
	/// if a previous routing attempt lost its reply, because it changed no state.
	const READ_ONLY: bool = false;
	/// Internal databases may be created lazily at the conventional KV block size.
	const CREATE_IF_MISSING: bool = false;
	/// When the command creates its volume, the host may first run it against an empty volume
	/// up to its first commit and send that commit's writes with the volume's creation, in one
	/// disk write. The commit waits for the creation: if the open created the volume with those
	/// writes, the handler continues and its result answers the command. Otherwise the commit
	/// fails and the command runs again on the opened volume, reading its real history.
	///
	/// The handler must be correct when rerun on that history: anything it does before its
	/// first commit may happen twice, and its effects must follow that commit. A volume that
	/// was not created by this open may hold none, all or a prefix of another run's writes, so
	/// the handler decides from the contents it reads, never from the presence of a key.
	const SPECULATIVE_CREATE: bool = false;
	const TIMEOUT: Duration = Duration::from_secs(5);
	/// The client and the host can run different binaries during a rolling deploy, so each
	/// body is encoded at its envelope family's negotiated version, never the compiled latest.
	type Request: VersionedFamily<Latest: Send>;
	type Response: VersionedFamily<Latest: Send>;
}

/// The version to encode a command body of envelope `V` at. A body with a family takes the
/// family's negotiated version from `write_version`, capped at the newest version `V`
/// encodes, since a family can be newer than one of its envelopes. A body without a family
/// uses its envelope's only version.
pub fn body_version<V: VersionedFamily>(
	write_version: impl FnOnce(VersionFamily) -> anyhow::Result<u16>,
) -> anyhow::Result<u16> {
	match V::FAMILY {
		Some(family) => Ok(write_version(family)?.min(V::LATEST)),
		None => Ok(V::LATEST),
	}
}

pub type Request<C> = <<C as Command>::Request as Versioned>::Latest;
pub type Response<C> = <<C as Command>::Response as Versioned>::Latest;

/// Read only. Repeating the command selects the latest committed snapshot again.
pub struct KvGet;

impl Command for KvGet {
	const TAG: CommandTag = CommandTag(tags::depot_command::KV_GET);
	type Request = rivet_data::versioned::DepotKvGetData;
	type Response = rivet_data::versioned::DepotKvGetResponseData;
}

/// Blind puts and deletes of fixed values. Repeating the command writes the same values.
pub struct KvApply;

impl Command for KvApply {
	const TAG: CommandTag = CommandTag(tags::depot_command::KV_APPLY);
	type Request = rivet_data::versioned::DepotKvApplyData;
	type Response = rivet_data::versioned::DepotKvApplyResponseData;
}
