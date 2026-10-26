//! Explicitly owned singleton lifecycle and the per-process signing/verifying cache.
use crate::key_ring_machine::{
	KeyRingStateMachine, event::Event, executor::RingExecutor, state::State, types::Settings,
};
use crate::store::SigningKeyRingKey;
use crate::*;
use anyhow::{Context, Result, ensure};
use object_store::{GetOptions, ObjectStore};
use rivet_config::Config;
use rivet_env::Env;
use rivet_keys::keys::KeyAddress;
use rivet_singleton_coordinator::authority::Authority;
use rivet_state_machine_ephemeral::Runner;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn policy(config: &Config) -> Result<RotationPolicy> {
	let jwt = &config.auth_required()?.jwt;
	Ok(RotationPolicy {
		rotation_interval_ms: jwt.key_rotation_interval().as_millis().try_into()?,
		publish_lead_ms: jwt.key_publish_lead().as_millis().try_into()?,
		max_signing_lifetime_ms: jwt.key_max_signing_lifetime().as_millis().try_into()?,
		max_token_ttl_ms: jwt.max_duration().as_millis().try_into()?,
		clock_skew_ms: (PROTOCOL_CLOCK_SKEW * 1000) as i64,
	})
}
/// Own and join the key-ring runner inside the existing coordinator tenure.
/// Only the configured leader datacenter mutates the cluster-wide ring.
pub async fn run(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	config: Config,
	authority: Authority,
	stop: tokio_util::sync::CancellationToken,
	pubsub: universalpubsub::PubSub,
) -> Result<()> {
	if !config.auth_required()?.jwt.enabled() || !config.is_leader() {
		rivet_env::select_biased!("JWT stop", _ = stop.cancelled() => {}, _ = authority.revoked() => {},);
		return Ok(());
	}
	let state = State::new(Settings {
		epoch: authority.tenure().coordinator_generation,
		datacenter_id: config.dc_label(),
		issuer: rivet_config::config::auth::derive_issuer(&config)?,
		audience: config.auth_required()?.jwt.audience().into(),
		policy: policy(&config)?,
	})?;
	let tasks = env.task_scope();
	let replies = Arc::new(crate::rotation::Replies::default());
	let executor_replies = replies.clone();
	let mut executor = None;
	let mut runner = Runner::<KeyRingStateMachine>::spawn_with_executor(env, state, |snapshot| {
		let created = Arc::new(RingExecutor {
			store,
			authority: authority.clone(),
			snapshot,
			tasks,
			active: parking_lot::Mutex::new(None),
			replies: executor_replies,
		});
		executor = Some(created.clone());
		created
	});
	let executor = executor.expect("executor constructed synchronously");
	let transport = crate::rotation::serve(
		pubsub,
		authority.clone(),
		runner.inbox.clone(),
		replies.clone(),
		config.clone(),
	);
	let cancelled = async {
		rivet_env::select_biased!("JWT authority revocation precedes shutdown",
			_ = authority.revoked() => (),
			_ = stop.cancelled() => (),
		)
	};
	let result = rivet_env::select_biased!("JWT tenure stops before further maintenance",
		_ = cancelled => Ok(()),
		_ = runner.snapshot.wait_for(|view| view.stopped) => Ok(()),
		result = transport => result,
	);
	let _ = runner.inbox.send(Event::StopRequest).await;
	let _ = runner.snapshot.wait_for(|view| view.stopped).await;
	replies.close();
	executor.shutdown().await;
	drop(runner.inbox);
	runner.stopped.await.context("JWT key-ring runner failed")?;
	result
}

struct Snapshot {
	ring: SigningKeyRing,
	etag: String,
	read_boot_ms: u64,
	refresh_boot_ms: u64,
}
#[derive(Default)]
struct CacheState {
	snapshot: Option<Snapshot>,
	last_attempt_boot_ms: Option<u64>,
	last_unknown_boot_ms: Option<u64>,
}
/// Shared by EngineContext clones. Construction performs no I/O or task spawning.
/// Concurrent misses coalesce under this mutex; successful verification needs only memory.
pub struct Cache {
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	config: Config,
	state: Mutex<CacheState>,
}
impl Cache {
	pub fn new(env: Arc<dyn Env>, store: Arc<dyn ObjectStore>, config: Config) -> Self {
		Self {
			env,
			store,
			config,
			state: Mutex::new(CacheState::default()),
		}
	}
	async fn ring(&self, signing: bool, kid: Option<KeyId>) -> Result<SigningKeyRing> {
		let jwt = &self.config.auth_required()?.jwt;
		ensure!(jwt.enabled(), "JWT authentication disabled");
		let mut state = self.state.lock().await;
		let now = self.env.clock().observe()?;
		let max_age = if signing {
			SIGNING_CACHE_LEASE.as_millis() as u64
		} else {
			jwt.verifier_max_stale().as_millis().min(300_000) as u64
		};
		let cooldown = jwt.unknown_kid_refresh_cooldown().as_millis() as u64;
		let unknown = kid.is_some_and(|kid| {
			state
				.snapshot
				.as_ref()
				.is_some_and(|s| !s.ring.all_public_keys().any(|key| key.kid == kid))
		});
		let unknown_due = unknown
			&& state
				.last_unknown_boot_ms
				.is_none_or(|last| now.boot_ms.saturating_sub(last) >= cooldown);
		let expired = state
			.snapshot
			.as_ref()
			.is_none_or(|s| now.boot_ms.saturating_sub(s.read_boot_ms) >= max_age);
		let refresh_due = !signing
			&& state
				.snapshot
				.as_ref()
				.is_some_and(|s| now.boot_ms >= s.refresh_boot_ms);
		// Coalesce failed reads too. A cold/expired cache fails closed throughout this backoff.
		let retry_due = state
			.last_attempt_boot_ms
			.is_none_or(|last| now.boot_ms.saturating_sub(last) >= 1_000);
		if (expired || refresh_due || unknown_due) && retry_due {
			state.last_attempt_boot_ms = Some(now.boot_ms);
			if unknown_due {
				state.last_unknown_boot_ms = Some(now.boot_ms);
			}
			let etag = state.snapshot.as_ref().map(|s| s.etag.clone());
			let fetched = self
				.store
				.get_opts(
					&SigningKeyRingKey::new().pack(),
					GetOptions {
						if_none_match: etag,
						..Default::default()
					},
				)
				.await;
			let result = match fetched {
				Err(object_store::Error::NotModified { .. }) => {
					let snapshot = state.snapshot.as_mut().context("missing cached JWT ring")?;
					snapshot.read_boot_ms = now.boot_ms;
					snapshot.refresh_boot_ms = refresh_deadline(&snapshot.ring, &self.config, now)?;
					Ok(())
				}
				Ok(result) => match crate::store::decode_result(result).await {
					Ok(stored) => {
						let ring = decode_signing_key_ring(&stored.bytes)?;
						ensure!(
							ring.issuer_state.active.issuer
								== rivet_config::config::auth::derive_issuer(&self.config)?
								&& ring.audience == jwt.audience(),
							"JWT ring does not belong to this cluster"
						);
						if let Some(previous) = &state.snapshot {
							ensure!(
								ring.generation >= previous.ring.generation,
								"JWT ring generation regressed"
							);
							ensure!(
								ring.generation != previous.ring.generation
									|| ring == previous.ring,
								"JWT ring changed without advancing its generation"
							);
						}
						let refresh_boot_ms = refresh_deadline(&ring, &self.config, now)?;
						state.snapshot = Some(Snapshot {
							ring,
							etag: stored.etag,
							read_boot_ms: now.boot_ms,
							refresh_boot_ms,
						});
						Ok(())
					}
					Err(error) => Err(error),
				},
				Err(error) => Err(error.into()),
			};
			if result.is_ok() {
				state.last_attempt_boot_ms = None;
			}
			if let Err(error) = result {
				// Neither an error nor an unknown-kid request renews the successful-read lease.
				if signing || state.snapshot.is_none() {
					return Err(error);
				}
			}
		}
		let snapshot = state.snapshot.as_ref().context("JWT ring unavailable")?;
		let age = self
			.env
			.clock()
			.boot_ms()?
			.saturating_sub(snapshot.read_boot_ms);
		ensure!(age < max_age, "JWT cache lease expired");
		Ok(snapshot.ring.clone())
	}
	pub async fn verify(&self, token: &str) -> Result<VerifiedJwt> {
		if !self.config.auth_required()?.jwt.enabled() {
			return Err(TokenError::InvalidClaims.into());
		}
		let kid = peek_header(token)?.kid;
		let ring = self
			.ring(false, Some(kid))
			.await
			.map_err(|_| rivet_auth_policy::errors::Auth::VerificationUnavailable.build())?;
		let now = self.env.clock().observe()?.unix_ms;
		let key = ring
			.all_public_keys()
			.find(|key| key.kid == kid)
			.ok_or(TokenError::InvalidKeyId)?;
		if ring
			.retiring
			.iter()
			.any(|key| key.key.kid == kid && now >= key.verify_until_ts)
		{
			return Err(TokenError::Expired.into());
		}
		let token = decode(
			token,
			&key.verification_key(),
			DecodeOptions {
				issuer: &ring.issuer_state.active.issuer,
				audience: &ring.audience,
				now: now.div_euclid(1000).try_into()?,
			},
		)?;
		crate::validation::validate_lifecycle(
			&token,
			&ring,
			kid,
			now.div_euclid(1000).try_into()?,
		)?;
		let authorization_deadline = token.claims.exp.saturating_add(PROTOCOL_CLOCK_SKEW);
		Ok(VerifiedJwt {
			token,
			kid,
			authorization_deadline,
		})
	}
	pub async fn issue(
		&self,
		namespace: String,
		grants: ValidatedGrantSet,
		subject: Option<String>,
		duration: u64,
	) -> Result<(String, i64, i64)> {
		let jwt = &self.config.auth_required()?.jwt;
		ensure!(
			jwt.issuance_enabled() && duration > 0 && duration <= jwt.max_duration().as_secs(),
			"JWT issuance disabled or invalid duration"
		);
		ensure!(
			grants.grants().iter().all(
				|grant| matches!(&grant.namespace, rivet_auth_policy::Scope::Id(ns) if ns == &namespace)
			),
			"JWT grants belong to another namespace"
		);
		let ring = self.ring(true, None).await?;
		let now = self.env.clock().observe()?.unix_ms;
		ensure!(
			now < ring.active.sign_until_ts,
			"JWT signing key exceeded its hard deadline"
		);
		let iat = u64::try_from(now.div_euclid(1000))?;
		let exp = iat
			.checked_add(duration)
			.context("JWT expiration overflow")?;
		let mut jti = [0; 16];
		rivet_env::backends::production::secure_random(&mut jti)?;
		let claims = Claims {
			rivet_ver: CLAIMS_VERSION,
			iss: ring.issuer_state.active.issuer,
			aud: ring.audience,
			sub: subject,
			iat,
			exp,
			jti: TokenId::from_bytes(jti).to_string(),
			rivet_ns: namespace,
			rivet_grants: encode_grants(&grants)?,
		};
		Ok((
			encode(&ring.active.key.signing_key, &claims)?,
			(iat * 1000).try_into()?,
			(exp * 1000).try_into()?,
		))
	}
}
/// A successful read leases the verifier for at most five minutes. Known lifecycle deadlines
/// bring a refresh forward, with a small margin for the singleton's conditional publication.
fn refresh_deadline(ring: &SigningKeyRing, config: &Config, now: rivet_env::Clocks) -> Result<u64> {
	let jwt = &config.auth_required()?.jwt;
	let lease = jwt.verifier_cache_ttl().as_millis().min(300_000) as u64;
	let lifecycle = next_wake_ts(ring, policy(config)?)?.saturating_add(1_000);
	let delay = if lifecycle > now.unix_ms {
		lease.min((lifecycle - now.unix_ms) as u64)
	} else {
		lease
	};
	Ok(now.boot_ms.saturating_add(delay))
}


