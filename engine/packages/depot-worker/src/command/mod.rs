//! Typed handlers execute beside the maintainer's filesystem, outside its lifecycle loop.
pub mod groups;
mod kv;

/// The name of the KV file every command volume keeps under its root; the point reads of
/// writerless commands and the writer path address the same file.
pub const KV_FILE: &[u8] = b"kv";
mod session;
mod speculation;
pub mod volume_host;

use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_client::filesystem::{Error, Filesystem};
pub use depot_client::filesystem::{Error as CommitError, Transaction};
use depot_protocol::command::{Command, CommandTag, Request, Response, body_version};
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use rivet_data::families::VersionedFamily;
use std::sync::Weak;
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};
use tokio_util::sync::{CancellationToken, DropGuard};
use vbare::Versioned;

use crate::tablet_maintainer_machine::state::MaintainerPhase;

/// A command-scoped database handle. The maintainer owns opening and closing the filesystem.
#[derive(Clone)]
pub struct KvHandle {
	alarm_owner: Option<rivet_data::generated::shared_depot_alarms_v1::AlarmOwner>,
	filesystem: Arc<Filesystem>,
	ino: u64,
	may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
	operation_id: [u8; 16],
	volume_key: depot_types::VolumeKey,
	locked_ranges: Arc<Mutex<BTreeMap<Bytes, LockEntry>>>,
	shared: Arc<OpeningState>,
	commits: Arc<CommandCommits>,
	/// Set only on a speculative first command's handle, whose own filesystem holds only the
	/// in-memory capture of the creating write. Its later transactions run where this says.
	speculative: Option<Arc<speculation::SpeculativeWriter>>,
	/// Selects the version of each versioned value a command stores; the worker's topology.
	versions: Arc<dyn rivet_topology::versions::WriteVersions>,
	/// The volume was made from its parent group's copy at a fork, as the record the open
	/// answered says; `None` for a volume created in place or opened speculatively.
	lazy_fork: Option<rivet_data::generated::shared_depot_fs_v2::FsLazyForkOrigin>,
	/// Makes a tablet version mark durable before a value above the read floor is stored;
	/// the hosting maintainer.
	marks: Arc<dyn VersionMarks>,
}

/// Raises the version marks of the tablet a command's volume lives in. A command stores a
/// value of a family above the version `KvHandle::write_version` selected only when the
/// selected version cannot express it, and only after the tablet's durable record carries
/// a mark of that family at that version.
///
/// Two layers keep a worker that cannot read the mark from hosting the tablet. The record is
/// durable before the command commits, and a worker that opens the record refuses a mark it
/// cannot read (reports the refusal and fences), so it never decodes the value. The raised
/// mark also reaches the coordinator, which then never grants or places the tablet on such a
/// worker; that report is sent once the pass carrying the mark is dispatched, so it usually
/// lands before the commit, but nothing orders the two. A grant to an older worker in that
/// window costs one refused open and a re-grant, never a misread.
pub trait VersionMarks: Send + Sync + 'static {
	/// Resolves once the tablet's durable record carries `mark` or a higher version of its
	/// family. Fails at once for a mark this binary does not write, and with a typed
	/// retryable refusal when the hosting maintainer is still opening (`TabletOpening`),
	/// cannot take the request now (`CommitError::Overloaded`) or is not serving or stops
	/// serving first (`CommitError::Fenced`, which the caller follows to the tablet's next
	/// host); nothing the command planned is stored.
	fn ensure(
		&self,
		mark: rivet_data::generated::shared_depot_types_v1::VersionMark,
	) -> BoxFuture<'_, Result<()>>;
}

/// A command needed a version mark while its tablet's maintainer was still opening. Nothing
/// was requested or stored; the command host answers it as an overloaded volume transition,
/// so the edge sends the command again after the hint instead of re-resolving the route.
#[derive(Debug, thiserror::Error)]
#[error("the tablet is still opening, so its version marks cannot be raised yet")]
pub(crate) struct TabletOpening;

/// Sends `MaintainerEvent::VersionMarkWanted` to the hosting maintainer's inbox. Refuses with
/// `CommitError::Overloaded` when the inbox is full and `CommitError::Fenced` when the
/// maintainer is gone. It is called while `MaintainerVersionMarks::requested` is locked, so
/// it must not block or await (`try_send` does neither).
pub(crate) type RequestVersionMark = Arc<
	dyn Fn(rivet_data::generated::shared_depot_types_v1::VersionMark) -> Result<(), CommitError>
		+ Send
		+ Sync,
>;

/// The hosting maintainer's marks: its published durable marks, and its inbox for a mark it
/// does not carry yet.
pub(crate) struct MaintainerVersionMarks {
	pub(crate) snapshot:
		tokio::sync::watch::Receiver<crate::tablet_maintainer_machine::MaintainerSnapshot>,
	pub(crate) request: RequestVersionMark,
	/// Marks one of this host's commands sent and is still waiting on, so concurrent commands
	/// that need the same or a lower version wait on that request instead of each taking an
	/// inbox slot. A command removes its entry when its wait ends, however it ends, so a later
	/// command asks again if the mark is still not durable. Held only for short pushes and
	/// removals, never across an await, and removed from `Drop`.
	pub(crate) requested:
		parking_lot::Mutex<Vec<rivet_data::generated::shared_depot_types_v1::VersionMark>>,
}

impl MaintainerVersionMarks {
	pub(crate) fn new(
		snapshot: tokio::sync::watch::Receiver<
			crate::tablet_maintainer_machine::MaintainerSnapshot,
		>,
		request: RequestVersionMark,
	) -> Self {
		Self {
			snapshot,
			request,
			requested: parking_lot::Mutex::new(Vec::new()),
		}
	}

	/// Sends a request for `mark` and records it, unless another command already waits on a
	/// delivered request covering it (`Ok(None)`). The send happens under the lock and the
	/// entry is recorded only when it succeeds, so a command never waits on a request the
	/// maintainer did not get: a racing command either sees the delivered request or sends
	/// its own and is refused at admission like the first. The returned guard removes the
	/// entry when dropped.
	fn request_or_share(
		&self,
		mark: &rivet_data::generated::shared_depot_types_v1::VersionMark,
	) -> Result<Option<RequestedMark<'_>>, CommitError> {
		let mut requested = self.requested.lock();
		if marks_cover(&requested, mark) {
			return Ok(None);
		}
		(self.request)(mark.clone())?;
		requested.push(mark.clone());
		Ok(Some(RequestedMark {
			requested: &self.requested,
			mark: mark.clone(),
		}))
	}
}

/// One command's outstanding `VersionMarkWanted` in `MaintainerVersionMarks::requested`.
struct RequestedMark<'a> {
	requested:
		&'a parking_lot::Mutex<Vec<rivet_data::generated::shared_depot_types_v1::VersionMark>>,
	mark: rivet_data::generated::shared_depot_types_v1::VersionMark,
}

impl Drop for RequestedMark<'_> {
	fn drop(&mut self) {
		let mut requested = self.requested.lock();
		if let Some(index) = requested.iter().position(|mark| mark == &self.mark) {
			requested.swap_remove(index);
		}
	}
}

/// Whether this binary writes `mark`: its family is known and its version lies between the
/// family's `min_read` and `max_write`.
fn writes_mark(mark: &rivet_data::generated::shared_depot_types_v1::VersionMark) -> bool {
	rivet_data::families::VersionFamily(mark.family)
		.info()
		.is_some_and(|info| info.min_read <= mark.version && mark.version <= info.max_write)
}

/// Whether `marks` carry `mark` or a higher version of its family.
fn marks_cover(
	marks: &[rivet_data::generated::shared_depot_types_v1::VersionMark],
	mark: &rivet_data::generated::shared_depot_types_v1::VersionMark,
) -> bool {
	rivet_data::families::mark_of(marks, rivet_data::families::VersionFamily(mark.family))
		.is_some_and(|version| version >= mark.version)
}

impl VersionMarks for MaintainerVersionMarks {
	fn ensure(
		&self,
		mark: rivet_data::generated::shared_depot_types_v1::VersionMark,
	) -> BoxFuture<'_, Result<()>> {
		Box::pin(async move {
			// The command host and the maintainer are the same binary, so a mark it does not
			// write is a bug in the command; the maintainer would ignore it and the command
			// would wait out its deadline.
			ensure!(
				writes_mark(&mark),
				"a command asked for version {} of family {}, which this binary does not write",
				mark.version,
				mark.family
			);
			let phase = {
				let snapshot = self.snapshot.borrow();
				if marks_cover(&snapshot.version_marks_durable, &mark) {
					return Ok(());
				}
				snapshot.phase.clone()
			};

			// Refuse before asking when no pass can raise the mark. An opening maintainer will
			// serve shortly on this host, so the command is shed as a transition and sent again
			// here; any other phase means the tablet's authority is moving or ending, so the
			// refusal is `Fenced` and the caller follows the route to the next host.
			match phase {
				MaintainerPhase::Serving => {}
				MaintainerPhase::Opening => return Err(TabletOpening.into()),
				MaintainerPhase::Frozen
				| MaintainerPhase::Draining
				| MaintainerPhase::Stopped
				| MaintainerPhase::Fenced => return Err(CommitError::Fenced.into()),
			}

			// Ask once per host, then wait for a pass to make the raised record durable. The
			// wait ends with the grant's serving phase, refused as `Fenced` like the phases
			// above, or with the command when its deadline drops it.
			let _requested = self.request_or_share(&mark)?;
			let mut snapshot = self.snapshot.clone();
			let reached = snapshot
				.wait_for(|snapshot| {
					marks_cover(&snapshot.version_marks_durable, &mark)
						|| snapshot.phase != MaintainerPhase::Serving
				})
				.await
				.map_err(|_| CommitError::Fenced)?;
			if !marks_cover(&reached.version_marks_durable, &mark) {
				return Err(CommitError::Fenced.into());
			}
			Ok(())
		})
	}
}

/// What a command may learn of the record of the volume it runs on: the incarnation it
/// holds and, for a volume made by a group fork, where it was forked from. A state machine
/// that inherits state through a fork resets it once per incarnation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeRecordView {
	pub incarnation: [u8; 16],
	pub lazy_fork: Option<rivet_data::generated::shared_depot_fs_v2::FsLazyForkOrigin>,
}

impl VolumeRecordView {
	/// The incarnation a state last reset for `stamp` must be reset for now: this volume's,
	/// when it was made by a group fork and the stamp is not already it. A volume created in
	/// place inherits nothing and never needs a reset.
	pub fn pending_fork_reset(&self, stamp: Option<[u8; 16]>) -> Option<[u8; 16]> {
		if self.lazy_fork.is_none() {
			return None;
		}
		if stamp == Some(self.incarnation) {
			return None;
		}
		Some(self.incarnation)
	}
}

/// The commits one command has in flight, and whether its caller's deadline has passed. A
/// commit dispatched before the deadline must run to its own outcome: dropping it abandons the
/// write and fences the filesystem. Work the command has not dispatched by then is refused, so
/// nothing it starts lands after the caller was told the command timed out.
#[derive(Debug)]
pub(crate) struct CommandCommits {
	deadline_boot_ms: u64,
	in_flight: tokio::sync::watch::Sender<usize>,
	expired: std::sync::atomic::AtomicBool,
	/// Set once the command dispatches its first commit and never cleared.
	dispatched: std::sync::atomic::AtomicBool,
}

impl Default for CommandCommits {
	fn default() -> Self {
		Self::new(u64::MAX)
	}
}

impl CommandCommits {
	fn new(deadline_boot_ms: u64) -> Self {
		Self {
			deadline_boot_ms,
			in_flight: tokio::sync::watch::Sender::new(0),
			expired: std::sync::atomic::AtomicBool::new(false),
			dispatched: std::sync::atomic::AtomicBool::new(false),
		}
	}
}

impl CommandCommits {
	/// Refuses every later `begin`, `snapshot` and `commit` of the command.
	pub(crate) fn expire(&self) {
		self.expired
			.store(true, std::sync::atomic::Ordering::Release);
	}

	fn expired(&self) -> bool {
		self.expired.load(std::sync::atomic::Ordering::Acquire)
	}

	/// Whether the command dispatched a commit, whatever its outcome.
	pub(crate) fn dispatched(&self) -> bool {
		self.dispatched.load(std::sync::atomic::Ordering::Acquire)
	}

	/// Resolves once no commit of the command is in flight.
	pub(crate) async fn settled(&self) {
		let mut in_flight = self.in_flight.subscribe();
		let _ = in_flight.wait_for(|count| *count == 0).await;
	}
}

/// Counts one in-flight commit until dropped.
struct CommitInFlight<'a>(&'a CommandCommits);

impl<'a> CommitInFlight<'a> {
	fn new(commits: &'a CommandCommits) -> Self {
		commits
			.dispatched
			.store(true, std::sync::atomic::Ordering::Release);
		commits.in_flight.send_modify(|count| *count += 1);
		Self(commits)
	}
}

impl Drop for CommitInFlight<'_> {
	fn drop(&mut self) {
		self.0.in_flight.send_modify(|count| *count -= 1);
	}
}

/// State one volume opening shares across its commands, one value per type. Components that
/// coordinate the commands of a volume, such as the durable runner's event groups, keep it here
/// so it lives exactly as long as the opening.
pub type OpeningState = scc::HashMap<std::any::TypeId, Arc<dyn std::any::Any + Send + Sync>>;

/// A weak identity for one database opening; retaining it never retains the filesystem.
#[derive(Clone)]
pub struct KvOwner {
	filesystem: std::sync::Weak<Filesystem>,
	may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
}
impl KvOwner {
	pub fn is_current(&self, handle: &KvHandle) -> bool {
		(self.may_serve)()
			&& (handle.may_serve)()
			&& self.filesystem.ptr_eq(&Arc::downgrade(&handle.writer().0))
	}
	pub fn same_opening(&self, other: &Self) -> bool {
		self.filesystem.ptr_eq(&other.filesystem)
	}

	/// Whether the opening still exists and may serve, for work the opening started that holds
	/// no handle of it, such as a background task that must stop once the volume moved away.
	pub fn may_serve(&self) -> bool {
		(self.may_serve)() && self.filesystem.strong_count() > 0
	}
}

impl rivet_topology::versions::WriteVersions for KvHandle {
	fn write_version(&self, family: rivet_data::families::VersionFamily) -> Result<u16> {
		self.versions.write_version(family)
	}

	fn floor_listed(&self) -> bool {
		self.versions.floor_listed()
	}

	fn wait_for_read_floor(&self, deadline_boot_ms: u64) -> BoxFuture<'_, Result<bool>> {
		self.versions.wait_for_read_floor(deadline_boot_ms)
	}
}

impl KvHandle {
	/// This command's admission deadline in the worker's monotonic clock domain.
	/// Nested reads may use an earlier deadline to leave time for their owning command to commit.
	pub fn deadline_boot_ms(&self) -> u64 {
		self.commits.deadline_boot_ms
	}

	/// The version to encode a value of `family` stored in this volume at: the highest every
	/// live storage worker reads. A command takes every versioned value's version from here.
	pub fn write_version(&self, family: rivet_data::families::VersionFamily) -> Result<u16> {
		self.versions.write_version(family)
	}

	/// Resolves once the tablet this volume lives in durably carries a mark of `family` at
	/// `version`. A command calls it before committing a value of `family` encoded above the
	/// version `write_version` selected; see [`VersionMarks`].
	pub async fn ensure_version_mark(
		&self,
		family: rivet_data::families::VersionFamily,
		version: u16,
	) -> Result<()> {
		self.marks
			.ensure(rivet_data::generated::shared_depot_types_v1::VersionMark {
				family: family.0,
				version,
			})
			.await
	}

	pub fn has_alarm_owner(
		&self,
		owner: &rivet_data::generated::shared_depot_alarms_v1::AlarmOwner,
	) -> bool {
		(self.may_serve)() && self.alarm_owner.as_ref() == Some(owner)
	}

	/// The record of the volume this handle runs on, as the open answered it.
	pub fn volume_record(&self) -> VolumeRecordView {
		VolumeRecordView {
			incarnation: self.volume_log_key().0.volume_incarnation,
			lazy_fork: self.lazy_fork.clone(),
		}
	}

	/// Whether this command's caller deadline has passed. Once it has, `begin`, `snapshot` and
	/// `commit` refuse, so a handler that coordinates other commands' work, such as an event
	/// group holder, must hand that work to a live command instead of taking it.
	pub fn expired(&self) -> bool {
		self.commits.expired()
	}

	/// Reserves optional derived data against the filesystem process cache budget.
	pub fn reserve_cache(
		&self,
		bytes: usize,
	) -> Option<depot_client::filesystem::CacheReservation> {
		self.writer().0.reserve_cache(bytes)
	}

	/// This opening's shared value of type `T`, created on first use. Commands of the same
	/// opening, and clones of their handles, get the same value.
	pub async fn opening_state<T: Default + Send + Sync + 'static>(&self) -> Result<Arc<T>> {
		let value = self
			.shared
			.entry_async(std::any::TypeId::of::<T>())
			.await
			.or_insert_with(|| Arc::new(T::default()))
			.get()
			.clone();
		value
			.downcast::<T>()
			.map_err(|_| anyhow::anyhow!("opening state has the wrong type"))
	}

	/// The filesystem and inode this handle names: the opened volume once a speculative first
	/// command's creating commit moved the handle there.
	fn writer(&self) -> (Arc<Filesystem>, u64) {
		match self
			.speculative
			.as_ref()
			.and_then(|speculative| speculative.moved())
		{
			Some(moved) => moved,
			None => (self.filesystem.clone(), self.ino),
		}
	}

	/// The filesystem and inode the next transaction runs on. A speculative handle runs only its
	/// first transaction on the capture of the creating write; any later one, whether the
	/// command's or an effect's, waits for the open's answer and runs on the opened volume, or
	/// fails as fenced when the speculation did not create it.
	async fn transaction_writer(&self) -> Result<(Arc<Filesystem>, u64)> {
		let Some(speculative) = &self.speculative else {
			return Ok((self.filesystem.clone(), self.ino));
		};
		if speculative.claim_first_transaction() {
			return Ok((self.filesystem.clone(), self.ino));
		}
		match speculative.outcome().await {
			Some(moved) => Ok(moved),
			None => Err(Error::Fenced.into()),
		}
	}

	pub fn owner(&self) -> KvOwner {
		KvOwner {
			filesystem: Arc::downgrade(&self.writer().0),
			may_serve: self.may_serve.clone(),
		}
	}

	/// Stable across transport retries; application commands can tag their effects with it.
	pub fn operation_id(&self) -> [u8; 16] {
		self.operation_id
	}

	/// The database selected by routing; handlers validate domain addresses against it.
	pub fn volume_key(&self) -> &depot_types::VolumeKey {
		&self.volume_key
	}

	/// The exact incarnation this handle's filesystem owns, so a delayed delete cannot remove a
	/// replacement.
	pub fn volume_log_key(&self) -> depot_types::VolumeLogKey {
		self.writer().0.volume_log_key().clone()
	}

	/// Refuses once the command's deadline passed. A snapshot never commits, so on a speculative
	/// handle it does not claim the capture as the handle's first transaction: before the open
	/// answers it reads the capture, which is the empty volume the command sees, and after it
	/// reads the opened volume. A handler may therefore read through a snapshot and still commit
	/// its creating write through `begin` afterwards.
	pub async fn snapshot(&self) -> Result<Transaction> {
		ensure!((self.may_serve)(), "command authority ended");
		ensure!(!self.commits.expired(), "command deadline passed");
		let (filesystem, ino) = self.writer();
		Ok(filesystem.begin(ino).await?)
	}

	/// Refuses with "command deadline passed" once the command's deadline passed, so a handler
	/// that begins again after a commit can fail there.
	pub async fn begin(&self) -> Result<Transaction> {
		ensure!((self.may_serve)(), "command authority ended");
		ensure!(!self.commits.expired(), "command deadline passed");
		let (filesystem, ino) = self.transaction_writer().await?;
		ensure!(!self.commits.expired(), "command deadline passed");
		Ok(filesystem.begin(ino).await?)
	}

	/// Revocation after admission cannot prove that a submitted commit did not land. Refuses
	/// with `DeadlineExceeded`, before dispatch, once the command's deadline passed; a commit
	/// dispatched before then runs to its own outcome.
	pub async fn commit(&self, transaction: Transaction) -> Result<u64, Error> {
		if !(self.may_serve)() {
			return Err(Error::Fenced);
		}
		if self.commits.expired() {
			return Err(Error::DeadlineExceeded);
		}
		let in_flight = CommitInFlight::new(&self.commits);
		let result = transaction.commit().await;
		drop(in_flight);
		if !(self.may_serve)() {
			return Err(Error::Fenced);
		}
		result
	}
	/// End exclusive. Not reentrant. Not fair.
	pub async fn lock_partition(
		&self,
		range: std::ops::Range<impl Into<Bytes>>,
	) -> Result<RangeLockGuard> {
		let start = range.start.into();
		let end = range.end.into();

		ensure!(start < end);

		loop {
			ensure!((self.may_serve)(), "command authority ended");
			let token = {
				let mut guard = self.locked_ranges.lock();

				let collision = guard
					.range::<Bytes, _>(..&end)
					.next_back()
					.filter(|(_, existing_lock)| start < existing_lock.end);

				match collision {
					Some((_, existing_lock)) => existing_lock.guard.token().clone(),
					None => {
						// Add new lock
						guard.insert(
							start.clone(),
							LockEntry {
								end,
								guard: CancellationToken::new().drop_guard(),
							},
						);

						return Ok(RangeLockGuard {
							start,
							locked_ranges: Arc::downgrade(&self.locked_ranges),
						});
					}
				}
			};

			// Lock collides, wait for it to be released
			token.cancelled().await;
		}
	}
}

/// A read-only view of a command volume's `kv` inode, given to read commands.
///
/// When the volume is open on the host, reads go through its writer. Otherwise each read is one
/// maintainer point read: the first reads the latest applied snapshot, which can lag the
/// committed log by the apply delay, and later reads of the command reuse that snapshot. A read
/// command must tolerate that staleness.
#[derive(Clone)]
pub struct ReadHandle {
	source: ReadSource,
	volume_key: depot_types::VolumeKey,
	/// Selects the version the command's response is encoded at; the worker's topology.
	versions: Arc<dyn rivet_topology::versions::WriteVersions>,
}

#[derive(Clone)]
enum ReadSource {
	Writer(KvHandle),
	Writerless(Arc<session::WriterlessRead>),
}

impl ReadHandle {
	/// Wraps a writer-backed command context, for tests and the host's writer path.
	pub fn from_writer(kv: KvHandle) -> Self {
		let volume_key = kv.volume_key.clone();
		let versions = kv.versions.clone();
		Self {
			source: ReadSource::Writer(kv),
			volume_key,
			versions,
		}
	}

	/// The database selected by routing; handlers validate domain addresses against it.
	pub fn volume_key(&self) -> &depot_types::VolumeKey {
		&self.volume_key
	}

	/// Whether this read is served by maintainer point reads instead of the volume writer.
	pub fn is_writerless(&self) -> bool {
		match &self.source {
			ReadSource::Writer(_) => false,
			ReadSource::Writerless(_) => true,
		}
	}

	pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		match &self.source {
			ReadSource::Writer(kv) => Ok(kv.snapshot().await?.get(key).await?),
			ReadSource::Writerless(read) => read.get(key).await,
		}
	}

	/// Reads `keys` from one snapshot, in one round trip when the read is writerless. The
	/// values come back in the order of `keys`, `None` for a key with no row.
	pub async fn get_many(&self, keys: &[Vec<u8>]) -> Result<Vec<Option<Vec<u8>>>> {
		match &self.source {
			ReadSource::Writer(kv) => {
				let mut snapshot = kv.snapshot().await?;
				let mut values = Vec::with_capacity(keys.len());
				for key in keys {
					values.push(snapshot.get(key).await?);
				}
				Ok(values)
			}
			ReadSource::Writerless(read) => read.get_many(keys).await,
		}
	}

	/// [`Self::get_many`], with how many volume seqs the snapshot lagged the highest commit the
	/// volume's maintainer knew of when it answered. A writer's snapshot holds every commit of
	/// the volume, so it lags none.
	pub async fn get_many_with_lag(&self, keys: &[Vec<u8>]) -> Result<(Vec<Option<Vec<u8>>>, u64)> {
		match &self.source {
			ReadSource::Writer(_) => Ok((self.get_many(keys).await?, 0)),
			ReadSource::Writerless(read) => read.get_many_with_lag(keys).await,
		}
	}
}

/// A read command. It may run on a maintainer read session with no volume writer, so it must
/// tolerate a snapshot that lags the committed log, and it must not write.
pub trait ReadHandler<C: Command>: Send + Sync + 'static {
	fn execute(
		&self,
		read: ReadHandle,
		request: Request<C>,
	) -> impl Future<Output = Result<Response<C>>> + Send;
}

impl<C, F, Fut> ReadHandler<C> for F
where
	C: Command,
	F: Fn(ReadHandle, Request<C>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Response<C>>> + Send,
{
	fn execute(
		&self,
		read: ReadHandle,
		request: Request<C>,
	) -> impl Future<Output = Result<Response<C>>> + Send {
		self(read, request)
	}
}

pub struct RangeLockGuard {
	start: Bytes,
	locked_ranges: Weak<Mutex<BTreeMap<Bytes, LockEntry>>>,
}

impl Drop for RangeLockGuard {
	fn drop(&mut self) {
		if let Some(locked_ranges) = self.locked_ranges.upgrade() {
			let mut locked_ranges = locked_ranges.lock();

			locked_ranges.remove(&self.start);
		}
	}
}

/// A command can run more than once. The client retries after a cancel, a timeout,
/// a maintainer move or an unknown outcome, so running a command again must leave
/// the same state and return an equivalent response. A conditional or counting write
/// tags its effect with an operation id and treats finding its own tag as success.
/// Each command documents how it meets this.
///
/// A handler may start several transactions. It must not retain its handle or spawn
/// storage work that outlives the command. The maintainer cancels it on revocation.
pub trait CommandHandler<C: Command>: Send + Sync + 'static {
	fn execute(
		&self,
		kv: KvHandle,
		request: Request<C>,
	) -> impl Future<Output = Result<Response<C>>> + Send;
}

impl<C, F, Fut> CommandHandler<C> for F
where
	C: Command,
	F: Fn(KvHandle, Request<C>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Response<C>>> + Send,
{
	fn execute(
		&self,
		kv: KvHandle,
		request: Request<C>,
	) -> impl Future<Output = Result<Response<C>>> + Send {
		self(kv, request)
	}
}

type Handler = dyn Fn(KvHandle, Vec<u8>) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync;
type ReadHandlerFn =
	dyn Fn(ReadHandle, Vec<u8>) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync;

/// How a command runs: through the volume writer, or as a read that can skip it.
enum RegisteredHandler {
	Write(Box<Handler>),
	Read(Box<ReadHandlerFn>),
}

impl RegisteredHandler {
	/// Runs the command through the volume writer.
	fn run_with_writer(&self, kv: KvHandle, body: Vec<u8>) -> BoxFuture<'static, Result<Vec<u8>>> {
		match self {
			RegisteredHandler::Write(handler) => handler(kv, body),
			RegisteredHandler::Read(handler) => handler(ReadHandle::from_writer(kv), body),
		}
	}
}

struct RegisteredCommand {
	handler: RegisteredHandler,
	timeout: Duration,
	concurrency: usize,
	/// How long a volume this command touched stays open while idle. `None` uses the host's
	/// `volume_idle_close_ms`; a volume takes the minimum over the commands of its open.
	idle_close: Option<Duration>,
	/// See `Command::SPECULATIVE_CREATE`.
	speculative_create: bool,
}

/// Register core and application commands once before serving; duplicate tags are rejected.
#[derive(Default)]
pub struct CommandRegistry {
	commands: BTreeMap<CommandTag, RegisteredCommand>,
}

impl CommandRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_core() -> Result<Self> {
		let mut registry = Self::new();
		registry.register::<depot_protocol::command::KvGet>(kv::get)?;
		registry.register::<depot_protocol::command::KvApply>(kv::apply)?;
		registry.register_read::<depot_protocol::command::GroupRead>(
			256,
			None,
			groups::group_read,
		)?;
		registry.register_read::<depot_protocol::command::EventsRead>(
			256,
			None,
			groups::events_read,
		)?;
		registry.register::<depot_protocol::command::GroupCreate>(groups::group_create)?;
		registry.register::<depot_protocol::command::GroupForkDraw>(groups::group_fork_draw)?;
		registry
			.register::<depot_protocol::command::GroupChildCreate>(groups::group_child_create)?;
		registry.register::<depot_protocol::command::GroupArchive>(groups::group_archive)?;
		registry.register::<depot_protocol::command::GroupChildArchived>(
			groups::group_child_archived,
		)?;
		registry.register::<depot_protocol::command::EventPost>(groups::event_post)?;
		Ok(registry)
	}

	pub fn register<C: Command>(&mut self, handler: impl CommandHandler<C>) -> Result<()> {
		self.register_with_concurrency::<C>(256, handler)
	}

	pub fn register_with_concurrency<C: Command>(
		&mut self,
		concurrency: usize,
		handler: impl CommandHandler<C>,
	) -> Result<()> {
		self.register_with::<C>(concurrency, None, handler)
	}

	/// Registers a command with its concurrency cap and an optional idle close. A volume that
	/// any command with a shorter idle close touched closes that much sooner after its last
	/// command, which also begins its maintainer unload.
	pub fn register_with<C: Command>(
		&mut self,
		concurrency: usize,
		idle_close: Option<Duration>,
		handler: impl CommandHandler<C>,
	) -> Result<()> {
		ensure!(
			!C::READ_ONLY,
			"a read-only command requires explicit read registration"
		);
		self.register_writer_backed::<C>(concurrency, idle_close, handler)
	}

	/// Registers a pure read over the existing writer-backed committed snapshot.
	/// The handler may obtain a snapshot and read metadata, but must not mutate,
	/// produce external effects, retain handles or spawn work. This is the same
	/// trusted command contract as idempotency; it does not enable writerless reads
	/// or change the snapshot consistency of an existing command.
	pub fn register_snapshot_read<C: Command>(
		&mut self,
		concurrency: usize,
		idle_close: Option<Duration>,
		handler: impl CommandHandler<C>,
	) -> Result<()> {
		ensure!(
			C::READ_ONLY,
			"snapshot read registration requires a read-only command"
		);
		ensure!(
			!(C::CREATE_IF_MISSING || C::SPECULATIVE_CREATE),
			"a read-only command cannot create its volume"
		);
		self.register_writer_backed::<C>(concurrency, idle_close, handler)
	}

	fn register_writer_backed<C: Command>(
		&mut self,
		concurrency: usize,
		idle_close: Option<Duration>,
		handler: impl CommandHandler<C>,
	) -> Result<()> {
		self.validate::<C>(concurrency, idle_close)?;
		let handler = Arc::new(handler);
		self.commands.insert(
			C::TAG,
			RegisteredCommand {
				timeout: C::TIMEOUT,
				concurrency,
				idle_close,
				speculative_create: C::SPECULATIVE_CREATE,
				handler: RegisteredHandler::Write(Box::new(move |kv, body| {
					let handler = handler.clone();
					Box::pin(async move {
						let request_version = embedded_version(&body)?;
						let request = C::Request::deserialize_with_embedded_version(&body)?;
						let response_version = response_version::<C>(request_version, |family| {
							kv.write_version(family)
						})?;
						let response = handler.execute(kv, request).await?;
						seal_response::<C>(&response, response_version)
					})
				})),
			},
		);
		Ok(())
	}

	/// Registers a read command. A read of a volume the host has not opened runs on maintainer
	/// point reads with no host slot, volume client, claim or append. When the maintainer
	/// refuses a read, the command is refused as overloaded and retried at the
	/// edge; a read never opens the writer. A read of an open volume runs through its writer,
	/// which uses `idle_close` as [`CommandRegistry::register_with`] does. A read command never
	/// creates a volume. A missing volume is reported only when the handler reads a key.
	pub fn register_read<C: Command>(
		&mut self,
		concurrency: usize,
		idle_close: Option<Duration>,
		handler: impl ReadHandler<C>,
	) -> Result<()> {
		ensure!(
			!C::CREATE_IF_MISSING,
			"a read command cannot create its volume"
		);
		self.validate::<C>(concurrency, idle_close)?;
		let handler = Arc::new(handler);
		self.commands.insert(
			C::TAG,
			RegisteredCommand {
				timeout: C::TIMEOUT,
				concurrency,
				idle_close,
				speculative_create: false,
				handler: RegisteredHandler::Read(Box::new(move |read, body| {
					let handler = handler.clone();
					Box::pin(async move {
						let request_version = embedded_version(&body)?;
						let request = C::Request::deserialize_with_embedded_version(&body)?;
						let response_version = response_version::<C>(request_version, |family| {
							read.versions.write_version(family)
						})?;
						let response = handler.execute(read, request).await?;
						seal_response::<C>(&response, response_version)
					})
				})),
			},
		);
		Ok(())
	}

	fn validate<C: Command>(&self, concurrency: usize, idle_close: Option<Duration>) -> Result<()> {
		ensure!(concurrency > 0, "command concurrency must be positive");
		ensure!(
			idle_close.is_none_or(|idle_close| !idle_close.is_zero()),
			"command idle close must be positive"
		);
		ensure!(
			!self.commands.contains_key(&C::TAG),
			"duplicate command tag {}",
			C::TAG.0
		);
		ensure!(!C::TIMEOUT.is_zero(), "command timeout must be positive");
		ensure!(
			rivet_keys::keys::tags::depot_command::str_from_key(C::TAG.0).is_some(),
			"undeclared command tag"
		);
		Ok(())
	}
}

/// A command's answer cannot be expressed at the version [`response_version`] selected for it:
/// the response family's negotiated version, capped at the request's version when both share a
/// family. The host answers no higher because a caller of the previous release reads no newer
/// one, and a caller of this release seals at the fleet's read floor, so it hits this while the
/// floor is below the answer's version: only when a storage worker of the previous release is
/// listed after grouped, forked or moving actors exist, which happens for one heartbeat ttl after
/// such a worker restarts once the roll is done (it then fences itself), or in a rollback.
///
/// The command may have been applied. A caller of this release reads this answer as a retryable
/// `depot.overloaded` (reason `response_version_pending`) and the edge sends the command again;
/// every command whose answer can need a newer version re-executes safely: `ActorRead` is a
/// read, `ActorCreate` of an existing actor wakes it with the same waiter, and `ActorEvent`
/// answers a version-only outcome only to a wake (`TryStart`, `Create`) or to a `Reallocate`
/// fenced by its epoch.
#[derive(Debug, serde::Serialize, serde::Deserialize, rivet_error::RivetError)]
#[error(
	"depot",
	"response_version_unavailable",
	"The command's answer cannot be expressed at the version its caller reads; it may have been applied.",
	"The command's answer cannot be expressed at version {version}, which its caller reads; it may have been applied."
)]
pub struct ResponseVersionUnavailable {
	pub version: u16,
}

/// The version header of a command body, which the caller sealed it at.
fn embedded_version(body: &[u8]) -> Result<u16> {
	let Some(header) = body.get(..2) else {
		anyhow::bail!("command body has no version header");
	};
	Ok(u16::from_le_bytes([header[0], header[1]]))
}

/// The version a command's answer is sealed at: the response family's negotiated version (see
/// [`body_version`]), so a caller on another binary of a rolling deploy decodes it. When the
/// request and response envelopes share a family, it is also no higher than the request's
/// version: a caller seals its request no higher than the version it reads, so a caller of an
/// older release, which seals and reads only an older version, decodes the answer. When no
/// version of that shared family is selectable, because a live storage worker of an older
/// release lacks the family, the caller sealed its request at its latest instead (see
/// `depot_client::command::request_version`), and the answer takes the request's version too.
/// Envelopes of different families number their versions independently, so the request's
/// version does not bound the answer then.
pub(crate) fn response_version<C: Command>(
	request_version: u16,
	write_version: impl FnOnce(rivet_data::families::VersionFamily) -> Result<u16>,
) -> Result<u16> {
	let negotiated = body_version::<C::Response>(write_version);
	let same_family = match (C::Request::FAMILY, C::Response::FAMILY) {
		(Some(request), Some(response)) => request == response,
		(Some(_), None) | (None, Some(_)) | (None, None) => false,
	};
	match (same_family, negotiated) {
		(true, Ok(negotiated)) => Ok(negotiated.min(request_version)),
		(true, Err(error)) => {
			tracing::debug!(
				?error,
				request_version,
				"no version of the command's family is selectable; answering at the request's version"
			);
			Ok(request_version)
		}
		(false, negotiated) => negotiated,
	}
}

/// Seals a command's answer at `version`, capped at the response envelope's latest. An answer
/// that version cannot express fails the command with [`ResponseVersionUnavailable`], which
/// travels in the command envelope every release decodes.
pub(crate) fn seal_response<C: Command>(response: &Response<C>, version: u16) -> Result<Vec<u8>> {
	let version = version.min(C::Response::LATEST);
	match C::Response::seal_to_vec(response, version) {
		Ok(body) => Ok(body),
		Err(vbare::EncodeError::Conversion(error)) if version < C::Response::LATEST => {
			tracing::warn!(
				?error,
				command = C::TAG.0,
				version,
				"a command answer cannot be expressed at the version its caller reads; a caller of an older release asked for state only a newer release writes"
			);
			Err(ResponseVersionUnavailable { version }.build())
		}
		Err(error) => Err(error.into()),
	}
}

pub(crate) struct LockEntry {
	end: Bytes,
	guard: DropGuard,
}
