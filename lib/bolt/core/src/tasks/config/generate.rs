use anyhow::*;
use duct::cmd;
use rand::{distributions::Alphanumeric, Rng};
use std::{
	future::Future,
	path::{Path, PathBuf},
};
use tokio::fs;
use tokio::task::block_in_place;
use toml_edit::value;
use uuid::Uuid;

use crate::{
	config::{self, service::RuntimeKind},
	context::ProjectContextData,
	utils,
};

/// Comment attached to the head of the namespace config.
const NS_CONFIG_COMMENT: &str = r#"# Documentation: doc/bolt/config/NAMESPACE.md
# Schema: lib/bolt/config/src/ns.rs

"#;

/// Helper for generating configs.
struct ConfigGenerator {
	term: rivet_term::console::Term,

	#[allow(unused)]
	ns_id: String,

	ns_path: PathBuf,
	ns: toml_edit::Document,

	secrets_path: PathBuf,
	secrets: toml_edit::Document,

	/// If true, this is a new config. If false, this is editing an existing
	/// config.
	is_new: bool,
}

impl ConfigGenerator {
	async fn new(
		term: rivet_term::console::Term,
		project_path: &Path,
		ns_id: impl ToString,
	) -> Result<Self> {
		let ns_id = ns_id.to_string();

		// Load namespace config
		let ns_path = project_path
			.join("namespaces")
			.join(format!("{ns_id}.toml"));
		let (ns, is_new) = if ns_path.exists() {
			let ns_str = fs::read_to_string(&ns_path).await?;
			(ns_str.parse::<toml_edit::Document>()?, false)
		} else {
			(toml_edit::Document::new(), true)
		};

		// Load secrets config
		let secrets_path = project_path.join("secrets").join(format!("{ns_id}.toml"));
		let secrets = if secrets_path.exists() {
			let secrets_str = fs::read_to_string(&secrets_path).await?;
			secrets_str.parse::<toml_edit::Document>()?
		} else {
			toml_edit::Document::new()
		};

		Ok(Self {
			term,
			ns_id,
			ns_path,
			ns,
			secrets_path,
			secrets,
			is_new,
		})
	}

	// Writes the config to the respective files.
	async fn write(&mut self) -> Result<()> {
		// Prepend comment
		let mut ns_str = self.ns.to_string();
		if self.is_new {
			ns_str = format!("{NS_CONFIG_COMMENT}{ns_str}");
		}

		// Write configs
		fs::write(&self.ns_path, ns_str.as_bytes()).await?;
		fs::write(&self.secrets_path, self.secrets.to_string().as_bytes()).await?;

		Ok(())
	}

	/// Moves a value's location. Useful for updating schemas.
	// async fn move_config(&mut self, from_path: &[&str], to_path: &[&str]) -> Result<()> {
	// 	// Read the parent
	// 	if let Some(parent_value) =
	// 		get_value_mut(self.ns.as_item_mut(), &from_path[0..(from_path.len() - 1)])
	// 			.and_then(|x| x.as_table_mut())
	// 	{
	// 		// Remove the item
	// 		if let Some(value) = parent_value.remove(from_path.last().unwrap()) {
	// 			// Write to the new path
	// 			write_value(self.ns.as_item_mut(), to_path, value);
	// 		}
	// 	}

	// 	Ok(())
	// }

	/// Inserts a config value if does not exist.
	async fn generate_config<Fut>(
		&mut self,
		path: &[&str],
		value_fn: impl FnOnce() -> Fut,
	) -> Result<()>
	where
		Fut: Future<Output = Result<toml_edit::Item>>,
	{
		// Check if item already exists
		if get_value(self.ns.as_item(), path).is_none() {
			let value = value_fn().await?;
			write_value(self.ns.as_item_mut(), path, value);
		}

		Ok(())
	}

	/// Prompts user for config value if does not exist.
	// async fn prompt_config(&mut self, message: &str, docs: &str, path: &[&str]) -> Result<()> {
	// 	// Check if item already exists
	// 	if get_value(self.ns.as_item(), path).is_none() {
	// 		let x = rivet_term::prompt::PromptBuilder::default()
	// 			.message(message)
	// 			.docs_url(docs)
	// 			.build()?
	// 			.string(&self.term)
	// 			.await?;
	// 		write_value(self.ns.as_item_mut(), path, value(x));
	// 	}

	// 	Ok(())
	// }

	/// Inserts a secret value if does not exist.
	async fn generate_secret<Fut>(
		&mut self,
		path: &[&str],
		value_fn: impl FnOnce() -> Fut,
	) -> Result<()>
	where
		Fut: Future<Output = Result<toml_edit::Item>>,
	{
		// Check if item already exists
		if get_value(self.secrets.as_item(), path).is_none() {
			let value = value_fn().await?;
			write_value(self.secrets.as_item_mut(), path, value);
		}

		Ok(())
	}

	/// Prompts user for config value if does not exist.
	async fn prompt_secret(&mut self, message: &str, docs: &str, path: &[&str]) -> Result<()> {
		self.prompt_secret_multiple(message, docs, &[path]).await
	}

	/// Prompts user for multiple config values if does not exist. Useful when
	/// there's multiple secrets that are OK being set to the same value for the
	/// default config.
	async fn prompt_secret_multiple(
		&mut self,
		message: &str,
		docs: &str,
		paths: &[&[&str]],
	) -> Result<()> {
		// Check if item already exists
		if !paths
			.iter()
			.all(|x| get_value(self.secrets.as_item(), x).is_some())
		{
			let x = rivet_term::prompt::PromptBuilder::default()
				.message(message)
				.docs_url(docs)
				.build()?
				.string_secure(&self.term)
				.await?;
			for path in paths {
				write_value(self.secrets.as_item_mut(), path, value(&x));
			}
		}

		Ok(())
	}
}

/// Generates a new config & secrets based on user input.
pub async fn generate(project_path: &Path, ns_id: &str) -> Result<()> {
	let term = rivet_term::terminal();

	let mut generator = ConfigGenerator::new(term, &project_path, ns_id).await?;

	// MARK: Cluster
	generator
		.generate_config(&["cluster", "id"], || async {
			Ok(value(Uuid::new_v4().to_string()).into())
		})
		.await?;

	if generator
		.ns
		.get("cluster")
		.unwrap()
		.get("single_node")
		.is_none()
		&& generator
			.ns
			.get("cluster")
			.unwrap()
			.get("distributed")
			.is_none()
	{
		generator
			.generate_config(&["cluster", "single_node", "public_ip"], || async {
				let public_ip = fetch_public_ip().await?;
				Ok(value(public_ip).into())
			})
			.await?;

		// Default to port 8080 since default port 80 is not suitable for most dev environments
		generator
			.generate_config(&["cluster", "single_node", "api_http_port"], || async {
				Ok(value(8080).into())
			})
			.await?;
	}

	// TODO: Prompt for provisioning servers
	// // MARK: Linode
	// generator
	// 	.prompt_secret(
	// 		"Linode Token",
	// 		"doc/bolt/config/LINODE.md",
	// 		&["linode", "terraform", "token"],
	// 	)
	// 	.await?;

	// // MARK: Pools
	// if generator.ns.get("pools").is_none() {
	// 	let mut pools = toml_edit::ArrayOfTables::new();

	// 	for (i, name_id) in ["lnd-sfo", "lnd-fra"].iter().enumerate() {
	// 		let base_netnum = i as i64 * 3;

	// 		let mut job = toml_edit::Table::new();
	// 		job["pool"] = value("job");
	// 		job["version"] = value("01");
	// 		job["region"] = value(*name_id);
	// 		job["count"] = value(1);
	// 		job["size"] = value("g6-standard-1");
	// 		job["netnum"] = value(base_netnum + 1);
	// 		pools.push(job);

	// 		let mut gg = toml_edit::Table::new();
	// 		gg["pool"] = value("gg");
	// 		gg["version"] = value("01");
	// 		gg["region"] = value(*name_id);
	// 		gg["count"] = value(1);
	// 		gg["size"] = value("g6-standard-1");
	// 		gg["netnum"] = value(base_netnum + 2);
	// 		pools.push(gg);

	// 		let mut ats = toml_edit::Table::new();
	// 		ats["pool"] = value("ats");
	// 		ats["version"] = value("01");
	// 		ats["region"] = value(*name_id);
	// 		ats["count"] = value(1);
	// 		ats["size"] = value("g6-standard-1");
	// 		ats["netnum"] = value(base_netnum + 3);
	// 		pools.push(ats);
	// 	}

	// 	generator.ns["pools"] = toml_edit::Item::ArrayOfTables(pools);
	// }

	// TODO: Prompt for DNS
	// // MARK: DNS
	// generator
	// 	.prompt_config(
	// 		"Domain Main",
	// 		"doc/bolt/config/DNS.md",
	// 		&["dns", "domain", "main"],
	// 	)
	// 	.await?;
	// generator
	// 	.prompt_config(
	// 		"Domain CDN",
	// 		"doc/bolt/config/DNS.md",
	// 		&["dns", "domain", "cdn"],
	// 	)
	// 	.await?;
	// generator
	// 	.prompt_config(
	// 		"Domain Job",
	// 		"doc/bolt/config/DNS.md",
	// 		&["dns", "domain", "job"],
	// 	)
	// 	.await?;

	// // MARK: Cloudflare
	// generator
	// 	.prompt_config(
	// 		"Cloudflare Account ID",
	// 		"doc/bolt/config/CLOUDFLARE.md",
	// 		&["dns", "cloudflare", "account_id"],
	// 	)
	// 	.await?;
	// generator
	// 	.prompt_secret_multiple(
	// 		"Cloudflare Auth Token",
	// 		"doc/bolt/config/CLOUDFLARE.md",
	// 		&[
	// 			// Permissions:
	// 			// - Zone > DNS > Edit
	// 			//
	// 			// Zone Resources:
	// 			// - rivet.run
	// 			&["cloudflare", "persistent", "auth_token"],
	// 			// Permissions:
	// 			// - Account > Cloudflare Tunnel > Edit (if using access)
	// 			// - Account > Access: Apps and Policies > Edit (if using access)
	// 			// - Account > Worker Scripts > Edit
	// 			// - Zone > Workers Routes > Edit
	// 			// - Zone > SSL and Certificates > Edit
	// 			// - Zone > DNS > Edit
	// 			//
	// 			// Zone Resources:
	// 			// - rivet.gg
	// 			// - rivet.game
	// 			// - rivet.run
	// 			&["cloudflare", "terraform", "auth_token"],
	// 		],
	// 	)
	// 	.await?;

	// MARK: S3
	if generator.ns.get("s3").is_none() {
		generator.ns["s3"] = {
			let mut x = toml_edit::Table::new();
			x.set_implicit(true);
			x["minio"] = toml_edit::table();
			toml_edit::Item::Table(x)
		};
	}

	// MARK: SendGrid
	if generator.ns.get("email").is_none() {
		generator.ns["email"] = {
			let mut x = toml_edit::Table::new();
			x.set_implicit(true);
			x["sendgrid"] = toml_edit::table();
			toml_edit::Item::Table(x)
		};
	}

	// TODO: Prompt login
	if generator
		.ns
		.get("email")
		.and_then(|x| x.get("sendgrid"))
		.is_some()
	{
		generator
			.prompt_secret(
				"SendGrid Key",
				"doc/bolt/config/SENDGRID.md",
				&["sendgrid", "key"],
			)
			.await?;
	}

	// MARK: SSH
	generator
		.generate_secret(&["ssh", "server", "private_key_openssh"], || async {
			let key = generate_private_key_openssh().await?;
			Ok(value(key))
		})
		.await?;

	// MARK: JWT
	if generator.secrets.get("jwt").is_none() {
		let mut table = toml_edit::Table::new();
		table.set_implicit(true);
		generator.secrets["jwt"] = toml_edit::Item::Table(table);
	}
	if generator.secrets["jwt"].get("key").is_none() {
		let key = generate_jwt_key().await?;

		let mut table = toml_edit::table();
		table["public_pem"] = value(key.public_pem);
		table["private_pem"] = value(key.private_pem);
		generator.secrets["jwt"]["key"] = table;
	}

	// MARK: Rivet
	generator
		.generate_secret(&["rivet", "api_admin", "token"], || async {
			Ok(value(generate_password(32)))
		})
		.await?;
	generator
		.generate_secret(&["rivet", "api_route", "token"], || async {
			Ok(value(generate_password(32)))
		})
		.await?;
	generator
		.generate_secret(&["rivet", "api_status", "token"], || async {
			Ok(value(generate_password(32)))
		})
		.await?;

	// MARK: ClickHouse
	for user in ["default", "bolt", "chirp", "grafana", "vector"] {
		generator
			.generate_secret(&["clickhouse", "users", user, "password"], || async {
				Ok(value(generate_clickhouse_password(32)))
			})
			.await?;
	}

	// MARK: Minio
	if generator.ns["s3"].get("minio").is_some() {
		let root_pass = generate_password(32);

		generator
			.generate_secret(&["s3", "minio", "root", "key_id"], || async {
				Ok(value("root"))
			})
			.await?;
		generator
			.generate_secret(&["s3", "minio", "root", "key"], {
				let root_pass = root_pass.clone();
				|| async { Ok(value(root_pass)) }
			})
			.await?;
		generator
			.generate_secret(&["s3", "minio", "terraform", "key_id"], || async {
				Ok(value("root"))
			})
			.await?;
		generator
			.generate_secret(&["s3", "minio", "terraform", "key"], || async {
				Ok(value(root_pass))
			})
			.await?;
	}

	// HACK: Write config and create new context so we can generate remaining
	// secrets that depend on service configurations
	generator.write().await?;
	let ctx = ProjectContextData::new(Some(ns_id.to_string())).await;

	// MARK: Redis
	for svc in ctx.all_services().await {
		let RuntimeKind::Redis { persistent } = svc.config().runtime else {
			continue;
		};

		let db_name = if persistent {
			"persistent"
		} else {
			"ephemeral"
		};
		let (db_name, username) = match &ctx.ns().redis.provider {
			config::ns::RedisProvider::Kubernetes {} => {
				(db_name.to_string(), "default".to_string())
			}
			config::ns::RedisProvider::Aws {} => {
				let db_name = format!("rivet-{}-{}", ctx.ns_id(), db_name);
				let username = format!("{db_name}-root");

				(db_name, username)
			}
		};

		generator
			.generate_secret(&["redis", &db_name, "username"], || async {
				Ok(value(username))
			})
			.await?;
		generator
			.generate_secret(&["redis", &db_name, "password"], || async {
				Ok(value(generate_password(32)))
			})
			.await?;
	}

	// MARK: CRDB
	generator
		.generate_secret(&["crdb", "username"], || async { Ok(value("rivet_root")) })
		.await?;
	generator
		.generate_secret(&["crdb", "password"], || async {
			Ok(value(generate_password(32)))
		})
		.await?;

	// Write configs again with new secrets
	generator.write().await?;

	let mut event = utils::telemetry::build_event(&ctx, "bolt_config_generate").await?;
	event.insert_prop("ns_id", ns_id)?;
	utils::telemetry::capture_event(&ctx, event).await?;

	eprintln!();
	rivet_term::status::success(
		"Generated config",
		&format!("namespaces/{ns_id}.toml & secrets/{ns_id}.toml"),
	);

	Ok(())
}

/// Returns a value at a given path.
fn get_value<'a>(mut item: &'a toml_edit::Item, path: &[&str]) -> Option<&'a toml_edit::Item> {
	for key in path {
		if let Some(x) = item.get(key).filter(|x| !x.is_none()) {
			item = x;
		} else {
			return None;
		}
	}

	Some(item)
}

/// Returns a mutable value at a given path.
// fn get_value_mut<'a>(
// 	mut item: &'a mut toml_edit::Item,
// 	path: &[&str],
// ) -> Option<&'a mut toml_edit::Item> {
// 	for key in path {
// 		if let Some(x) = item.get_mut(key).filter(|x| !x.is_none()) {
// 			item = x;
// 		} else {
// 			return None;
// 		}
// 	}

// 	Some(item)
// }

/// Writes a value to a path in a TOML item.
fn write_value(item: &mut toml_edit::Item, path: &[&str], value: toml_edit::Item) {
	if path.is_empty() {
		panic!("empty path");
	} else if path.len() == 1 {
		item[path[0]] = value;
	} else {
		let key = path[0];
		let sub_path = &path[1..];
		if let Some(x) = item.get_mut(key).filter(|x| !x.is_none()) {
			write_value(x, sub_path, value);
		} else {
			let mut table = toml_edit::Table::new();
			table.set_implicit(true);

			item[key] = toml_edit::Item::Table(table);
			write_value(&mut item[key], sub_path, value);
		}
	}
}

/// Returns the public IP of this machine.
async fn fetch_public_ip() -> Result<String> {
	let response = reqwest::get("https://ipinfo.io/ip").await?.text().await?;
	Ok(response.trim().to_string())
}

/// Generates an OpenSSH key and returns the private key.
async fn generate_private_key_openssh() -> Result<String> {
	block_in_place(|| {
		let tmp_dir = tempfile::TempDir::new()?;
		let key_path = tmp_dir.path().join("key");

		cmd!(
			"ssh-keygen",
			"-f",
			&key_path,
			"-t",
			"ecdsa",
			"-b",
			"521",
			"-N",
			""
		)
		.stdout_null()
		.run()?;

		let key = std::fs::read_to_string(&key_path)?;

		Ok(key)
	})
}

struct JwtKey {
	private_pem: String,
	public_pem: String,
}

/// Generates a JWT key pair.
async fn generate_jwt_key() -> Result<JwtKey> {
	block_in_place(|| {
		let tmp_dir = tempfile::TempDir::new()?;

		let private_pem_path = tmp_dir.path().join("private.pem");
		let public_pem_path = tmp_dir.path().join("public.pem");

		cmd!(
			"openssl",
			"genpkey",
			"-algorithm",
			"ed25519",
			"-outform",
			"PEM",
			"-out",
			&private_pem_path
		)
		.stdout_null()
		.run()?;

		cmd!(
			"openssl",
			"pkey",
			"-in",
			&private_pem_path,
			"-pubout",
			"-out",
			&public_pem_path
		)
		.stdout_null()
		.run()?;

		let private_pem = std::fs::read_to_string(&private_pem_path)?;
		let public_pem = std::fs::read_to_string(&public_pem_path)?;

		Ok(JwtKey {
			private_pem,
			public_pem,
		})
	})
}

/// Generates a random string for a secret.
fn generate_password(length: usize) -> String {
	rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length)
		.map(char::from)
		.collect()
}

/// Random password plus a special character in there somewhere.
fn generate_clickhouse_password(length: usize) -> String {
	let mut rng = rand::thread_rng();
	let split = rng.gen_range(0..length);

	rng.clone()
		.sample_iter(&Alphanumeric)
		.take(split)
		.chain(std::iter::once(46))
		.chain(rng.sample_iter(&Alphanumeric).take(length - split))
		.map(char::from)
		.collect()
}
