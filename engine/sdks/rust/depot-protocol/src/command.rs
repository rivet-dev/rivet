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
	/// The command's answer may be asked for again by sending the same command anew after it
	/// committed: running it again changes nothing further, starts no second waiter or effect,
	/// and answers equivalently. A host that ran the command but cannot express its answer at
	/// the version it selected for it (`depot.response_version_unavailable`) is then returned to
	/// the edge as a retryable refusal. Without it, that answer is `depot.unavailable`, whose
	/// outcome is unknown, unless the command is `READ_ONLY`. Set it only on a command whose
	/// response envelope can fail to downgrade, after checking every outcome that can need
	/// the newer version.
	const IDEMPOTENT_ANSWER: bool = false;
	const TIMEOUT: Duration = Duration::from_secs(5);
	/// The client and the host can run different binaries during a rolling deploy, so each
	/// body is encoded at its envelope family's negotiated version, never the compiled latest.
	/// The caller seals the request at the highest version every live storage worker reads,
	/// since any of them may host the volume. The host answers at the response family's
	/// negotiated version, and no higher than the request's version when both envelopes share
	/// a family, which a caller of an older release reads.
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

/// Read only: a group volume's record and some of its fork child rows, from the latest applied
/// snapshot. A storage worker runs it on the first load of a group's volume and when a pin
/// needs children it has not cached.
pub struct GroupRead;

impl Command for GroupRead {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_READ);
	const READ_ONLY: bool = true;
	type Request = rivet_data::versioned::DepotGroupReadRequestData;
	type Response = rivet_data::versioned::DepotGroupReadResponseData;
}

/// Read only: a bounded page of an event shard, or its tail. Every storage worker runs it on
/// every shard once per poll interval.
pub struct EventsRead;

impl Command for EventsRead {
	const TAG: CommandTag = CommandTag(tags::depot_command::EVENTS_READ);
	const READ_ONLY: bool = true;
	type Request = rivet_data::versioned::DepotEventsReadRequestData;
	type Response = rivet_data::versioned::DepotEventsReadResponseData;
}

/// Creates a root group's record; creates the group volume.
pub struct GroupCreate;

impl Command for GroupCreate {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_CREATE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::DepotGroupCreateRequestData;
	type Response = rivet_data::versioned::DepotGroupCreateResponseData;
}

/// Draws a fork number on a parent group; idempotent by operation id.
pub struct GroupForkDraw;

impl Command for GroupForkDraw {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_FORK_DRAW);
	type Request = rivet_data::versioned::DepotGroupForkDrawRequestData;
	type Response = rivet_data::versioned::DepotGroupForkDrawResponseData;
}

/// Writes a child group's record; creates the group volume.
pub struct GroupChildCreate;

impl Command for GroupChildCreate {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_CHILD_CREATE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::DepotGroupChildCreateRequestData;
	type Response = rivet_data::versioned::DepotGroupChildCreateResponseData;
}

/// Archives a group whose children are archived.
pub struct GroupArchive;

impl Command for GroupArchive {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_ARCHIVE);
	type Request = rivet_data::versioned::DepotGroupArchiveRequestData;
	type Response = rivet_data::versioned::DepotGroupArchiveResponseData;
}

/// Marks a fork child row of a parent group archived.
pub struct GroupChildArchived;

impl Command for GroupChildArchived {
	const TAG: CommandTag = CommandTag(tags::depot_command::GROUP_CHILD_ARCHIVED);
	type Request = rivet_data::versioned::DepotGroupChildArchivedRequestData;
	type Response = rivet_data::versioned::DepotGroupChildArchivedResponseData;
}

/// Appends an event to a shard; creates the shard volume.
pub struct EventPost;

impl Command for EventPost {
	const TAG: CommandTag = CommandTag(tags::depot_command::EVENT_POST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::DepotEventPostRequestData;
	type Response = rivet_data::versioned::DepotEventPostResponseData;
}
