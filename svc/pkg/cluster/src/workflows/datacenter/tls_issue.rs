use acme_lib::{
	create_p384_key,
	persist::{MemoryPersist, Persist, PersistKey, PersistKind},
	Account, Directory, DirectoryUrl,
};
use cloudflare::endpoints as cf;

use chirp_workflow::prelude::*;
use futures_util::StreamExt;
use tokio::task;
use trust_dns_resolver::{
	config::{ResolverConfig, ResolverOpts},
	error::ResolveErrorKind,
	TokioAsyncResolver,
};

use crate::{
	types::TlsState,
	util::{cf_client, create_dns_record, delete_dns_record},
};

const ENCRYPT_EMAIL: &str = "letsencrypt@rivet.gg";

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub datacenter_id: Uuid,
	pub renew: bool,
}

#[workflow]
pub(crate) async fn cluster_datacenter_tls_issue(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	let datacenter_id = input.datacenter_id;

	let base_zone_id = unwrap!(
		util::env::cloudflare::zone::main::id(),
		"dns not configured"
	);
	let job_zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
	let domain_main = unwrap!(util::env::domain_main(), "dns not enabled");
	let domain_job = unwrap!(util::env::domain_job(), "dns not enabled");

	let (gg_cert, job_cert) = ctx
		.join((
			activity(OrderInput {
				renew: input.renew,
				zone_id: base_zone_id.to_string(),
				common_name: domain_main.to_string(),
				subject_alternative_names: vec![format!("*.{datacenter_id}.{domain_main}")],
			}),
			activity(OrderInput {
				renew: input.renew,
				zone_id: job_zone_id.to_string(),
				common_name: domain_job.to_string(),
				subject_alternative_names: vec![
					format!("*.lobby.{datacenter_id}.{domain_job}"),
					format!("*.{datacenter_id}.{domain_job}"),
				],
			}),
		))
		.await?;

	ctx.activity(InsertDbInput {
		datacenter_id: input.datacenter_id,
		gg_cert: gg_cert.cert,
		gg_private_key: gg_cert.private_key,
		job_cert: job_cert.cert,
		job_private_key: job_cert.private_key,
		expire_ts: gg_cert.expire_ts,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct OrderInput {
	renew: bool,
	zone_id: String,
	common_name: String,
	subject_alternative_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct OrderOutput {
	cert: String,
	private_key: String,
	expire_ts: i64,
}

#[activity(Order)]
#[timeout = 130]
async fn order(ctx: &ActivityCtx, input: &OrderInput) -> GlobalResult<OrderOutput> {
	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	let client = cf_client(Some(&cf_token)).await?;

	// Fetch ACME account registration
	let account = acme_account().await?;

	tracing::info!(cn=%input.common_name, "creating order");

	let mut order = task::block_in_place(|| {
		account.new_order(
			&input.common_name,
			&input
				.subject_alternative_names
				.iter()
				.map(|s| s.as_str())
				.collect::<Vec<_>>(),
		)
	})?;

	// When not renewing, if the ownership of the domain(s) have already been authorized in a previous order
	// we might be able to skip validation. The ACME API provider decides.
	let order_csr = if let Some(order_csr) =
		input.renew.then(|| order.confirm_validations()).flatten()
	{
		order_csr
	} else {
		let client = &client;
		let cf_token = &cf_token;

		loop {
			tracing::info!(cn=%input.common_name, "fetching authorizations");
			let auths = task::block_in_place(|| order.authorizations())?;

			// Run authorizations in parallel
			let results = futures_util::stream::iter(auths.into_iter().map(|auth| {
				async move {
					let challenge = auth.dns_challenge();
					let proof = challenge.dns_proof();

					let hostname = format!("_acme-challenge.{}", auth.api_auth().identifier.value);
					let dns_record_id = create_dns_record(
						client,
						cf_token,
						&input.zone_id,
						&hostname,
						cf::dns::DnsContent::TXT {
							content: proof.to_string(),
						},
					)
					.await?;

					let try_block = async {
						// Wait for DNS to propagate
						poll_txt_dns(&hostname, &proof).await?;

						tracing::info!(%hostname, "validating authorization");
						task::block_in_place(|| challenge.validate(5000))?;

						GlobalResult::Ok(())
					}
					.await;

					// Delete regardless of success of the above try block
					delete_dns_record(client, &input.zone_id, &dns_record_id).await?;

					try_block
				}
			}))
			.buffer_unordered(4)
			.collect::<Vec<_>>()
			.await;

			// Handle errors only after all futures have completed so that we ensure all dns records are deleted
			for res in results {
				res?;
			}

			tracing::info!("refreshing order");
			task::block_in_place(|| order.refresh())?;

			if let Some(order_csr) = order.confirm_validations() {
				break order_csr;
			}
		}
	};

	tracing::info!("order validated");

	// Submit the CSR
	let cert_pri = create_p384_key();
	let ord_cert = task::block_in_place(|| order_csr.finalize_pkey(cert_pri, 5000))?;
	let cert = task::block_in_place(|| ord_cert.download_and_save_cert())?;

	tracing::info!("order finalized");

	Ok(OrderOutput {
		cert: cert.certificate().to_string(),
		private_key: cert.private_key().to_string(),
		expire_ts: util::timestamp::now() + util::duration::days(cert.valid_days_left()),
	})
}

async fn acme_account() -> GlobalResult<Account<MemoryPersist>> {
	let url = match util::env::var("TLS_ACME_DIRECTORY")?.as_str() {
		"lets_encrypt" => DirectoryUrl::LetsEncrypt,
		"lets_encrypt_staging" => DirectoryUrl::LetsEncryptStaging,
		x => bail!(format!("unknown ACME directory: {x}")),
	};

	tracing::info!("fetching account from directory {:?}", url);

	let persist = MemoryPersist::new();

	// Write account private key (from terraform) to persistence
	let pem_key = PersistKey::new(
		ENCRYPT_EMAIL,
		PersistKind::AccountPrivateKey,
		"acme_account",
	);
	let pem = util::env::var("TLS_ACME_ACCOUNT_PRIVATE_KEY_PEM")?;
	persist.put(&pem_key, pem.as_bytes())?;

	// Get ACME account info
	let acc = tokio::task::spawn_blocking(move || {
		// Initialize ACME directory
		let dir = Directory::from_url(persist, url)?;

		tracing::info!("fetching account");
		dir.account(ENCRYPT_EMAIL)
	})
	.await??;

	Ok(acc)
}

async fn poll_txt_dns(hostname: &str, content: &str) -> GlobalResult<()> {
	// Because the dns resolver has its own internal cache, we create a new one for each poll function call
	// so that clearing cache does not affect other concurrent txt lookup calls
	let dns_resolver =
		TokioAsyncResolver::tokio(ResolverConfig::cloudflare_tls(), ResolverOpts::default());

	// Fully qualified domain name lookups are faster
	let fqdn = format!("{hostname}.");

	// Retry DNS until the TXT record shows up
	for attempt in 1..=60 {
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;

		tracing::info!(%attempt, %fqdn, "attempting to resolve dns");

		dns_resolver.clear_cache();

		match dns_resolver.txt_lookup(&fqdn).await {
			Ok(res) => {
				if res.iter().any(|record| record.to_string() == content) {
					return Ok(());
				}
			}
			// Retry
			Err(err) if matches!(err.kind(), ResolveErrorKind::NoRecordsFound { .. }) => {}
			Err(err) => return Err(err.into()),
		}
	}

	bail!("dns not resolved");
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
	datacenter_id: Uuid,
	gg_cert: String,
	gg_private_key: String,
	job_cert: String,
	job_private_key: String,
	expire_ts: i64,
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.datacenter_tls
		SET
			gg_cert_pem = $2,
			gg_private_key_pem = $3,
			job_cert_pem = $4,
			job_private_key_pem = $5,
			state = $6,
			expire_ts = $7
		WHERE datacenter_id = $1
		",
		input.datacenter_id,
		&input.gg_cert,
		&input.gg_private_key,
		&input.job_cert,
		&input.job_private_key,
		TlsState::Active as i32,
		input.expire_ts,
	)
	.await?;

	Ok(())
}
