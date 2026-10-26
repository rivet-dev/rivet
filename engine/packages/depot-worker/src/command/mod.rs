//! Typed handlers execute beside the maintainer's filesystem, outside its lifecycle loop.
mod kv;
mod session;
mod speculation;
pub mod volume_host;

use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_client::filesystem::{Error, Filesystem};
pub use depot_client::filesystem::{Error as CommitError, Transaction};
use depot_protocol::command::{Command, CommandTag, Request, Response};
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use std::sync::Weak;
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};
use tokio_util::sync::{CancellationToken, DropGuard};
use vbare::Versioned;

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
}

/// The lowest version this binary writes of each family, for tests that run storage code
/// without a topology: handles around a test-owned filesystem, and supervisors with a stub
/// lease.
#[cfg(any(test, feature = "test-support"))]
pub struct LowestWriteVersions;

#[cfg(any(test, feature = "test-support"))]
impl rivet_topology::versions::WriteVersions for LowestWriteVersions {
	fn write_version(&self, family: rivet_data::families::VersionFamily) -> Result<u16> {
		rivet_topology::versions::lowest_write(
			family
				.info()
				.map(rivet_data::families::FamilyInfo::versions)
				.as_ref(),
			family,
		)
	}
}

/// The commits one command has in flight, and whether its caller's deadline has passed. A
/// commit dispatched before the deadline must run to its own outcome: dropping it abandons the
/// write and fences the filesystem. Work the command has not dispatched by then is refused, so
/// nothing it starts lands after the caller was told the command timed out.
#[derive(Debug)]
pub(crate) struct CommandCommits {
	in_flight: tokio::sync::watch::Sender<usize>,
	expired: std::sync::atomic::AtomicBool,
}

impl Default for CommandCommits {
	fn default() -> Self {
		Self {
			in_flight: tokio::sync::watch::Sender::new(0),
			expired: std::sync::atomic::AtomicBool::new(false),
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
}

impl rivet_topology::versions::WriteVersions for KvHandle {
	fn write_version(&self, family: rivet_data::families::VersionFamily) -> Result<u16> {
		self.versions.write_version(family)
	}
}

impl KvHandle {
	/// The version to encode a value of `family` stored in this volume at: the highest every
	/// live storage worker reads. A command takes every versioned value's version from here.
	pub fn write_version(&self, family: rivet_data::families::VersionFamily) -> Result<u16> {
		self.versions.write_version(family)
	}

	pub fn has_alarm_owner(
		&self,
		owner: &rivet_data::generated::shared_depot_alarms_v1::AlarmOwner,
	) -> bool {
		(self.may_serve)() && self.alarm_owner.as_ref() == Some(owner)
	}
	/// Constructs a command context around a test-owned filesystem and authority check.
	#[cfg(feature = "test-support")]
	pub fn for_test(
		filesystem: Arc<Filesystem>,
		ino: u64,
		may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
		operation_id: [u8; 16],
		volume_key: depot_types::VolumeKey,
	) -> Self {
		Self {
			alarm_owner: None,
			filesystem,
			ino,
			may_serve,
			operation_id,
			volume_key,
			locked_ranges: Default::default(),
			shared: Default::default(),
			commits: Default::default(),
			speculative: None,
			versions: Arc::new(LowestWriteVersions),
		}
	}

	/// A clone of this handle with its own command deadline, as another command of the same
	/// opening would get.
	#[cfg(feature = "test-support")]
	pub fn for_another_command(&self) -> Self {
		Self {
			commits: Default::default(),
			..self.clone()
		}
	}

	/// Passes this command's deadline, as the host does when the caller's deadline expires.
	#[cfg(feature = "test-support")]
	pub fn expire_for_test(&self) {
		self.commits.expire();
	}

	/// Whether this command's caller deadline has passed. Once it has, `begin`, `snapshot` and
	/// `commit` refuse, so a handler that coordinates other commands' work, such as an event
	/// group holder, must hand that work to a live command instead of taking it.
	pub fn expired(&self) -> bool {
		self.commits.expired()
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
		Self {
			source: ReadSource::Writer(kv),
			volume_key,
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
						let request = C::Request::deserialize_with_embedded_version(&body)?;
						let response = handler.execute(kv, request).await?;
						C::Response::serialize_with_embedded_version(&response, C::Response::LATEST)
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
						let request = C::Request::deserialize_with_embedded_version(&body)?;
						let response = handler.execute(read, request).await?;
						C::Response::serialize_with_embedded_version(&response, C::Response::LATEST)
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

pub(crate) struct LockEntry {
	end: Bytes,
	guard: DropGuard,
}




