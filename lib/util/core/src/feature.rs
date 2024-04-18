pub fn cf_custom_hostname() -> bool {
	crate::env::cloudflare::zone::game::id().is_some()
}

pub fn hcaptcha() -> bool {
	std::env::var(crate::env::secret_env_var_key(&["hcaptcha", "secret"]))
		.ok()
		.is_some()
}

pub fn dns() -> bool {
	crate::env::domain_cdn().is_some()
}

pub fn fly() -> bool {
	std::env::var("FLY_ORGANIZATION_ID")
		.ok()
		.and(std::env::var("FLY_REGION").ok())
		.is_some()
}

pub fn billing() -> bool {
	std::env::var("RIVET_BILLING").ok().is_some()
}

pub fn job_run() -> bool {
	server_provision()
}

pub fn server_provision() -> bool {
	std::env::var("RIVET_DEFAULT_CLUSTER_CONFIG").ok().is_some()
}

pub fn email() -> bool {
	std::env::var("SENDGRID_KEY").ok().is_some()
}
