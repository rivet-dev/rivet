use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(feature = "test-faults")]
use crate::fault::{
	ColdTierFaultPoint, DepotFaultAction, DepotFaultContext, DepotFaultController, DepotFaultPoint,
};
use crate::metrics;

use super::{ColdTier, ColdTierObjectMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColdTierOperation {
	Put,
	Get,
	Delete,
	List,
}

#[derive(Debug)]
pub struct FaultyColdTier<T> {
	inner: T,
	node_id: String,
	state: Arc<FaultyColdTierState>,
	#[cfg(feature = "test-faults")]
	fault_controller: Option<DepotFaultController>,
}

#[derive(Debug, Default)]
struct FaultyColdTierState {
	latency_ms: AtomicU64,
	fail_puts: AtomicBool,
	fail_gets: AtomicBool,
	fail_deletes: AtomicBool,
	fail_lists: AtomicBool,
	fail_next_operations: AtomicUsize,
}

impl<T> FaultyColdTier<T> {
	pub fn new(inner: T, node_id: impl Into<String>) -> Self {
		FaultyColdTier {
			inner,
			node_id: node_id.into(),
			state: Arc::new(FaultyColdTierState::default()),
			#[cfg(feature = "test-faults")]
			fault_controller: None,
		}
	}

	#[cfg(feature = "test-faults")]
	pub fn new_with_fault_controller_for_test(
		inner: T,
		node_id: impl Into<String>,
		fault_controller: DepotFaultController,
	) -> Self {
		FaultyColdTier {
			inner,
			node_id: node_id.into(),
			state: Arc::new(FaultyColdTierState::default()),
			fault_controller: Some(fault_controller),
		}
	}

	pub fn set_latency(&self, latency: Duration) {
		self.state
			.latency_ms
			.store(latency.as_millis() as u64, Ordering::SeqCst);
	}

	pub fn fail_operation(&self, operation: ColdTierOperation, enabled: bool) {
		let flag = match operation {
			ColdTierOperation::Put => &self.state.fail_puts,
			ColdTierOperation::Get => &self.state.fail_gets,
			ColdTierOperation::Delete => &self.state.fail_deletes,
			ColdTierOperation::List => &self.state.fail_lists,
		};
		flag.store(enabled, Ordering::SeqCst);
	}

	pub fn fail_next_operations(&self, count: usize) {
		self.state
			.fail_next_operations
			.store(count, Ordering::SeqCst);
	}

	async fn maybe_fail(&self, operation: ColdTierOperation) -> Result<()> {
		let latency_ms = self.state.latency_ms.load(Ordering::SeqCst);
		if latency_ms > 0 {
			tokio::time::sleep(Duration::from_millis(latency_ms)).await;
		}

		let fail_by_operation = match operation {
			ColdTierOperation::Put => self.state.fail_puts.load(Ordering::SeqCst),
			ColdTierOperation::Get => self.state.fail_gets.load(Ordering::SeqCst),
			ColdTierOperation::Delete => self.state.fail_deletes.load(Ordering::SeqCst),
			ColdTierOperation::List => self.state.fail_lists.load(Ordering::SeqCst),
		};

		let fail_next = self
			.state
			.fail_next_operations
			.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
				(remaining > 0).then_some(remaining - 1)
			})
			.is_ok();

		if fail_by_operation || fail_next {
			metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
				.with_label_values(&[self.node_id.as_str(), operation.as_label()])
				.inc();
			anyhow::bail!("injected cold-tier failure for {operation:?}");
		}

		Ok(())
	}

	#[cfg(feature = "test-faults")]
	async fn maybe_fire_controller_fault(
		&self,
		point: ColdTierFaultPoint,
	) -> Result<Option<DepotFaultAction>> {
		let Some(controller) = &self.fault_controller else {
			return Ok(None);
		};
		let Some(fired) = controller
			.maybe_fire(DepotFaultPoint::ColdTier(point), DepotFaultContext::new())
			.await?
		else {
			return Ok(None);
		};

		Ok(Some(fired.action))
	}
}

impl ColdTierOperation {
	fn as_label(self) -> &'static str {
		match self {
			ColdTierOperation::Put => "put",
			ColdTierOperation::Get => "get",
			ColdTierOperation::Delete => "delete",
			ColdTierOperation::List => "list",
		}
	}
}

impl<T> Clone for FaultyColdTier<T>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		FaultyColdTier {
			inner: self.inner.clone(),
			node_id: self.node_id.clone(),
			state: self.state.clone(),
			#[cfg(feature = "test-faults")]
			fault_controller: self.fault_controller.clone(),
		}
	}
}

#[async_trait]
impl<T> ColdTier for FaultyColdTier<T>
where
	T: ColdTier,
{
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.maybe_fail(ColdTierOperation::Put).await?;
		#[cfg(feature = "test-faults")]
		let drop_ack = matches!(
			self.maybe_fire_controller_fault(ColdTierFaultPoint::PutObject)
				.await?,
			Some(DepotFaultAction::DropArtifact)
		);
		self.inner.put_object(key, bytes).await.and_then(|()| {
			#[cfg(feature = "test-faults")]
			if drop_ack {
				anyhow::bail!("injected cold-tier put acknowledgement drop for {key}");
			}

			Ok(())
		})
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.maybe_fail(ColdTierOperation::Get).await?;
		#[cfg(feature = "test-faults")]
		if matches!(
			self.maybe_fire_controller_fault(ColdTierFaultPoint::GetObject)
				.await?,
			Some(DepotFaultAction::DropArtifact)
		) {
			return Ok(None);
		}
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.maybe_fail(ColdTierOperation::Delete).await?;
		#[cfg(feature = "test-faults")]
		if matches!(
			self.maybe_fire_controller_fault(ColdTierFaultPoint::DeleteObjects)
				.await?,
			Some(DepotFaultAction::DropArtifact)
		) {
			anyhow::bail!("cold-tier DropArtifact is not supported for delete_objects");
		}
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.maybe_fail(ColdTierOperation::List).await?;
		#[cfg(feature = "test-faults")]
		if matches!(
			self.maybe_fire_controller_fault(ColdTierFaultPoint::ListPrefix)
				.await?,
			Some(DepotFaultAction::DropArtifact)
		) {
			anyhow::bail!("cold-tier DropArtifact is not supported for list_prefix");
		}
		self.inner.list_prefix(prefix).await
	}
}
