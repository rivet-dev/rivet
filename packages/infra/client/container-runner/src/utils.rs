use anyhow::*;

pub enum ActorOwner {
	DynamicServer { server_id: String },
}

pub fn var(name: &str) -> Result<String> {
	std::env::var(name).context(name.to_string())
}
