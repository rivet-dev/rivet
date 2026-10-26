//! Entry point of the single-node tablet provisioning service.
//!
//! Provisioning is a create pass and a grant pass, and the process can stop between them. The
//! service records its intent under this worker's ID before the first create and removes it
//! once the manifest matches the plan, so a restart that finds tablets resumes only a layout
//! this service started. A cluster whose tablets exist without that intent belongs to an
//! operator and is never touched.

use std::time::Duration;

use anyhow::{Context, Result};
use depot_types::keys::s3::AutoProvisionIntentKey;
use depot_worker::{
	coordinator_requests::CoordinatorRequests, services::storage::CoordinatorActive,
};
use object_store::{ObjectStoreExt, PutPayload};
use rivet_data::generated::{
	depot_tablet_manifest_v1::TabletManifest, shared_depot_types_v1::StorageWorkerDisk,
};
use rivet_engine_context::EngineContext;
use rivet_keys::keys::codec::KeyAddress;
use rivet_service_manager::{Service, ServiceClass};
use tokio::sync::watch;
use uuid::Uuid;

use crate::depot_command::{self, provision};

/// Manifest reads made after this worker's coordinator is elected, while it publishes the
/// first manifest. A fresh cluster publishes it within seconds of startup.
const MANIFEST_ATTEMPTS: usize = 120;
const MANIFEST_RETRY: Duration = Duration::from_millis(500);
const PROVISION_CONCURRENCY: usize = 32;
/// How long one attempt waits for this worker's coordinator to be elected before failing for
/// the service manager to start another.
const COORDINATOR_DEADLINE: Duration = Duration::from_secs(60);

type Coordinators = watch::Receiver<Option<CoordinatorActive>>;

/// Creates and grants `tablets` tablets over this storage worker once its own tablet
/// coordinator admits commands, unless the cluster already has tablets this service did not
/// start. The service manager retries a failure, which waits for the coordinator again and
/// resumes from the manifest, so a tenure lost midway is not fatal.
pub fn service(tablets: usize, coordinators: Coordinators) -> Service {
	Service::new(
		"depot_auto_provision",
		ServiceClass::Once,
		move |_config, ctx| {
			let coordinators = coordinators.clone();
			async move { start(ctx, tablets, coordinators).await }
		},
		false,
	)
}

pub async fn start(
	ctx: EngineContext,
	tablets: usize,
	mut coordinators: Coordinators,
) -> Result<()> {
	// Commands sent before the election are refused with `NotCoordinator`.
	rivet_env::timeout(
		ctx.env().clock().as_ref(),
		COORDINATOR_DEADLINE,
		wait_for_coordinator(&mut coordinators),
	)
	.await
	.context("this worker's tablet coordinator was not elected before the deadline")??;

	let Some(provisioning) = Provisioning::begin(&ctx, tablets).await? else {
		return Ok(());
	};
	provisioning.create().await?;
	provisioning.grant().await?;
	provisioning.finish().await
}

/// Waits until this worker's tablet coordinator admits commands. A storage service restart
/// publishes a new view, which replaces the one being watched.
async fn wait_for_coordinator(coordinators: &mut Coordinators) -> Result<()> {
	loop {
		let current = coordinators.borrow_and_update().clone();
		let Some(mut active) = current else {
			coordinators
				.changed()
				.await
				.context("storage stopped before its coordinator started")?;
			continue;
		};
		let elected = rivet_env::select_biased!("a replaced coordinator view supersedes the current one",
			changed = coordinators.changed() => {
				changed.context("storage stopped before its coordinator was elected")?;
				continue;
			},
			elected = active.wait_for(Option::is_some) => elected.is_ok(),
		);
		if elected {
			return Ok(());
		}

		// The coordinator service stopped; its replacement arrives as a new view.
		coordinators
			.changed()
			.await
			.context("storage stopped before its coordinator was elected")?;
	}
}

/// One run of provisioning over this worker, after its intent is recorded.
pub(crate) struct Provisioning {
	ctx: EngineContext,
	requests: CoordinatorRequests,
	disks: Vec<StorageWorkerDisk>,
	planned: Vec<provision::PlannedTablet>,
	intent: AutoProvisionIntentKey,
}

impl Provisioning {
	/// Waits for the manifest and decides whether this service owns provisioning. An empty
	/// manifest records the intent before anything is created; tablets with a recorded intent
	/// are a run this service left unfinished; tablets without one return `None`.
	pub(crate) async fn begin(ctx: &EngineContext, tablets: usize) -> Result<Option<Self>> {
		let manifest = wait_for_manifest(ctx).await?;
		let worker = Uuid::from_bytes(*ctx.worker_identity().worker_id.as_bytes());
		let intent = AutoProvisionIntentKey::new(worker);

		if manifest.tablets.is_empty() {
			write_intent(ctx, &intent).await?;
		} else if !intent_exists(ctx, &intent).await? {
			tracing::debug!(
				tablets = manifest.tablets.len(),
				"cluster already has tablets; skipping auto provisioning"
			);
			return Ok(None);
		} else {
			tracing::info!(
				tablets = manifest.tablets.len(),
				%worker,
				"resuming interrupted auto provisioning"
			);
		}

		Ok(Some(Self {
			ctx: ctx.clone(),
			requests: depot_command::coordinator_requests(ctx)?,
			disks: depot_command::storage_disks(ctx, vec![worker]).await?,
			planned: provision::plan(tablets, 1)?,
			intent,
		}))
	}

	/// Creates the planned tablets the manifest does not have yet.
	async fn create(&self) -> Result<()> {
		provision::create_missing(
			&self.ctx,
			&self.requests,
			&self.disks,
			&self.planned,
			PROVISION_CONCURRENCY,
		)
		.await
	}

	/// Grants the planned tablets that have no maintainer.
	async fn grant(&self) -> Result<()> {
		provision::grant_ungranted(
			&self.ctx,
			&self.requests,
			&self.disks,
			&self.planned,
			PROVISION_CONCURRENCY,
		)
		.await
	}

	/// Verifies the plan, then removes the intent so later starts leave the cluster alone.
	async fn finish(&self) -> Result<()> {
		let result = provision::verify_placement(&self.ctx, &self.disks, &self.planned).await?;
		let timeout = request_timeout(&self.ctx);
		let stores = self.ctx.object_stores();
		let path = self.intent.pack();
		let delete = stores.primary.delete(&path);
		match rivet_env::timeout(self.ctx.env().clock().as_ref(), timeout, delete).await? {
			Ok(()) | Err(object_store::Error::NotFound { .. }) => {}
			Err(error) => {
				return Err(error).context("remove the auto provisioning intent");
			}
		}
		tracing::info!(
			tablets = self.planned.len(),
			worker = %self.intent.worker_id,
			placement = %result["placement"],
			"auto provisioned tablets"
		);
		Ok(())
	}
}

fn request_timeout(ctx: &EngineContext) -> Duration {
	Duration::from_millis(ctx.config().depot().volume_request_timeout_ms())
}

/// Records the intent with an unconditional put, so repeating it after a stop is harmless.
async fn write_intent(ctx: &EngineContext, intent: &AutoProvisionIntentKey) -> Result<()> {
	let stores = ctx.object_stores();
	let path = intent.pack();
	let put = stores.primary.put(&path, PutPayload::new());
	rivet_env::timeout(ctx.env().clock().as_ref(), request_timeout(ctx), put)
		.await?
		.context("record the auto provisioning intent")?;
	Ok(())
}

async fn intent_exists(ctx: &EngineContext, intent: &AutoProvisionIntentKey) -> Result<bool> {
	let stores = ctx.object_stores();
	let path = intent.pack();
	let head = stores.primary.head(&path);
	match rivet_env::timeout(ctx.env().clock().as_ref(), request_timeout(ctx), head).await? {
		Ok(_) => Ok(true),
		Err(object_store::Error::NotFound { .. }) => Ok(false),
		Err(error) => Err(error).context("read the auto provisioning intent"),
	}
}

/// The manifest is published by the coordinator after its election and offers no
/// notification, so this reads it on a short bounded retry. Exhausting the attempts returns
/// the last error for the service manager to retry.
async fn wait_for_manifest(ctx: &EngineContext) -> Result<TabletManifest> {
	let mut attempt = 0;
	loop {
		attempt += 1;
		match depot_command::read_manifest(ctx).await {
			Ok(manifest) => return Ok(manifest),
			Err(error) if attempt < MANIFEST_ATTEMPTS => {
				tracing::debug!(?error, attempt, "waiting for the tablet manifest");
				rivet_env::sleep(ctx.env().clock().as_ref(), MANIFEST_RETRY).await?;
			}
			Err(error) => return Err(error),
		}
	}
}
