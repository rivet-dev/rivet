use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicI32, AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use scc::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vbare::OwnedVersionedData;

use crate::{
	RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased, TransactionDriver},
	error::DatabaseError,
	key_selector::KeySelector,
	options::{ConflictRangeType, MutationType},
	range_option::RangeOption,
	tx_ops::{Operation, TransactionOperations},
	utils::{IsolationLevel, MaybeCommitted, calculate_tx_retry_backoff},
	value::{KeyValue, Slice, Value, Values},
};

use super::lease::SlateDbLease;

const PROTOCOL_VERSION: u16 = 1;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
pub trait SlateDbForwardingTransport: Send + Sync {
	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Duration,
	) -> Result<Option<Vec<u8>>>;

	async fn serve(
		&self,
		subject: String,
		handler: Arc<dyn SlateDbForwardingHandler>,
	) -> Result<Box<dyn SlateDbForwardingServerHandle>>;
}

#[async_trait]
pub trait SlateDbForwardingHandler: Send + Sync {
	async fn handle(&self, payload: Vec<u8>) -> Result<Vec<u8>>;
}

pub trait SlateDbForwardingServerHandle: Send + Sync {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireKeySelector {
	key: Vec<u8>,
	or_equal: bool,
	offset: i32,
}

impl WireKeySelector {
	fn from_selector(selector: &KeySelector<'_>) -> Self {
		Self {
			key: selector.key().to_vec(),
			or_equal: selector.or_equal(),
			offset: selector.offset(),
		}
	}

	fn to_selector(&self) -> KeySelector<'static> {
		KeySelector::new(self.key.clone().into(), self.or_equal, self.offset)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireRangeOption {
	begin: WireKeySelector,
	end: WireKeySelector,
	limit: Option<usize>,
	target_bytes: usize,
	mode: crate::options::StreamingMode,
	reverse: bool,
}

impl WireRangeOption {
	fn from_range(opt: &RangeOption<'_>) -> Self {
		Self {
			begin: WireKeySelector::from_selector(&opt.begin),
			end: WireKeySelector::from_selector(&opt.end),
			limit: opt.limit,
			target_bytes: opt.target_bytes,
			mode: opt.mode,
			reverse: opt.reverse,
		}
	}

	fn to_range(&self) -> RangeOption<'static> {
		RangeOption {
			begin: self.begin.to_selector(),
			end: self.end.to_selector(),
			limit: self.limit,
			target_bytes: self.target_bytes,
			mode: self.mode,
			reverse: self.reverse,
			__non_exhaustive: std::marker::PhantomData,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireOperation {
	SetValue {
		key: Vec<u8>,
		value: Vec<u8>,
	},
	Clear {
		key: Vec<u8>,
	},
	ClearRange {
		begin: Vec<u8>,
		end: Vec<u8>,
	},
	AtomicOp {
		key: Vec<u8>,
		param: Vec<u8>,
		op_type: MutationType,
	},
}

impl From<Operation> for WireOperation {
	fn from(value: Operation) -> Self {
		match value {
			Operation::SetValue { key, value } => Self::SetValue { key, value },
			Operation::Clear { key } => Self::Clear { key },
			Operation::ClearRange { begin, end } => Self::ClearRange { begin, end },
			Operation::AtomicOp {
				key,
				param,
				op_type,
			} => Self::AtomicOp {
				key,
				param,
				op_type,
			},
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireConflictRange {
	begin: Vec<u8>,
	end: Vec<u8>,
	conflict_type: ConflictRangeType,
}

impl From<(Vec<u8>, Vec<u8>, ConflictRangeType)> for WireConflictRange {
	fn from((begin, end, conflict_type): (Vec<u8>, Vec<u8>, ConflictRangeType)) -> Self {
		Self {
			begin,
			end,
			conflict_type,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireKeyValue {
	key: Vec<u8>,
	value: Vec<u8>,
}

impl From<KeyValue> for WireKeyValue {
	fn from(value: KeyValue) -> Self {
		let (key, value) = value.into_parts();
		Self { key, value }
	}
}

impl From<WireKeyValue> for KeyValue {
	fn from(value: WireKeyValue) -> Self {
		KeyValue::new(value.key, value.value)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireValues {
	values: Vec<WireKeyValue>,
	more: bool,
}

impl From<Values> for WireValues {
	fn from(value: Values) -> Self {
		Self {
			more: value.more(),
			values: value.into_vec().into_iter().map(Into::into).collect(),
		}
	}
}

impl From<WireValues> for Values {
	fn from(value: WireValues) -> Self {
		Values::with_more(value.values.into_iter().map(Into::into).collect(), value.more)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireRequestBody {
	Get {
		key: Vec<u8>,
	},
	GetKey {
		selector: WireKeySelector,
	},
	GetRange {
		opt: WireRangeOption,
		iteration: usize,
	},
	GetEstimatedRangeSizeBytes {
		begin: Vec<u8>,
		end: Vec<u8>,
	},
	Commit {
		operations: Vec<WireOperation>,
		conflict_ranges: Vec<WireConflictRange>,
	},
	Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireRequest {
	txn_id: [u8; 16],
	body: WireRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireError {
	NotCommitted,
	TransactionTooOld,
	MaxRetriesReached,
	UsedDuringCommit,
	Other { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireResponseBody {
	Value(Option<Vec<u8>>),
	Key(Vec<u8>),
	Values(WireValues),
	EstimatedRangeSizeBytes(i64),
	Committed,
	Canceled,
	Error(WireError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireResponse {
	body: WireResponseBody,
}

enum VersionedWireRequest {
	V1(WireRequest),
}

impl OwnedVersionedData for VersionedWireRequest {
	type Latest = WireRequest;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid SlateDB forwarding request version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedWireResponse {
	V1(WireResponse),
}

impl OwnedVersionedData for VersionedWireResponse {
	type Latest = WireResponse;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid SlateDB forwarding response version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

fn encode_request(request: WireRequest) -> Result<Vec<u8>> {
	VersionedWireRequest::wrap_latest(request).serialize_with_embedded_version(PROTOCOL_VERSION)
}

fn decode_request(payload: &[u8]) -> Result<WireRequest> {
	VersionedWireRequest::deserialize_with_embedded_version(payload)
}

fn encode_response(response: WireResponse) -> Result<Vec<u8>> {
	VersionedWireResponse::wrap_latest(response).serialize_with_embedded_version(PROTOCOL_VERSION)
}

fn decode_response(payload: &[u8]) -> Result<WireResponse> {
	VersionedWireResponse::deserialize_with_embedded_version(payload)
}

fn wire_error_from_anyhow(error: &anyhow::Error) -> WireError {
	match error.chain().find_map(|err| err.downcast_ref::<DatabaseError>()) {
		Some(DatabaseError::NotCommitted) => WireError::NotCommitted,
		Some(DatabaseError::TransactionTooOld) => WireError::TransactionTooOld,
		Some(DatabaseError::MaxRetriesReached) => WireError::MaxRetriesReached,
		Some(DatabaseError::UsedDuringCommit) => WireError::UsedDuringCommit,
		None => WireError::Other {
			message: error.to_string(),
		},
	}
}

fn anyhow_from_wire_error(error: WireError) -> anyhow::Error {
	match error {
		WireError::NotCommitted => DatabaseError::NotCommitted.into(),
		WireError::TransactionTooOld => DatabaseError::TransactionTooOld.into(),
		WireError::MaxRetriesReached => DatabaseError::MaxRetriesReached.into(),
		WireError::UsedDuringCommit => DatabaseError::UsedDuringCommit.into(),
		WireError::Other { message } => anyhow!(message),
	}
}

fn now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
		.unwrap_or(0)
}

#[derive(Clone)]
pub struct SlateDbForwardingClient {
	transport: Arc<dyn SlateDbForwardingTransport>,
	lease: Option<SlateDbLease>,
	default_subject: String,
}

impl SlateDbForwardingClient {
	pub fn new(
		transport: Arc<dyn SlateDbForwardingTransport>,
		lease: Option<SlateDbLease>,
		default_subject: String,
	) -> Self {
		Self {
			transport,
			lease,
			default_subject,
		}
	}

	async fn current_subject(&self) -> Result<String> {
		if let Some(lease) = &self.lease {
			if let Some(state) = lease
				.read_current()
				.await
				.context("failed to read SlateDB forwarding lease")?
			{
				if state.body.expires_at_ms > now_ms() {
					return Ok(state.body.nats_subject);
				}
			}
		}

		Ok(self.default_subject.clone())
	}

	async fn request(&self, txn_id: Uuid, body: WireRequestBody) -> Result<WireResponseBody> {
		let payload = encode_request(WireRequest {
			txn_id: *txn_id.as_bytes(),
			body,
		})?;
		let subject = self.current_subject().await?;

		let Some(payload) = self
			.transport
			.request(&subject, &payload, REQUEST_TIMEOUT)
			.await
			.with_context(|| format!("failed to request SlateDB leader on subject {subject}"))?
		else {
			return Err(DatabaseError::NotCommitted.into());
		};

		match decode_response(&payload)?.body {
			WireResponseBody::Error(error) => Err(anyhow_from_wire_error(error)),
			body => Ok(body),
		}
	}
}

pub struct SlateDbForwardingDatabaseDriver {
	client: Arc<SlateDbForwardingClient>,
	pub(super) max_retries: AtomicI32,
}

impl SlateDbForwardingDatabaseDriver {
	pub fn new(client: SlateDbForwardingClient) -> Self {
		Self {
			client: Arc::new(client),
			max_retries: AtomicI32::new(100),
		}
	}
}

impl DatabaseDriver for SlateDbForwardingDatabaseDriver {
	fn create_txn(&self) -> Result<Transaction> {
		Ok(Transaction::new(Arc::new(
			SlateDbForwardingTransactionDriver::new(self.client.clone()),
		)))
	}

	fn run<'a>(
		&'a self,
		closure: Box<dyn Fn(RetryableTransaction) -> BoxFut<'a, Result<Erased>> + Send + Sync + 'a>,
	) -> BoxFut<'a, Result<Erased>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = self.max_retries.load(Ordering::SeqCst);

			for attempt in 0..max_retries {
				let tx = self.create_txn()?;
				let mut retryable = RetryableTransaction::new(tx);
				retryable.maybe_committed = maybe_committed;

				let error =
					match tokio::time::timeout(crate::transaction::TXN_TIMEOUT, closure(retryable.clone()))
						.await
					{
						Ok(Ok(res)) => match retryable.inner.driver.commit_ref().await {
							Ok(_) => return Ok(res),
							Err(e) => e,
						},
						Ok(Err(e)) => e,
						Err(_) => anyhow::Error::from(DatabaseError::TransactionTooOld),
					};

				let chain = error
					.chain()
					.find_map(|x| x.downcast_ref::<DatabaseError>());

				if let Some(db_error) = chain
					&& db_error.is_retryable()
				{
					if db_error.is_maybe_committed() {
						maybe_committed = MaybeCommitted(true);
					}

					let backoff_ms = calculate_tx_retry_backoff(attempt as usize);
					tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
					continue;
				}

				return Err(error);
			}

			Err(DatabaseError::MaxRetriesReached.into())
		})
	}

	fn txn_retry_limit(&self, limit: i32) -> Result<()> {
		self.max_retries.store(limit, Ordering::SeqCst);
		Ok(())
	}
}

pub struct SlateDbForwardingTransactionDriver {
	client: Arc<SlateDbForwardingClient>,
	txn_id: Uuid,
	operations: TransactionOperations,
	committed: AtomicU64,
}

impl SlateDbForwardingTransactionDriver {
	fn new(client: Arc<SlateDbForwardingClient>) -> Self {
		Self {
			client,
			txn_id: Uuid::new_v4(),
			operations: TransactionOperations::default(),
			committed: AtomicU64::new(0),
		}
	}

	async fn remote_get(&self, key: Vec<u8>) -> Result<Option<Slice>> {
		match self
			.client
			.request(self.txn_id, WireRequestBody::Get { key })
			.await?
		{
			WireResponseBody::Value(value) => Ok(value.map(Into::into)),
			_ => bail!("unexpected SlateDB forwarding get response"),
		}
	}

	async fn remote_get_key(&self, selector: WireKeySelector) -> Result<Slice> {
		match self
			.client
			.request(self.txn_id, WireRequestBody::GetKey { selector })
			.await?
		{
			WireResponseBody::Key(key) => Ok(key.into()),
			_ => bail!("unexpected SlateDB forwarding get_key response"),
		}
	}

	async fn remote_get_range(&self, opt: WireRangeOption, iteration: usize) -> Result<Values> {
		match self
			.client
			.request(
				self.txn_id,
				WireRequestBody::GetRange {
					opt,
					iteration,
				},
			)
			.await?
		{
			WireResponseBody::Values(values) => Ok(values.into()),
			_ => bail!("unexpected SlateDB forwarding get_range response"),
		}
	}

	async fn commit_inner(&self) -> Result<()> {
		if self.committed.swap(1, Ordering::SeqCst) == 1 {
			return Ok(());
		}

		let (operations, conflict_ranges) = self.operations.consume();
		match self
			.client
			.request(
				self.txn_id,
				WireRequestBody::Commit {
					operations: operations.into_iter().map(Into::into).collect(),
					conflict_ranges: conflict_ranges.into_iter().map(Into::into).collect(),
				},
			)
			.await?
		{
			WireResponseBody::Committed => Ok(()),
			_ => bail!("unexpected SlateDB forwarding commit response"),
		}
	}
}

impl TransactionDriver for SlateDbForwardingTransactionDriver {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.operations.atomic_op(key, param, op_type);
	}

	fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Option<Slice>>> + Send + 'a>> {
		let key = key.to_vec();
		Box::pin(async move {
			self.operations
				.get_with_callback(&key, isolation_level, || async {
					self.remote_get(key.clone()).await
				})
				.await
		})
	}

	fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Slice>> + Send + 'a>> {
		let selector = selector.clone();
		let wire_selector = WireKeySelector::from_selector(&selector);
		Box::pin(async move {
			self.operations
				.get_key(&selector, isolation_level, || async {
					self.remote_get_key(wire_selector).await
				})
				.await
		})
	}

	fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Values>> + Send + 'a>> {
		let opt = opt.clone();
		let wire_opt = WireRangeOption::from_range(&opt);
		Box::pin(async move {
			self.operations
				.get_range(&opt, isolation_level, || async {
					self.remote_get_range(wire_opt, iteration).await
				})
				.await
		})
	}

	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		let fut = async move {
			match self.get_range(&opt, 1, isolation_level).await {
				Ok(values) => values
					.into_iter()
					.map(|kv| Ok(Value::from_keyvalue(kv)))
					.collect::<Vec<_>>(),
				Err(e) => vec![Err(e)],
			}
		};

		Box::pin(stream::once(fut).flat_map(stream::iter))
	}

	fn set(&self, key: &[u8], value: &[u8]) {
		self.operations.set(key, value);
	}

	fn clear(&self, key: &[u8]) {
		self.operations.clear(key);
	}

	fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.operations.clear_range(begin, end);
	}

	fn commit(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		Box::pin(async move { self.commit_inner().await })
	}

	fn reset(&mut self) {
		self.operations.clear_all();
		self.committed.store(0, Ordering::SeqCst);
		self.txn_id = Uuid::new_v4();
	}

	fn cancel(&self) {
		self.operations.clear_all();
		self.committed.store(1, Ordering::SeqCst);
		let client = self.client.clone();
		let txn_id = self.txn_id;
		tokio::spawn(async move {
			if let Err(error) = client.request(txn_id, WireRequestBody::Cancel).await {
				tracing::debug!(?error, "failed to cancel forwarded SlateDB transaction");
			}
		});
	}

	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		self.operations
			.add_conflict_range(begin, end, conflict_type);
		Ok(())
	}

	fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		let begin = begin.to_vec();
		let end = end.to_vec();
		Box::pin(async move {
			match self
				.client
				.request(
					self.txn_id,
					WireRequestBody::GetEstimatedRangeSizeBytes { begin, end },
				)
				.await?
			{
				WireResponseBody::EstimatedRangeSizeBytes(bytes) => Ok(bytes),
				_ => bail!("unexpected SlateDB forwarding size-estimate response"),
			}
		})
	}

	fn commit_ref(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async move { self.commit_inner().await })
	}
}

pub struct SlateDbForwardingServer {
	_handle: Box<dyn SlateDbForwardingServerHandle>,
}

impl SlateDbForwardingServer {
	pub fn spawn(
		transport: Arc<dyn SlateDbForwardingTransport>,
		subject: String,
		local_driver: Arc<super::database::SlateDbDatabaseDriver>,
	) -> impl Future<Output = Result<Self>> + Send {
		let handler = Arc::new(LocalForwardingHandler {
			sessions: Arc::new(HashMap::new()),
			local_driver,
		});
		async move {
			let handle = transport.serve(subject, handler).await?;
			Ok(Self { _handle: handle })
		}
	}
}

struct LocalForwardingHandler {
	sessions: Arc<HashMap<Uuid, Transaction>>,
	local_driver: Arc<super::database::SlateDbDatabaseDriver>,
}

#[async_trait]
impl SlateDbForwardingHandler for LocalForwardingHandler {
	async fn handle(&self, payload: Vec<u8>) -> Result<Vec<u8>> {
		let response =
			match handle_forwarded_request(&self.sessions, &self.local_driver, &payload).await {
				Ok(body) => WireResponse { body },
				Err(error) => WireResponse {
					body: WireResponseBody::Error(wire_error_from_anyhow(&error)),
				},
			};

		encode_response(response)
	}
}

async fn get_or_create_session(
	sessions: &HashMap<Uuid, Transaction>,
	local_driver: &Arc<super::database::SlateDbDatabaseDriver>,
	txn_id: Uuid,
) -> Result<Transaction> {
	if let Some(entry) = sessions.get_async(&txn_id).await {
		return Ok(entry.get().clone());
	}

	let tx = local_driver.create_txn()?;
	match sessions.entry_async(txn_id).await {
		scc::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
		scc::hash_map::Entry::Vacant(entry) => {
			entry.insert_entry(tx.clone());
			Ok(tx)
		}
	}
}

async fn handle_forwarded_request(
	sessions: &HashMap<Uuid, Transaction>,
	local_driver: &Arc<super::database::SlateDbDatabaseDriver>,
	payload: &[u8],
) -> Result<WireResponseBody> {
	let request = decode_request(payload)?;
	let txn_id = Uuid::from_bytes(request.txn_id);

	match request.body {
		WireRequestBody::Get { key } => {
			let tx = get_or_create_session(sessions, local_driver, txn_id).await?;
			Ok(WireResponseBody::Value(
				tx.informal()
					.get(&key, IsolationLevel::Snapshot)
					.await?
					.map(Into::into),
			))
		}
		WireRequestBody::GetKey { selector } => {
			let tx = get_or_create_session(sessions, local_driver, txn_id).await?;
			let selector = selector.to_selector();
			Ok(WireResponseBody::Key(
				tx.informal()
					.get_key(&selector, IsolationLevel::Snapshot)
					.await?
					.into(),
			))
		}
		WireRequestBody::GetRange { opt, iteration } => {
			let tx = get_or_create_session(sessions, local_driver, txn_id).await?;
			let opt = opt.to_range();
			Ok(WireResponseBody::Values(
				tx.informal()
					.get_range(&opt, iteration, IsolationLevel::Snapshot)
					.await?
					.into(),
			))
		}
		WireRequestBody::GetEstimatedRangeSizeBytes { begin, end } => {
			let tx = get_or_create_session(sessions, local_driver, txn_id).await?;
			Ok(WireResponseBody::EstimatedRangeSizeBytes(
				tx.informal()
					.get_estimated_range_size_bytes(&begin, &end)
					.await?,
			))
		}
		WireRequestBody::Commit {
			operations,
			conflict_ranges,
		} => {
			let tx = get_or_create_session(sessions, local_driver, txn_id).await?;
			for operation in operations {
				apply_forwarded_operation(&tx, operation);
			}
			for conflict_range in conflict_ranges {
				tx.informal().add_conflict_range(
					&conflict_range.begin,
					&conflict_range.end,
					conflict_range.conflict_type,
				)?;
			}

			let result = tx.driver.commit_ref().await;
			let _ = sessions.remove_async(&txn_id).await;
			result?;
			Ok(WireResponseBody::Committed)
		}
		WireRequestBody::Cancel => {
			let _ = sessions.remove_async(&txn_id).await;
			Ok(WireResponseBody::Canceled)
		}
	}
}

fn apply_forwarded_operation(tx: &Transaction, operation: WireOperation) {
	match operation {
		WireOperation::SetValue { key, value } => tx.informal().set(&key, &value),
		WireOperation::Clear { key } => tx.informal().clear(&key),
		WireOperation::ClearRange { begin, end } => tx.informal().clear_range(&begin, &end),
		WireOperation::AtomicOp {
			key,
			param,
			op_type,
		} => tx.informal().atomic_op(&key, &param, op_type),
	}
}
