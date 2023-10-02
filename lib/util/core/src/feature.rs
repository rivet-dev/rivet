pub fn cf_custom_hostname() -> bool {
	crate::env::cloudflare::zone::game::id().is_some()
}

pub fn job_run() -> bool {
	false
}
