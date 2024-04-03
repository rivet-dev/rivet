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
	std::env::var("RIVET_HAS_POOLS").ok().is_some()
}
