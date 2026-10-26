//! Internal typed operator commands use the same coordinator and volume transports as applications.

#[path = "depot_command/balancing.rs"]
pub(crate) mod balancing;
#[path = "depot_command/health.rs"]
pub(crate) mod health;
#[path = "depot_command/jwt.rs"]
pub(crate) mod jwt;
#[path = "depot_command/kv.rs"]
pub(crate) mod kv;
#[path = "depot_command/provision.rs"]
pub(crate) mod provision;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use depot_client::volumes::Error as VolumeError;
use depot_volume::volume_client_machine::{event::RpcError, types::WriteError};
use depot_worker::{
	coordinator_requests::CoordinatorRequests,
	tablet_coordinator_machine::{store, types::ReadResult},
};
use rivet_data::generated::{
	depot_client_protocol_v1::VolumeRejectResponse,
	depot_storage_worker_protocol_v1::{
		GetDiskIncarnationResponse, RejectResponse, StorageWorkerRequest, StorageWorkerResponse,
	},
	depot_tablet_coordinator_protocol_v1::{
		AbortReconfiguration, CreateTablet, GrantTablet, MoveTablet, PauseTablet, ReplaceReplica,
		ResumeTablet, SplitTablet, TabletCoordinatorRequest, TabletCoordinatorResponse,
	},
	shared_depot_types_v1::VolumeLogKey,
};
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::shutdown::ShutdownTrigger;
use rivet_service_manager::{Service, ServiceClass};
use rivet_singleton_coordinator::{
	singleton_coordinator_machine::{
		executor::LeaseStore,
		types::{LeaseRead, ReleaseRequest},
	},
	store::{ObjectLeaseStore, OperatorWrite},
};
use serde_json::{Value, json};
use std::{future::Future, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{Mutex, oneshot};
use universalpubsub::SubjectCapacities;
use uuid::Uuid;

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Split a stable partition at the given hash, or at the midpoint of its range.
	Split {
		#[arg(value_name = "partition")]
		tablet: Uuid,
		split_key_hash: Option<u128>,
	},
	/// Inspect placement or submit a partition operation to the current coordinator.
	#[command(subcommand)]
	#[command(name = "partition")]
	Tablet(TabletCommand),
	/// Read and write filesystem files using volume addresses, raw names, and bytes encoded as hex.
	#[command(subcommand)]
	Volume(VolumeCommand),

	/// Read or mutate partition KV data through the replicated volume client.
	#[command(subcommand)]
	Kv(kv::Command),

	/// Inspect or hand over the regional singleton coordinator tenure.
	#[command(subcommand)]
	Coordinator(CoordinatorCommand),

	/// Inspect or change the cluster's dynamic automatic balancing switch.
	#[command(subcommand)]
	Balancing(balancing::Command),
}

pub enum OperatorCommand {
	Storage(Command),
	Jwt(jwt::Command),
}

#[derive(Debug, Subcommand)]
pub enum CoordinatorCommand {
	/// Print the coordinator lease: holder, generation, release request and step-down deadline.
	Show,
	/// Ask the current holder to hand the coordinator tenure over. It announces a step-down at
	/// its next manifest check, keeps serving until shortly before the deadline, then
	/// releases; another eligible storage worker claims at the deadline or the release, and
	/// the old holder stays out of that election for one step-down period. Repeating the
	/// request while it is pending changes nothing.
	StepDown {
		/// The worker expected to hold the tenure; refused when another worker holds it.
		#[arg(long)]
		holder: Option<Uuid>,
		/// The tenure generation `show` printed; refused when the tenure has moved on, even to
		/// the same worker, so a newer holder is never stepped down.
		#[arg(long)]
		expected_generation: u64,
	},
}

#[derive(Debug, Subcommand)]
pub enum TabletCommand {
	/// Split a stable partition at the given hash, or at the midpoint of its range.
	Split {
		#[arg(value_name = "partition")]
		tablet: Uuid,
		split_key_hash: Option<u128>,
	},
	Create {
		#[arg(long, default_value_t = 0)]
		start: u128,
		/// Exclusive end; omitted means the end of the 128-bit hash space.
		#[arg(long)]
		end: Option<u128>,
		#[arg(long, num_args = 1..=3, required = true)]
		members: Vec<Uuid>,
	},
	Grant {
		#[arg(value_name = "partition")]
		tablet: Uuid,
		maintainer: Uuid,
	},
	Move {
		#[arg(value_name = "partition")]
		tablet: Uuid,
		maintainer: Uuid,
	},
	/// Move the maintainer first if it is the member being replaced.
	ReplaceReplica {
		#[arg(value_name = "partition")]
		tablet: Uuid,
		leaving: Uuid,
		joining: Uuid,
	},
	/// Abort Learning; Joint must finish promotion.
	Abort {
		#[arg(value_name = "partition")]
		tablet: Uuid,
	},
	/// Hold the balancer's discretionary actions off one partition until `resume`: no automatic
	/// split, rebalancing replacement or grant move. A dead or stopping replica is still
	/// replaced, and operator commands, worker stop handoffs, missing-owner failover and the
	/// Learning deadline still apply. Cluster-wide, use `storage balancing disable`.
	Pause {
		#[arg(value_name = "partition")]
		tablet: Uuid,
	},
	/// Lift a `pause`.
	Resume {
		#[arg(value_name = "partition")]
		tablet: Uuid,
	},
	/// Create and grant every partition of a fresh cluster: equal hash ranges, rotating up-to-three-member
	/// windows over the listed workers, each partition maintained by its first member. Fails unless
	/// the manifest has no partitions, and verifies the result before printing the placement.
	Provision {
		#[arg(long = "partitions")]
		tablets: usize,
		/// Coordinator requests in flight at once.
		#[arg(long, default_value_t = 32)]
		concurrency: usize,
		#[arg(long, num_args = 1.., required = true)]
		workers: Vec<Uuid>,
	},
	/// Observe assigned disks and maintainer-log replicas without changing replica fences.
	Health {
		#[arg(value_name = "partition")]
		tablet: Option<Uuid>,
	},
	/// Print the current manifest; optionally select one partition.
	Show {
		#[arg(value_name = "partition")]
		tablet: Option<Uuid>,
	},
}

#[derive(Debug, Subcommand)]
pub enum VolumeCommand {
	Create {
		key: VolumeAddress,
	},
	Write {
		key: VolumeAddress,
		#[arg(long)]
		offset: u64,
		/// Root-directory file name encoded as hex. Creates the file when absent.
		file: HexBytes,
		/// File bytes encoded as hex.
		data: HexBytes,
	},
	Read {
		key: VolumeAddress,
		#[arg(long, default_value_t = 0)]
		offset: u64,
		#[arg(long, default_value_t = 4096)]
		count: usize,
		/// File path encoded as hex, relative to the volume root.
		file: HexBytes,
	},
	/// Require the incarnation so a delayed delete cannot remove a replacement volume.
	Delete {
		key: VolumeAddress,
		incarnation: Uuid,
	},
}

/// A volume address is partition-key hex, optionally followed by a colon and name hex.
/// A trailing colon names the empty name, which is distinct from an unnamed volume.
#[derive(Clone, Debug)]
pub struct VolumeAddress(depot_types::VolumeKey);

impl FromStr for VolumeAddress {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self> {
		let (partition, name) = match value.split_once(':') {
			Some((partition, name)) => (partition, Some(hex::decode(name)?)),
			None => (value, None),
		};
		Ok(Self(depot_types::VolumeKey {
			partition_key: (hex::decode(partition)?).into(),
			name: name.map(Into::into),
		}))
	}
}

/// Hex is a command-line codec only; requests carry the original bytes.
#[derive(Clone, Debug)]
pub struct HexBytes(Vec<u8>);

impl FromStr for HexBytes {
	type Err = hex::FromHexError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		hex::decode(value).map(Self)
	}
}

/// Report the command result separately from service success so the supervisor never retries
/// a possibly applied operator mutation. Shutdown keeps the peer and volume owners alive for closes.
pub fn service(
	command: OperatorCommand,
	trigger: ShutdownTrigger,
) -> (Service, oneshot::Receiver<Result<Value>>) {
	let (reply, result) = oneshot::channel();
	let operation = Arc::new(Mutex::new(Some((command, reply))));
	let service = Service::new(
		"depot_command",
		ServiceClass::Once,
		move |_, ctx| {
			let operation = operation.clone();
			let trigger = trigger.clone();
			async move {
				let operation = operation.lock().await.take();
				if let Some((command, reply)) = operation {
					let result = match command {
						OperatorCommand::Storage(command) => execute(&ctx, command).await,
						OperatorCommand::Jwt(command) => jwt::execute(&ctx, command).await,
					}
					.inspect_err(|error| {
						tracing::error!(?error, "operator command failed");
					});
					let _ = reply.send(result);
					trigger.request(ctx.env().clock().as_ref());
				}
				Ok(())
			}
		},
		true,
	);
	(service, result)
}

#[tracing::instrument(skip_all)]
pub(crate) async fn execute(ctx: &EngineContext, command: Command) -> Result<Value> {
	match command {
		Command::Split {
			tablet: id,
			split_key_hash,
		} => {
			tablet(
				ctx,
				TabletCommand::Split {
					tablet: id,
					split_key_hash,
				},
			)
			.await
		}
		Command::Tablet(command) => tablet(ctx, command).await,
		Command::Coordinator(command) => coordinator(ctx, command).await,
		Command::Balancing(command) => balancing::execute(ctx, command).await,
		Command::Kv(command) => {
			let client = ctx.depot();
			routed(ctx, || kv::execute(&client, &command)).await
		}
		Command::Volume(command) => {
			let client = ctx.depot();
			match command {
				VolumeCommand::Create { key } => {
					let row = routed(ctx, || client.create(key.0.clone())).await?;
					Ok(
						json!({ "volumeIncarnation": Uuid::from_bytes(row.volume_incarnation), "row": row }),
					)
				}
				VolumeCommand::Write {
					key,
					offset,
					file,
					data,
				} => {
					let owner = routed(ctx, || client.open_filesystem(key.0.clone())).await?;
					let fs = owner.filesystem();
					let ino = match fs.lookup(depot_fs_core::ROOT_INO, &file.0).await {
						Ok(entry) => entry.child,
						Err(depot_client::filesystem::Error::Filesystem(
							depot_fs_core::Error::NotFound,
						)) => {
							fs.create(
								depot_fs_core::ROOT_INO,
								&file.0,
								depot_fs_core::rules::new_inode(
									depot_fs_core::FsKind::File,
									0o600,
									0,
									0,
									0,
								),
							)
							.await?
						}
						Err(error) => return Err(error.into()),
					};
					let volume_seq = fs.write(ino, offset, &data.0).await?;
					owner.shutdown().await?;
					Ok(json!({ "volumeSeq": volume_seq, "bytesWritten": data.0.len() }))
				}
				VolumeCommand::Read {
					key,
					offset,
					count,
					file,
				} => {
					let owner = routed(ctx, || client.open_filesystem(key.0.clone())).await?;
					let fs = owner.filesystem();
					let ino = fs.resolve(depot_fs_core::ROOT_INO, &file.0, false).await?;
					let bytes = fs.read(ino, offset, count).await?;
					owner.shutdown().await?;
					Ok(json!({ "bytes": hex::encode(bytes) }))
				}
				VolumeCommand::Delete { key, incarnation } => {
					let key = depot_types::VolumeLogKey::from(VolumeLogKey {
						volume_key: key.0,
						volume_incarnation: *incarnation.as_bytes(),
					});
					routed(ctx, || client.delete(key.clone())).await?;
					Ok(json!({ "deleted": true, "volumeIncarnation": incarnation }))
				}
			}
		}
	}
}

/// The operator is the edge for its commands, so it alone retries them, and only after
/// explicit admission failures that prove the command was not executed, waiting each
/// refusal's own hint when it carries one. Every other outcome is returned unchanged,
/// including uncertain writes and their retry tokens.
async fn routed<T, E: Into<anyhow::Error>, F: Future<Output = Result<T, E>>>(
	ctx: &EngineContext,
	mut request: impl FnMut() -> F,
) -> Result<T> {
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(
		clock.as_ref(),
		Duration::from_millis(ctx.config().depot().volume_request_timeout_ms()),
	)?;
	loop {
		let error = match request().await {
			Ok(result) => return Ok(result),
			Err(error) => error.into(),
		};
		let Some(wait_ms) = refusal_wait_ms(&error) else {
			return Err(error);
		};
		if clock.observe()?.boot_ms >= deadline {
			bail!("volume admission deadline exceeded; command was not executed");
		}
		rivet_env::sleep(clock.as_ref(), Duration::from_millis(wait_ms)).await?;
	}
}

/// How long the operator waits before sending a command again that was refused without being
/// executed, or `None` when the error is final or its outcome may be unknown.
fn refusal_wait_ms(error: &anyhow::Error) -> Option<u64> {
	let volume_error = error.downcast_ref::<VolumeError>()?;
	let refused = matches!(
		volume_error,
		VolumeError::Lifecycle(
			RpcError::RoutingUnavailable
				| RpcError::Overloaded { .. }
				| RpcError::LifecycleRejected(
					VolumeRejectResponse::Overloaded(_)
						| VolumeRejectResponse::NotMaintainer(_)
						| VolumeRejectResponse::TabletSplitting
				)
		) | VolumeError::Overloaded
			| VolumeError::Filesystem(depot_client::filesystem::Error::ColdReadRefused)
			| VolumeError::Write(
				WriteError::Overloaded { token: None, .. }
					| WriteError::RoutingUnavailable
					| WriteError::Refused { .. }
			)
	);
	if !refused {
		return None;
	}
	Some(depot_client::retry_after_ms(error).unwrap_or(OPERATOR_RETRY_MS))
}

/// The wait before resending a command whose refusal carries no hint, such as a missing route.
const OPERATOR_RETRY_MS: u64 = 20;

/// One coordinator client for every request this operator process sends.
pub(crate) fn coordinator_requests(ctx: &EngineContext) -> Result<CoordinatorRequests> {
	let settings = ctx.config().depot();
	CoordinatorRequests::new(
		ctx.ups()?,
		ctx.topology(),
		Arc::new(ObjectLeaseStore::new(ctx.object_stores().lease.clone())),
		SubjectCapacities {
			incoming: settings.coordinator_request_capacity(),
			outgoing: settings.coordinator_request_capacity(),
		},
		Duration::from_millis(settings.volume_request_timeout_ms()),
	)
}

pub(crate) async fn read_manifest(
	ctx: &EngineContext,
) -> Result<rivet_data::generated::depot_tablet_manifest_v1::TabletManifest> {
	let view = rivet_env::timeout(
		ctx.env().clock().as_ref(),
		Duration::from_millis(ctx.config().depot().volume_request_timeout_ms()),
		store::read(ctx.object_stores().primary.as_ref()),
	)
	.await?;
	match view {
		ReadResult::Found(stored) => Ok(stored.manifest),
		other => bail!("manifest unavailable: {other:?}"),
	}
}

async fn tablet(ctx: &EngineContext, command: TabletCommand) -> Result<Value> {
	let (request, selected) = match command {
		TabletCommand::Provision {
			tablets,
			concurrency,
			workers,
		} => {
			let disks = storage_disks(ctx, workers).await?;
			let requests = coordinator_requests(ctx)?;
			return provision::provision(ctx, &requests, disks, tablets, concurrency).await;
		}
		TabletCommand::Split {
			tablet,
			split_key_hash,
		} => {
			let split_key_hash = match split_key_hash {
				Some(hash) => hash.to_be_bytes(),
				None => {
					let read = rivet_env::timeout(
						ctx.env().clock().as_ref(),
						Duration::from_millis(ctx.config().depot().volume_request_timeout_ms()),
						store::read(ctx.object_stores().primary.as_ref()),
					)
					.await?;
					let ReadResult::Found(stored) = read else {
						bail!("manifest unavailable: {read:?}");
					};
					let parent = stored
						.manifest
						.tablets
						.iter()
						.find(|entry| &entry.id == tablet.as_bytes())
						.ok_or_else(|| anyhow::anyhow!("partition not found"))?;
					parent
						.hash_range
						.midpoint()
						.ok_or_else(|| anyhow::anyhow!("partition range cannot split"))?
				}
			};
			// Publication can finish before this command reads back the manifest.
			// Return the complete view rather than requiring the parent to remain.
			(
				Some(TabletCoordinatorRequest::SplitTablet(SplitTablet {
					tablet_id: *tablet.as_bytes(),
					split_key_hash,
				})),
				None,
			)
		}
		TabletCommand::Create {
			start,
			end,
			members,
		} => (
			Some(TabletCoordinatorRequest::CreateTablet(CreateTablet {
				hash_range: rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange {
					start: start.to_be_bytes(),
					end: end.map(u128::to_be_bytes),
				},
				members: storage_disks(ctx, members).await?,
			})),
			None,
		),
		TabletCommand::Grant { tablet, maintainer } => (
			Some(TabletCoordinatorRequest::GrantTablet(GrantTablet {
				tablet_id: *tablet.as_bytes(),
				maintainer: *maintainer.as_bytes(),
			})),
			Some(tablet),
		),
		TabletCommand::Move { tablet, maintainer } => (
			Some(TabletCoordinatorRequest::MoveTablet(MoveTablet {
				tablet_id: *tablet.as_bytes(),
				maintainer: *maintainer.as_bytes(),
			})),
			Some(tablet),
		),
		TabletCommand::ReplaceReplica {
			tablet,
			leaving,
			joining,
		} => (
			Some(TabletCoordinatorRequest::ReplaceReplica(ReplaceReplica {
				tablet_id: *tablet.as_bytes(),
				leaving: *leaving.as_bytes(),
				joining: storage_disks(ctx, vec![joining]).await?.remove(0),
			})),
			Some(tablet),
		),
		TabletCommand::Abort { tablet } => (
			Some(TabletCoordinatorRequest::AbortReconfiguration(
				AbortReconfiguration {
					tablet_id: *tablet.as_bytes(),
				},
			)),
			Some(tablet),
		),
		TabletCommand::Pause { tablet } => (
			Some(TabletCoordinatorRequest::PauseTablet(PauseTablet {
				tablet_id: *tablet.as_bytes(),
			})),
			Some(tablet),
		),
		TabletCommand::Resume { tablet } => (
			Some(TabletCoordinatorRequest::ResumeTablet(ResumeTablet {
				tablet_id: *tablet.as_bytes(),
			})),
			Some(tablet),
		),
		TabletCommand::Health { tablet } => return health::execute(ctx, tablet).await,
		TabletCommand::Show { tablet } => (None, tablet),
	};

	// A confirmed coordinator reply is the command result; the following manifest is a fresh view.
	let timeout = Duration::from_millis(ctx.config().depot().volume_request_timeout_ms());
	let accepted = if let Some(request) = request {
		let requests = coordinator_requests(ctx)?;
		match requests.request(request).await? {
			TabletCoordinatorResponse::StopPermitted
			| TabletCoordinatorResponse::StopPending(_) => {
				anyhow::bail!("unexpected coordinator stop response to operator command")
			}
			TabletCoordinatorResponse::Accepted(accepted) => Some(accepted.tablet_manifest_version),
			TabletCoordinatorResponse::TabletCoordinatorRejectResponse(reason) => {
				bail!("coordinator rejected command: {reason:?}")
			}
		}
	} else {
		None
	};
	let view = rivet_env::timeout(
		ctx.env().clock().as_ref(),
		timeout,
		store::read(ctx.object_stores().primary.as_ref()),
	)
	.await?;
	let manifest = match view {
		ReadResult::Found(stored) => stored.manifest,
		other => {
			bail!("manifest unavailable after command (accepted version {accepted:?}): {other:?}")
		}
	};
	let paused = manifest.paused;
	let tablets = manifest
		.tablets
		.into_iter()
		.filter(|tablet| selected.is_none_or(|id| id.as_bytes() == &tablet.id))
		.map(|tablet| {
			let paused_at = paused
				.iter()
				.find(|pause| pause.tablet_id == tablet.id)
				.map(|pause| pause.paused_at);
			json!({
				"partitionId": Uuid::from_bytes(tablet.id),
				"maintainer": tablet.maintainer.map(Uuid::from_bytes),
				"partitionGrantEpoch": tablet.tablet_grant_epoch,
				"grantedAt": tablet.granted_at,
				"pausedAt": paused_at,
				"replicaSet": tablet.replica_set,
				"phaseStartedAt": tablet.phase_started_at,
				"hashRange": { "start": hex::encode(tablet.hash_range.start), "end": tablet.hash_range.end.map(hex::encode) },
				"split": tablet.split,
			})
		})
		.collect::<Vec<_>>();
	if selected.is_some() && tablets.is_empty() {
		bail!("partition not found (accepted version {accepted:?})");
	}
	Ok(
		json!({ "acceptedPartitionManifestVersion": accepted, "partitionManifestVersion": manifest.tablet_manifest_version, "partitions": tablets }),
	)
}

/// The operator reads the lease and conditions its request on that exact version, so a
/// holder that changed in between is reported instead of asked to step down.
async fn coordinator(ctx: &EngineContext, command: CoordinatorCommand) -> Result<Value> {
	let store = ObjectLeaseStore::new(ctx.object_stores().lease.clone());
	let timeout = Duration::from_millis(ctx.config().depot().volume_request_timeout_ms());
	let clock = ctx.env().clock();
	let lease = match rivet_env::timeout(clock.as_ref(), timeout, store.read()).await? {
		LeaseRead::Found(lease) => lease,
		LeaseRead::Missing => {
			bail!("no coordinator lease exists yet; no storage worker has claimed the singleton")
		}
		LeaseRead::Unknown => bail!("the coordinator lease could not be read"),
	};
	match command {
		CoordinatorCommand::Show => Ok(lease_view(&lease)),
		CoordinatorCommand::StepDown {
			holder,
			expected_generation,
		} => {
			if lease.released {
				bail!("no worker holds the coordinator tenure; the last holder released it");
			}
			let current = Uuid::from_bytes(*lease.tenure.owner.worker_id.as_bytes());
			if let Some(holder) = holder
				&& holder != current
			{
				bail!("the coordinator is held by {current}, not {holder}");
			}
			if expected_generation != lease.tenure.coordinator_generation {
				bail!(
					"the coordinator tenure is at generation {}, not {expected_generation}",
					lease.tenure.coordinator_generation
				);
			}
			if lease.release_request == ReleaseRequest::Pending {
				return Ok(
					json!({ "requested": false, "alreadyPending": true, "lease": lease_view(&lease) }),
				);
			}
			let commit_id = *ctx
				.env()
				.rng("depot.coordinator.step-down")
				.uuid_v4()
				.as_bytes();
			let outcome = rivet_env::timeout(
				clock.as_ref(),
				timeout,
				store.request_release(&lease, commit_id),
			)
			.await
			.map_err(|error| {
				anyhow::anyhow!(
					"the step-down request's outcome is unknown; run `coordinator show` before retrying: {error}"
				)
			})??;
			match outcome {
				OperatorWrite::Written => {}
				OperatorWrite::Superseded => bail!(
					"the coordinator lease changed while requesting the step-down; run `coordinator show` and retry"
				),
			}
			let after = match rivet_env::timeout(clock.as_ref(), timeout, store.read()).await? {
				LeaseRead::Found(lease) => Some(lease_view(&lease)),
				LeaseRead::Missing | LeaseRead::Unknown => None,
			};
			Ok(json!({ "requested": true, "lease": after }))
		}
	}
}

fn lease_view(
	lease: &rivet_singleton_coordinator::singleton_coordinator_machine::types::ObservedLease,
) -> Value {
	json!({
		"holder": Uuid::from_bytes(*lease.tenure.owner.worker_id.as_bytes()),
		"holderInstanceId": Uuid::from_bytes(*lease.tenure.owner.instance_id.as_bytes()),
		"coordinatorGeneration": lease.tenure.coordinator_generation,
		"released": lease.released,
		"releaseRequest": match lease.release_request {
			ReleaseRequest::None => "none",
			ReleaseRequest::Pending => "pending",
		},
		"stepDownMs": lease.step_down_ms,
		"stepDownAtUnixMs": lease.step_down_at_unix_ms(),
		"modifiedAtUnixMs": lease.modified_unix_ms,
		"version": lease.version,
	})
}



/// How long disk discovery waits before asking a recovering storage worker again.
const DISK_DISCOVERY_RETRY_MS: u64 = 20;

/// Asks one storage worker for its disk identity through `request`. A freshly started
/// worker answers `NotServing` until its log shard finishes recovery, so that answer is asked
/// again until `deadline_boot_ms`. Every other answer, including other rejections, returns
/// at once.
async fn disk_incarnation<F, Fut, E>(
	clock: &dyn rivet_env::Clock,
	deadline_boot_ms: u64,
	mut request: F,
) -> Result<GetDiskIncarnationResponse>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<StorageWorkerResponse, E>>,
	E: Into<anyhow::Error>,
{
	loop {
		let reply = request().await.map_err(Into::into)?;
		match reply {
			StorageWorkerResponse::GetDiskIncarnationResponse(reply) => return Ok(reply),
			StorageWorkerResponse::RejectResponse(RejectResponse::NotServing) => {
				if clock.observe()?.boot_ms >= deadline_boot_ms {
					bail!("storage worker still recovering its log shard at the deadline");
				}
				rivet_env::sleep(clock, Duration::from_millis(DISK_DISCOVERY_RETRY_MS)).await?;
			}
			reply => bail!("unexpected disk identity reply: {reply:?}"),
		}
	}
}

/// Placement records the disk each destination currently owns. Retries of the coordinator
/// request retain this observation; a disk replaced afterwards fails its first load fence.
pub(crate) async fn storage_disks(
	ctx: &EngineContext,
	workers: Vec<Uuid>,
) -> Result<Vec<rivet_data::generated::shared_depot_types_v1::StorageWorkerDisk>> {
	let settings = ctx.config().depot();
	let topology = ctx.topology();
	let client = depot_volume::ups::UpsLogClient::new(
		ctx.ups()?,
		SubjectCapacities {
			incoming: settings.log_request_capacity(),
			outgoing: settings.log_request_capacity(),
		},
		Duration::from_millis(settings.volume_request_timeout_ms()),
		Arc::new(move || topology.may_admit().unwrap_or(false)),
	)?;
	let clock = ctx.env().clock();
	let mut disks = Vec::with_capacity(workers.len());
	for worker in workers {
		let id = rivet_topology::worker::WorkerId::from_bytes(*worker.as_bytes());
		let deadline_boot_ms = rivet_env::deadline_after(
			clock.as_ref(),
			Duration::from_millis(settings.volume_request_timeout_ms()),
		)?;
		let reply = disk_incarnation(clock.as_ref(), deadline_boot_ms, || {
			client.request(&id, StorageWorkerRequest::GetDiskIncarnationRequest)
		})
		.await
		.with_context(|| format!("storage worker {id} did not return its disk identity"))?;
		anyhow::ensure!(
			reply.disk_incarnation != [0; 16],
			"storage worker returned an invalid disk identity"
		);
		disks.push(
			rivet_data::generated::shared_depot_types_v1::StorageWorkerDisk {
				worker_id: *worker.as_bytes(),
				disk_incarnation: reply.disk_incarnation,
			},
		);
	}
	Ok(disks)
}
