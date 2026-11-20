use std::{
	fs,
	io::Write,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::Args;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	DEFAULT_CLOUD_API,
	auth::{
		Auth, AuthSource, github_actions_token, has_explicit_token, in_github_actions, resolve_auth,
	},
	cloud::{CloudClient, TokenInspectResponse},
	terminal::is_interactive,
	util::encode,
};

const LINK_DIR: &str = ".rivet";
const LINK_FILE: &str = "project.json";

#[derive(Args)]
pub struct CloudArgs {
	/// Rivet Cloud API token. Overrides `rivet login`; intended for CI.
	#[arg(long)]
	pub token: Option<String>,
	/// Project name. Defaults to the token's project, then the linked project.
	#[arg(long, env = "RIVET_PROJECT")]
	pub project: Option<String>,
	/// Organization slug. Defaults to the token's organization, then the linked
	/// project's organization. With a cluster, names the organization the
	/// cluster belongs to.
	#[arg(long, env = "RIVET_ORG")]
	pub org: Option<String>,
	/// Cluster name. Selects a cluster instead of a project.
	#[arg(long, env = "RIVET_CLUSTER", conflicts_with = "project")]
	pub cluster: Option<String>,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	pub cloud_api: String,
}

pub enum Target {
	Project(ProjectTarget),
	Cluster(ClusterTarget),
}

pub struct ProjectTarget {
	pub org: String,
	pub project: String,
}

pub struct ClusterTarget {
	pub cluster: String,
	/// The organization the cluster belongs to, when one was named.
	pub org: Option<String>,
}

struct GitHubActionsCluster {
	org: String,
	cluster: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Link {
	Cluster {
		cluster: String,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		org: Option<String>,
	},
	Project {
		org: String,
		project: String,
	},
}

impl CloudArgs {
	pub async fn connect(&self) -> Result<(CloudClient, Auth, Target)> {
		if let Some(connected) = self.connect_github_actions().await? {
			return Ok(connected);
		}

		let auth = resolve_auth(self.token.as_deref(), &self.cloud_api).await?;
		let cloud = CloudClient::new(&self.cloud_api, auth.token.clone())?
			.with_login_refresh(&auth, &self.cloud_api);
		let target = self.resolve_target(&cloud, &auth).await?;
		let cloud = match &target {
			Target::Cluster(cluster) => cloud.with_organization(cluster.org.clone()),
			Target::Project(_) => cloud,
		};
		Ok((cloud, auth, target))
	}

	/// Connects for a command that only works on a cluster, taken from
	/// `--cluster` or a linked cluster. Unlike `connect`, it never inspects a
	/// token, prompts for a project or offers to link the directory.
	pub async fn connect_cluster(&self) -> Result<(CloudClient, ClusterTarget)> {
		let target = self.cluster_target()?;
		let auth = resolve_auth(self.token.as_deref(), &self.cloud_api).await?;
		let cloud = CloudClient::new(&self.cloud_api, auth.token.clone())?
			.with_login_refresh(&auth, &self.cloud_api)
			.with_organization(target.org.clone());
		Ok((cloud, target))
	}

	/// The cluster from `--cluster`, else from a linked cluster unless
	/// `--project` names a project.
	fn cluster_target(&self) -> Result<ClusterTarget> {
		if let Some(cluster) = &self.cluster {
			return Ok(ClusterTarget {
				cluster: cluster.clone(),
				org: self.org.clone(),
			});
		}
		if self.project.is_none()
			&& let Some((Link::Cluster { cluster, org }, _)) = find_link()?
		{
			return Ok(ClusterTarget {
				cluster,
				org: self.org.clone().or(org),
			});
		}
		bail!("no cluster selected; pass --cluster or run `rivet link --cluster <name>`")
	}

	pub async fn connect_project(&self) -> Result<(CloudClient, Auth, ProjectTarget)> {
		let (cloud, auth, target) = self.connect().await?;
		match target {
			Target::Project(project) => Ok((cloud, auth, project)),
			Target::Cluster(ClusterTarget { cluster, .. }) => bail!(
				"this command does not support clusters, but cluster {cluster} is selected; pass --org and --project to target a project"
			),
		}
	}

	/// Connects with a GitHub Actions OIDC token exchanged for a cluster
	/// credential, when the job may request one, no token was given and a
	/// cluster is selected.
	pub async fn connect_github_actions(&self) -> Result<Option<(CloudClient, Auth, Target)>> {
		let Some(target) = self.github_actions_cluster()? else {
			return Ok(None);
		};
		let token = github_actions_token(&self.cloud_api, &target.org, &target.cluster).await?;
		let auth = Auth {
			token: token.clone(),
			source: AuthSource::GitHubActions,
			user: None,
		};
		let cloud = CloudClient::new(&self.cloud_api, token)?
			.with_github_actions_refresh(&self.cloud_api, &target.org, &target.cluster)
			.with_organization(Some(target.org.clone()));
		let target = Target::Cluster(ClusterTarget {
			cluster: target.cluster,
			org: Some(target.org),
		});
		Ok(Some((cloud, auth, target)))
	}

	fn github_actions_cluster(&self) -> Result<Option<GitHubActionsCluster>> {
		if has_explicit_token(self.token.as_deref()) || !in_github_actions() {
			return Ok(None);
		}
		let (cluster, linked_org) = match (&self.cluster, find_link()?) {
			(Some(cluster), _) => (cluster.clone(), None),
			(None, Some((Link::Cluster { cluster, org }, _))) if self.project.is_none() => {
				(cluster, org)
			}
			_ => return Ok(None),
		};
		let org = self.org.clone().or(linked_org).with_context(|| {
			format!("pass --org or set RIVET_ORG to reach cluster {cluster} from GitHub Actions")
		})?;
		Ok(Some(GitHubActionsCluster { org, cluster }))
	}

	async fn resolve_target(&self, cloud: &CloudClient, auth: &Auth) -> Result<Target> {
		if let Some(cluster) = &self.cluster {
			return Ok(Target::Cluster(ClusterTarget {
				cluster: cluster.clone(),
				org: self.org.clone(),
			}));
		}
		if let (Some(org), Some(project)) = (&self.org, &self.project) {
			return Ok(Target::Project(ProjectTarget {
				org: org.clone(),
				project: project.clone(),
			}));
		}

		let link = find_link()?.map(|(link, _)| link);

		// A cluster has no project for a token to name, so a linked cluster
		// applies under any credentials.
		if self.project.is_none()
			&& let Some(Link::Cluster { cluster, org }) = &link
		{
			return Ok(Target::Cluster(ClusterTarget {
				cluster: cluster.clone(),
				org: self.org.clone().or_else(|| org.clone()),
			}));
		}

		// A Cloud API token is bound to one project, so it decides the target
		// before any linked directory.
		if !auth.is_login() {
			tracing::info!("inspecting Rivet Cloud token");
			let inspect: TokenInspectResponse = cloud
				.request(Method::GET, "/tokens/api/inspect", None)
				.await?
				.context("token inspect returned no body")?;
			return Ok(Target::Project(ProjectTarget {
				org: self.org.clone().unwrap_or(inspect.organization),
				project: self.project.clone().unwrap_or(inspect.project),
			}));
		}

		match link {
			Some(Link::Project { org, project }) => {
				return Ok(Target::Project(ProjectTarget {
					org: self.org.clone().unwrap_or(org),
					project: self.project.clone().unwrap_or(project),
				}));
			}
			Some(Link::Cluster { .. }) | None => {}
		}

		if !is_interactive() {
			bail!(
				"no project selected; run `rivet link` in this directory or pass --org and --project"
			);
		}
		let target = pick_target(cloud, self.org.as_deref(), self.project.as_deref()).await?;
		if confirm("link this directory to the selected project?")? {
			let path = write_link(&Link::Project {
				org: target.org.clone(),
				project: target.project.clone(),
			})?;
			eprintln!("linked {}", path.display());
		}
		Ok(Target::Project(target))
	}
}

#[derive(Deserialize)]
struct OrganizationsResponse {
	organizations: Vec<Organization>,
}

#[derive(Deserialize)]
struct Organization {
	name: String,
	slug: String,
}

#[derive(Deserialize)]
struct ProjectsResponse {
	projects: Vec<Project>,
	pagination: Option<Pagination>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
	name: String,
	display_name: Option<String>,
}

#[derive(Deserialize)]
struct Pagination {
	cursor: Option<String>,
}

/// Prompts for one of the signed-in user's organizations and returns its slug.
pub async fn pick_org(cloud: &CloudClient) -> Result<String> {
	let response: OrganizationsResponse = cloud
		.request_ok(Method::GET, "/oauth/organizations", None)
		.await?
		.context("organization list returned no body")?;
	let mut organizations = response.organizations;
	if organizations.is_empty() {
		bail!("your account has no organizations; create one in the dashboard first");
	}
	let labels: Vec<String> = organizations
		.iter()
		.map(|org| format!("{} ({})", org.name, org.slug))
		.collect();
	let index = choose("Select an organization", &labels)?;
	Ok(organizations.swap_remove(index).slug)
}

/// Prompts for the organization and project the caller did not already pass.
pub async fn pick_target(
	cloud: &CloudClient,
	org: Option<&str>,
	project: Option<&str>,
) -> Result<ProjectTarget> {
	let org = match org {
		Some(org) => org.to_string(),
		None => pick_org(cloud).await?,
	};

	let project = match project {
		Some(project) => project.to_string(),
		None => {
			let mut projects = list_projects(cloud, &org).await?;
			if projects.is_empty() {
				bail!("organization {org} has no projects; create one in the dashboard first");
			}
			let labels: Vec<String> = projects
				.iter()
				.map(|project| match &project.display_name {
					Some(display) if display != &project.name => {
						format!("{display} ({})", project.name)
					}
					_ => project.name.clone(),
				})
				.collect();
			let index = choose("Select a project", &labels)?;
			projects.swap_remove(index).name
		}
	};

	Ok(ProjectTarget { org, project })
}

async fn list_projects(cloud: &CloudClient, org: &str) -> Result<Vec<Project>> {
	let mut projects = Vec::new();
	let mut cursor: Option<String> = None;
	loop {
		let mut path = format!("/projects?org={}&limit=100", encode(org));
		if let Some(cursor) = &cursor {
			path.push_str(&format!("&cursor={}", encode(cursor)));
		}
		let response: ProjectsResponse = cloud
			.request_ok(Method::GET, &path, None)
			.await?
			.context("project list returned no body")?;
		projects.extend(response.projects);
		match response.pagination.and_then(|p| p.cursor) {
			Some(next) => cursor = Some(next),
			None => return Ok(projects),
		}
	}
}

pub fn choose(prompt: &str, options: &[String]) -> Result<usize> {
	if options.len() == 1 {
		eprintln!("{prompt}: {}", options[0]);
		return Ok(0);
	}
	eprintln!("{prompt}:");
	for (index, option) in options.iter().enumerate() {
		eprintln!("  {}) {option}", index + 1);
	}
	loop {
		eprint!("> ");
		std::io::stderr().flush()?;
		let mut input = String::new();
		if std::io::stdin().read_line(&mut input)? == 0 {
			bail!("no selection made");
		}
		match input.trim().parse::<usize>() {
			Ok(choice) if (1..=options.len()).contains(&choice) => return Ok(choice - 1),
			_ => eprintln!("enter a number between 1 and {}", options.len()),
		}
	}
}

fn confirm(prompt: &str) -> Result<bool> {
	eprint!("{prompt} [Y/n] ");
	std::io::stderr().flush()?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	let answer = input.trim().to_ascii_lowercase();
	Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

/// Finds the nearest `.rivet/project.json` walking up from the current
/// directory.
pub fn find_link() -> Result<Option<(Link, PathBuf)>> {
	let cwd = std::env::current_dir()?;
	for dir in cwd.ancestors() {
		let path = link_path(dir);
		if path.is_file() {
			let link = serde_json::from_str(&fs::read_to_string(&path)?)
				.with_context(|| format!("parse {}", path.display()))?;
			return Ok(Some((link, path)));
		}
	}
	Ok(None)
}

pub fn write_link(link: &Link) -> Result<PathBuf> {
	let path = link_path(&std::env::current_dir()?);
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	fs::write(&path, format!("{}\n", serde_json::to_string_pretty(link)?))
		.with_context(|| format!("write {}", path.display()))?;
	Ok(path)
}

pub fn remove_link() -> Result<Option<PathBuf>> {
	let path = link_path(&std::env::current_dir()?);
	if !path.is_file() {
		return Ok(None);
	}
	fs::remove_file(&path)?;
	if let Some(parent) = path.parent() {
		let _ = fs::remove_dir(parent);
	}
	Ok(Some(path))
}

fn link_path(dir: &Path) -> PathBuf {
	dir.join(LINK_DIR).join(LINK_FILE)
}
