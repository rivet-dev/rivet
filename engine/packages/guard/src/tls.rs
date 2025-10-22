use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::CertResolverFn;

// /// Certificate pair with name for logging
// struct CertificatePair {
// 	name: &'static str,
// 	cert_path: Box<Path>,
// 	key_path: Box<Path>,
// }

// /// Helper function to load a certificate and key into a CertifiedKey
// fn load_certified_key(cert_pair: &CertificatePair) -> GlobalResult<Arc<CertifiedKey>> {
// 	// Validate that paths exist
// 	if !cert_pair.cert_path.exists() {
// 		bail!(
// 			"{} certificate file not found at {:?}",
// 			cert_pair.name,
// 			cert_pair.cert_path
// 		);
// 	}
// 	if !cert_pair.key_path.exists() {
// 		bail!(
// 			"{} key file not found at {:?}",
// 			cert_pair.name,
// 			cert_pair.key_path
// 		);
// 	}
//
// 	tracing::debug!("Loading {} certificate from:", cert_pair.name);
// 	tracing::debug!("  Cert: {:?}", cert_pair.cert_path);
// 	tracing::debug!("  Key: {:?}", cert_pair.key_path);
//
// 	// Load certificate
// 	let cert_file = match File::open(&cert_pair.cert_path) {
// 		Ok(file) => file,
// 		Err(e) => bail!("Failed to open {} certificate file: {}", cert_pair.name, e),
// 	};
// 	let cert_reader = &mut BufReader::new(cert_file);
//
// 	let cert_chain = match certs(cert_reader).collect::<Result<Vec<_>, _>>() {
// 		Ok(chain) => chain,
// 		Err(e) => bail!("Failed to parse {} certificate: {}", cert_pair.name, e),
// 	};
//
// 	if cert_chain.is_empty() {
// 		bail!(
// 			"No certificates found in {} certificate file",
// 			cert_pair.name
// 		);
// 	}
//
// 	// Load private key
// 	let key_file = match File::open(&cert_pair.key_path) {
// 		Ok(file) => file,
// 		Err(e) => bail!("Failed to open {} key file: {}", cert_pair.name, e),
// 	};
// 	let key_reader = &mut BufReader::new(key_file);
//
// 	let key_der = match private_key(key_reader) {
// 		Ok(Some(key)) => key,
// 		Ok(None) => bail!("No private key found in {} key file", cert_pair.name),
// 		Err(e) => bail!("Failed to parse {} key: {}", cert_pair.name, e),
// 	};
//
// 	let signing_key = match any_supported_type(&key_der) {
// 		Ok(key) => key,
// 		Err(e) => bail!("Failed to load {} signing key: {}", cert_pair.name, e),
// 	};
//
// 	tracing::info!("{} certificate loaded successfully", cert_pair.name);
// 	Ok(Arc::new(CertifiedKey::new(cert_chain, signing_key)))
// }

/// Create a certificate resolver function for TLS
///
/// This function sets up a certificate resolver that will serve:
/// - Actor certificate for hostnames that match the actor routing logic
/// - API certificate for all other hostnames
///
/// It follows the same routing logic as the main routing function to ensure
/// consistent behavior between routing and certificate selection.
#[tracing::instrument(skip_all)]
pub async fn create_cert_resolver(
	_ctx: &gas::prelude::StandaloneCtx,
) -> Result<Option<CertResolverFn>> {
	return Ok(None);

	// // Get the Guard configuration
	// let guard_config = match ctx.config().guard() {
	// 	Ok(config) => config,
	// 	Err(e) => {
	// 		tracing::warn!("Failed to get Guard configuration: {}", e);
	// 		return Ok(None);
	// 	}
	// };
	//
	// // If HTTPS is not configured, return None
	// let https_config = match &guard_config.https {
	// 	Some(config) => config,
	// 	None => {
	// 		tracing::info!("HTTPS configuration not found in Guard config - TLS disabled");
	// 		return Ok(None);
	// 	}
	// };
	//
	// // TLS config is directly in the HTTPS config section
	// let tls_config = &https_config.tls;
	//
	// // Load certificates
	// let api_cert = load_certified_key(&CertificatePair {
	// 	name: "API",
	// 	cert_path: Path::new(&tls_config.api_cert_path).into(),
	// 	key_path: Path::new(&tls_config.api_key_path).into(),
	// })?;
	//
	// // Load actor certificate
	// let actor_cert = load_certified_key(&CertificatePair {
	// 	name: "Actor",
	// 	cert_path: Path::new(&tls_config.actor_cert_path).into(),
	// 	key_path: Path::new(&tls_config.actor_key_path).into(),
	// })?;
	//
	// // Get the datacenter ID from config
	// let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	//
	// // Get datacenter information to get the guard public hostname
	// let dc_res = ctx
	// 	.op(cluster::ops::datacenter::get::Input {
	// 		datacenter_ids: vec![dc_id],
	// 	})
	// 	.await?;
	//
	// let dc = unwrap!(dc_res.datacenters.first());
	// let guard_hostname = &dc.guard_public_hostname;
	// let api_hostname = ctx
	// 	.config()
	// 	.server()?
	// 	.rivet
	// 	.edge_api_routing_host(&dc.name_id)?;
	//
	// tracing::info!("Using datacenter guard hostname: {:?}", guard_hostname);
	// if let Some(api_host) = &api_hostname {
	// 	tracing::info!("Using datacenter API hostname: {:?}", api_host);
	// }
	//
	// // Get the hostname regexes for actor routing with hostname-based endpoint type
	// let actor_hostname_regex_dynamic =
	// 	match build_actor_hostname_and_path_regex(EndpointType::Hostname, guard_hostname) {
	// 		Ok(Some((x, _))) => {
	// 			tracing::info!("Successfully built dynamic hostname actor routing regex");
	// 			Some(x)
	// 		}
	// 		Ok(None) => {
	// 			tracing::warn!(
	// 				"Could not build dynamic hostname actor routing regex - pattern will be skipped"
	// 			);
	// 			None
	// 		}
	// 		Err(e) => bail!(
	// 			"Failed to build dynamic hostname actor routing regex: {}",
	// 			e
	// 		),
	// 	};
	// let actor_hostname_regex_static =
	// 	match build_actor_hostname_and_path_regex(EndpointType::Path, guard_hostname) {
	// 		Ok(Some((x, _))) => {
	// 			tracing::info!("Successfully built static path actor routing regex");
	// 			Some(x)
	// 		}
	// 		Ok(None) => {
	// 			tracing::warn!(
	// 				"Could not build static path actor routing regex - pattern will be skipped"
	// 			);
	// 			None
	// 		}
	// 		Err(e) => bail!("Failed to build static path actor routing regex: {}", e),
	// 	};
	//
	// // Create resolver function that matches the routing logic
	// let api_cert_clone = api_cert.clone();
	// let actor_cert_clone = actor_cert.clone();
	// let api_hostname_clone = api_hostname.clone();
	//
	// let resolver_fn: CertResolverFn = Arc::new(
	// 	move |hostname: &str| -> Result<Arc<CertifiedKey>, Box<dyn Error + Send + Sync>> {
	// 		// Extract just the host, stripping the port if present
	// 		let host = hostname.split(':').next().unwrap_or(hostname);
	//
	// 		// First check if hostname matches the actor pattern
	// 		// This follows the same routing precedence as in routing/mod.rs
	// 		if let Some(x) = &actor_hostname_regex_dynamic {
	// 			if x.is_match(host) {
	// 				tracing::debug!(
	// 					"Using dynamic hostname actor certificate for hostname: {}",
	// 					host
	// 				);
	// 				return Ok(actor_cert_clone.clone());
	// 			}
	// 		}
	// 		if let Some(x) = &actor_hostname_regex_static {
	// 			if x.is_match(host) {
	// 				tracing::debug!(
	// 					"Using static hostname actor certificate for hostname: {}",
	// 					host
	// 				);
	// 				return Ok(actor_cert_clone.clone());
	// 			}
	// 		}
	//
	// 		// Then check if it matches the API hostname
	// 		if let Some(api_host) = &api_hostname_clone {
	// 			if host == api_host {
	// 				tracing::debug!("Using API certificate for API hostname: {}", host);
	// 				return Ok(api_cert_clone.clone());
	// 			}
	// 		}
	//
	// 		bail!("Did not match any routes")
	// 	},
	// );
	//
	// Ok(Some(resolver_fn))
}
