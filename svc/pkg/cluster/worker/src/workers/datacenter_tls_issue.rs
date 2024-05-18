use acme_lib::{
	create_p384_key,
	persist::{MemoryPersist, Persist, PersistKey, PersistKind},
	Account, Certificate, Directory, DirectoryUrl,
};
use chirp_worker::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework, framework::async_api::ApiClient};
use futures_util::StreamExt;
use proto::backend::{self, pkg::*};
use tokio::task;
use trust_dns_resolver::{
	config::{ResolverConfig, ResolverOpts},
	error::ResolveErrorKind,
	TokioAsyncResolver,
};

use crate::util::CloudflareError;

const ENCRYPT_EMAIL: &str = "letsencrypt@rivet.gg";

#[worker(name = "cluster-datacenter-tls-issue", timeout = 300)]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_tls_issue::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	// Create CF client
	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	let client = cf_framework::async_api::Client::new(
		cf_framework::auth::Credentials::UserAuthToken { token: cf_token },
		Default::default(),
		cf_framework::Environment::Production,
	)
	.map_err(CloudflareError::from)?;

	// Fetch ACME account registration
	let account = acme_account().await?;

	let base_zone_id = unwrap!(
		util::env::cloudflare::zone::main::id(),
		"dns not configured"
	);
	let job_zone_id = unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
	let domain_main = unwrap!(util::env::domain_main(), "dns not enabled");
	let domain_job = unwrap!(util::env::domain_job(), "dns not enabled");

	// NOTE: We don't use try_join because these run in parallel, the dns record needs to be deleted for each
	// order upon failure
	let (gg_cert, job_cert) = tokio::join!(
		order(
			&client,
			ctx.renew,
			base_zone_id,
			&account,
			domain_main,
			vec![format!("*.{datacenter_id}.{domain_main}")],
		),
		order(
			&client,
			ctx.renew,
			job_zone_id,
			&account,
			domain_job,
			vec![
				format!("*.lobby.{datacenter_id}.{domain_job}"),
				format!("*.{datacenter_id}.{domain_job}"),
			],
		),
	);
	let (gg_cert, job_cert) = (gg_cert?, job_cert?);

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
		datacenter_id,
		gg_cert.certificate(),
		gg_cert.private_key(),
		job_cert.certificate(),
		job_cert.private_key(),
		backend::cluster::TlsState::Active as i64,
		util::timestamp::now() + util::duration::days(gg_cert.valid_days_left()),
	)
	.await?;

	Ok(())
}

async fn acme_account() -> GlobalResult<Account<MemoryPersist>> {
	let url = match util::env::var("TLS_ACME_DIRECTORY")?.as_str() {
		"lets_encrypt" => DirectoryUrl::LetsEncrypt,
		"lets_encrypt_staging" => DirectoryUrl::LetsEncryptStaging,
		x => bail!(format!("unknown ACME directory: {x}")),
	};

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

// TODO: This function contains both blocking calls that cannot be shared between threads and async calls.
// Maybe theres a way to defer the blocking calls somehow
async fn order<P: Persist>(
	client: &cf_framework::async_api::Client,
	renew: bool,
	zone_id: &str,
	account: &Account<P>,
	common_name: &str,
	subject_alternative_names: Vec<String>,
) -> GlobalResult<Certificate> {
	tracing::info!(cn=%common_name, "creating order");

	let mut order = task::block_in_place(|| {
		account.new_order(
			common_name,
			&subject_alternative_names
				.iter()
				.map(|s| s.as_str())
				.collect::<Vec<_>>(),
		)
	})?;

	// When not renewing, if the ownership of the domain(s) have already been authorized in a previous order
	// we might be able to skip validation. The ACME API provider decides.
	let order_csr = if let Some(order_csr) = renew.then(|| order.confirm_validations()).flatten() {
		order_csr
	} else {
		loop {
			tracing::info!(%common_name, "fetching authorizations");
			let auths = task::block_in_place(|| order.authorizations())?;

			// Run authorizations in parallel
			let results = futures_util::stream::iter(auths.into_iter().map(|auth| {
				async move {
					let challenge = auth.dns_challenge();
					let proof = challenge.dns_proof();

					let hostname = format!("_acme-challenge.{}", auth.api_auth().identifier.value);
					let dns_record_id =
						create_dns_record(client, zone_id, &hostname, &proof).await?;

					let try_block = async {
						// Wait for DNS to propagate
						poll_txt_dns(&hostname, &proof).await?;

						tracing::info!(%hostname, "validating authorization");
						task::block_in_place(|| challenge.validate(5000))?;

						GlobalResult::Ok(())
					}
					.await;

					// Delete regardless of success of the above try block
					delete_dns_record(client, zone_id, &dns_record_id).await?;

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

	Ok(cert)
}

async fn create_dns_record(
	client: &cf_framework::async_api::Client,
	zone_id: &str,
	record_name: &str,
	content: &str,
) -> GlobalResult<String> {
	tracing::info!(%record_name, "creating dns record");

	let create_record_res = client
		.request(&cf::dns::CreateDnsRecord {
			zone_identifier: zone_id,
			params: cf::dns::CreateDnsRecordParams {
				name: record_name,
				content: cf::dns::DnsContent::TXT {
					content: content.to_string(),
				},
				proxied: Some(false),
				ttl: Some(60),
				priority: None,
			},
		})
		.await?;

	Ok(create_record_res.result.id)
}

async fn delete_dns_record(
	client: &cf_framework::async_api::Client,
	zone_id: &str,
	record_id: &str,
) -> GlobalResult<()> {
	tracing::info!(%record_id, "deleting dns record");

	client
		.request(&cf::dns::DeleteDnsRecord {
			zone_identifier: zone_id,
			identifier: record_id,
		})
		.await?;

	Ok(())
}

async fn poll_txt_dns(hostname: &str, content: &str) -> GlobalResult<()> {
	// Because the dns resolver has its own internal cache, we create a new one for each poll function call
	// so that clearing cache does not affect other concurrent txt lookup calls
	let dns_resolver =
		TokioAsyncResolver::tokio(ResolverConfig::cloudflare_tls(), ResolverOpts::default());

	// Fully qualified domain name lookups are faster
	let fqdn = format!("{hostname}.");

	// Retry DNS until the TXT record shows up
	for attempt in 1..=100 {
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
