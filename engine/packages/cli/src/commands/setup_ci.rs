use std::{fs, path::PathBuf, process::Command as StdCommand};

use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use crate::{
	DEFAULT_CLOUD_API,
	auth::{Auth, OIDC_TRUSTS_WRITE_SCOPE, require_login_scopes, resolve_auth},
	cloud::CloudClient,
	target::{Link, choose, find_link, pick_org},
	templates::{RIVET_DEPLOY_WORKFLOW_FILE, rivet_deploy_workflow},
	terminal::{confirm, is_interactive},
	util::encode,
};

const DEPLOY_SCOPES: [&str; 2] = ["rivet:cluster:read", "rivet:cluster:deploy"];

const AFTER_HELP: &str = "\
The workflow deploys --branch to the cluster on every push. It signs in with the
job's GitHub Actions OIDC token, so the repository stores no Rivet secret, but
the cluster must trust the repository, branch and workflow file.

When signed in with `rivet login`, setup-ci checks that the cluster exists and
adds that trust itself. Managing trusts needs a scope `rivet login` does not
request by default, so setup-ci first asks you to sign in again with it:
`rivet login --scope rivet:v2:cluster:oidc-trusts:write`. Otherwise it still
writes the workflow and prints the trust to add in the cluster's settings.

Without --repo, the trusted repository is the `origin` remote. A clone may still
point at someone else's repository, so when signed in setup-ci asks to confirm
it at a terminal and requires --repo without one.

It prompts for a missing --org or --cluster only when signed in at a terminal.
Agents and scripts should pass --org, --cluster and --repo; without a terminal
it never prompts and fails with the flags it needs.

Examples:
  rivet login --scope rivet:v2:cluster:oidc-trusts:write
  rivet setup-ci --org acme --cluster production
  rivet setup-ci --org acme --cluster production --branch release --repo acme/app

The workflow is .github/workflows/rivet-deploy.yml. Commit and push it to deploy.";

#[derive(Parser)]
#[command(after_help = AFTER_HELP)]
pub struct Opts {
	/// Cluster to deploy to. Defaults to the linked cluster, then prompts when
	/// signed in at a terminal.
	#[arg(long, env = "RIVET_CLUSTER")]
	cluster: Option<String>,
	/// Organization slug the cluster belongs to. Defaults to the linked
	/// cluster's organization, then prompts when signed in at a terminal.
	#[arg(long, env = "RIVET_ORG")]
	org: Option<String>,
	/// Branch that deploys to the cluster.
	#[arg(long, default_value = "main")]
	branch: String,
	/// GitHub repository as `owner/repo`. Defaults to the `origin` remote,
	/// confirmed at a terminal before the cluster trusts it.
	#[arg(long)]
	repo: Option<String>,
	/// Overwrite the workflow file if it already exists.
	#[arg(long)]
	force: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

#[derive(Deserialize)]
struct TrustList {
	items: Vec<Trust>,
}

#[derive(Deserialize)]
struct Trust {
	repository: String,
	#[serde(rename = "ref")]
	git_ref: Option<String>,
	environment: Option<String>,
	workflow: String,
	scopes: Vec<String>,
	expires_at: Option<String>,
}

impl Trust {
	fn is_expired(&self) -> bool {
		self.expires_at.as_deref().is_some_and(|expires_at| {
			chrono::DateTime::parse_from_rfc3339(expires_at)
				.map_or(true, |expires_at| expires_at <= chrono::Utc::now())
		})
	}
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let login = self.login().await;
		if login.is_some() {
			require_login_scopes(
				&self.cloud_api,
				"rivet setup-ci",
				&[OIDC_TRUSTS_WRITE_SCOPE],
			)
			.await?;
		}
		let (org, cluster) = self.resolve_cluster(login.as_ref()).await?;
		let cloud = match &login {
			Some(auth) => Some(self.client(auth, Some(&org))?),
			None => None,
		};
		if let Some(cloud) = &cloud {
			check_cluster(cloud, &org, &cluster).await?;
		}

		let git_ref = format!("refs/heads/{}", self.branch);
		let repository = self.repository(cloud.is_some(), &cluster, &git_ref)?;

		let path = PathBuf::from(".github/workflows").join(RIVET_DEPLOY_WORKFLOW_FILE);
		if path.exists() && !self.force {
			bail!(
				"{} already exists; pass --force to overwrite",
				path.display()
			);
		}
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)?;
		}
		fs::write(&path, rivet_deploy_workflow(&org, &cluster, &self.branch))?;
		eprintln!(
			"wrote {}, which deploys branch {} to cluster {cluster}",
			path.display(),
			self.branch
		);

		let (Some(cloud), Some(repository)) = (&cloud, &repository) else {
			let repository = repository.as_deref().unwrap_or("<owner/repo>");
			print_manual_trust(&cluster, repository, &git_ref);
			return Ok(());
		};
		if let Err(err) = ensure_trust(cloud, &cluster, repository, &git_ref).await {
			eprintln!("could not create the GitHub Actions trust: {err:#}");
			print_manual_trust(&cluster, repository, &git_ref);
		}
		Ok(())
	}

	/// The repository the cluster trusts to deploy. One inferred from `origin`
	/// is confirmed before the CLI adds a trust for it.
	fn repository(&self, adds_trust: bool, cluster: &str, git_ref: &str) -> Result<Option<String>> {
		if let Some(repository) = &self.repo {
			return Ok(Some(repository.clone()));
		}
		let remotes = github_remotes();
		let Some(origin) = remotes
			.iter()
			.find(|(name, _)| name == "origin")
			.map(|(_, repository)| repository.clone())
		else {
			return Ok(None);
		};
		if !adds_trust {
			return Ok(Some(origin));
		}

		if !is_interactive() {
			bail!(
				"pass --repo <owner/repo> with the repository cluster {cluster} should trust to deploy; origin points at {origin}, which may not be yours"
			);
		}
		for (name, repository) in &remotes {
			if !repository.eq_ignore_ascii_case(&origin) {
				eprintln!("remote {name} points at {repository}");
			}
		}
		let question = format!("let pushes to {git_ref} in {origin} deploy to cluster {cluster}?");
		if !confirm(&question, "trust origin")? {
			bail!("pass --repo <owner/repo> with the repository cluster {cluster} should trust");
		}
		Ok(Some(origin))
	}

	/// The `rivet login` session, if any. Other credentials cannot manage
	/// trusts, so they count as signed out here.
	async fn login(&self) -> Option<Auth> {
		match resolve_auth(None, &self.cloud_api).await {
			Ok(auth) if auth.is_login() => Some(auth),
			Ok(_) | Err(_) => None,
		}
	}

	fn client(&self, auth: &Auth, org: Option<&str>) -> Result<CloudClient> {
		Ok(CloudClient::new(&self.cloud_api, auth.token.clone())?
			.with_login_refresh(auth, &self.cloud_api)
			.with_organization(org.map(str::to_string)))
	}

	/// Takes the cluster and organization from flags or the linked cluster,
	/// and prompts for what is missing when signed in at a terminal.
	async fn resolve_cluster(&self, login: Option<&Auth>) -> Result<(String, String)> {
		let (cluster, linked_org) = match (&self.cluster, find_link()?) {
			(Some(cluster), _) => (Some(cluster.clone()), None),
			(None, Some((Link::Cluster { cluster, org }, _))) => (Some(cluster), org),
			(None, Some((Link::Project { .. }, _)) | None) => (None, None),
		};
		let org = self.org.clone().or(linked_org);
		if let (Some(org), Some(cluster)) = (&org, &cluster) {
			return Ok((org.clone(), cluster.clone()));
		}

		let Some(auth) = login.filter(|_| is_interactive()) else {
			match cluster {
				Some(cluster) => bail!("pass --org with the organization of cluster {cluster}"),
				None => bail!(
					"no cluster selected; pass --cluster and --org, or run `rivet link --cluster <name> --org <slug>`"
				),
			}
		};

		let org = match org {
			Some(org) => org,
			None => pick_org(&self.client(auth, None)?).await?,
		};
		let cluster = match cluster {
			Some(cluster) => cluster,
			None => pick_cluster(&self.client(auth, Some(&org))?, &org).await?,
		};
		Ok((org, cluster))
	}
}

#[derive(Deserialize)]
struct ClusterList {
	items: Vec<ClusterSummary>,
}

#[derive(Deserialize)]
struct ClusterSummary {
	cluster: String,
}

async fn pick_cluster(cloud: &CloudClient, org: &str) -> Result<String> {
	let list: ClusterList = cloud
		.request_ok(Method::GET, "/v2/clusters", None)
		.await?
		.context("cluster list returned no body")?;
	let mut clusters: Vec<String> = list.items.into_iter().map(|item| item.cluster).collect();
	if clusters.is_empty() {
		bail!("organization {org} has no clusters; create one in the dashboard first");
	}
	let index = choose("Select a cluster", &clusters)?;
	Ok(clusters.swap_remove(index))
}

async fn check_cluster(cloud: &CloudClient, org: &str, cluster: &str) -> Result<()> {
	let path = format!("/v2/clusters/{}", encode(cluster));
	let found: Option<serde_json::Value> = cloud.request(Method::GET, &path, None).await?;
	if found.is_none() {
		bail!("cluster {cluster} not found in organization {org}");
	}
	Ok(())
}

/// Creates a trust letting the workflow deploy, unless one the CLI can see
/// already allows it.
async fn ensure_trust(
	cloud: &CloudClient,
	cluster: &str,
	repository: &str,
	git_ref: &str,
) -> Result<()> {
	let path = format!("/v2/clusters/{}/oidc-trusts", encode(cluster));

	let trusts: TrustList = cloud
		.request_ok(Method::GET, &path, None)
		.await?
		.context("trust list returned no body")?;
	let existing = trusts.items.iter().any(|trust| {
		!trust.is_expired()
			&& trust.repository.eq_ignore_ascii_case(repository)
			&& trust.git_ref.as_deref() == Some(git_ref)
			&& trust.environment.is_none()
			&& trust.workflow == RIVET_DEPLOY_WORKFLOW_FILE
			&& DEPLOY_SCOPES
				.iter()
				.all(|scope| trust.scopes.iter().any(|granted| granted == scope))
	});
	if existing {
		eprintln!(
			"cluster {cluster} already trusts {RIVET_DEPLOY_WORKFLOW_FILE} of {repository} on {git_ref}"
		);
		return Ok(());
	}

	let body = json!({
		"repository": repository,
		"ref": git_ref,
		"workflow": RIVET_DEPLOY_WORKFLOW_FILE,
		"scopes": DEPLOY_SCOPES,
	});
	let _: Option<serde_json::Value> = cloud.request_ok(Method::POST, &path, Some(body)).await?;
	eprintln!(
		"cluster {cluster} now trusts {RIVET_DEPLOY_WORKFLOW_FILE} of {repository} on {git_ref} to deploy"
	);
	Ok(())
}

fn print_manual_trust(cluster: &str, repository: &str, git_ref: &str) {
	eprintln!(
		"to let the workflow deploy, add a GitHub Actions trust in cluster {cluster}'s settings:"
	);
	eprintln!("  repository: {repository}");
	eprintln!("  ref:        {git_ref}");
	eprintln!("  workflow:   {RIVET_DEPLOY_WORKFLOW_FILE}");
	eprintln!("  scopes:     Deploy");
	eprintln!("or run `rivet login` and then `rivet setup-ci --force` again");
}

/// The GitHub repository of each git remote, by remote name.
fn github_remotes() -> Vec<(String, String)> {
	let output = StdCommand::new("git")
		.args(["config", "--get-regexp", r"^remote\..*\.url$"])
		.output();
	match output {
		Ok(output) if output.status.success() => {
			parse_github_remotes(&String::from_utf8_lossy(&output.stdout))
		}
		Ok(_) | Err(_) => Vec::new(),
	}
}

/// Parses `git config --get-regexp` lines of the form `remote.<name>.url <url>`,
/// keeping the remotes that point at GitHub.
fn parse_github_remotes(config: &str) -> Vec<(String, String)> {
	config
		.lines()
		.filter_map(|line| {
			let (key, url) = line.split_once(' ')?;
			let name = key.strip_prefix("remote.")?.strip_suffix(".url")?;
			let repository = github_repository(url.trim())?;
			Some((name.to_string(), repository))
		})
		.collect()
}

/// Extracts `owner/repo` from a GitHub remote URL in HTTPS, SSH or scp form.
fn github_repository(remote: &str) -> Option<String> {
	let path = remote
		.strip_prefix("git@github.com:")
		.or_else(|| remote.strip_prefix("ssh://git@github.com/"))
		.or_else(|| remote.strip_prefix("https://github.com/"))?;
	let path = path.trim_end_matches('/');
	let path = path.strip_suffix(".git").unwrap_or(path);
	let (owner, repo) = path.split_once('/')?;
	if owner.is_empty() || repo.is_empty() || repo.contains('/') {
		return None;
	}
	Some(format!("{owner}/{repo}"))
}
