use serde_json::json;

/// File name of the GitHub Actions workflow installed by `rivet setup-ci`, which
/// the cluster's trust pins.
pub const RIVET_DEPLOY_WORKFLOW_FILE: &str = "rivet-deploy.yml";

/// First `rivet-dev/deploy-action` release whose CLI exchanges the job's
/// GitHub Actions OIDC token when `rivet-token` is omitted.
const DEPLOY_ACTION_VERSION: &str = "v2.0.0-rc.2";

/// GitHub Actions workflow that deploys `branch` to `cluster` on push. The job
/// authenticates with its GitHub Actions OIDC token, so the cluster needs a
/// trust for the repository, branch and this workflow file. It pins this CLI's own version, which
/// can exchange that token.
pub fn rivet_deploy_workflow(org: &str, cluster: &str, branch: &str) -> String {
	// JSON strings are valid YAML double-quoted scalars.
	let branch_yaml = json!(branch);
	let org_yaml = json!(org);
	let cli_version_yaml = json!(env!("CARGO_PKG_VERSION"));
	let branch_clusters = json!({ branch: cluster });

	format!(
		r#"name: Rivet Deploy

on:
  push:
    branches: [{branch_yaml}]
  workflow_dispatch:

concurrency:
  group: rivet-deploy-${{{{ github.ref_name }}}}
  cancel-in-progress: false

jobs:
  rivet-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
    steps:
      - uses: actions/checkout@v4
      - uses: rivet-dev/deploy-action@{DEPLOY_ACTION_VERSION}
        with:
          rivet-cli-version: {cli_version_yaml}
          rivet-org: {org_yaml}
          branch-clusters: |
            {branch_clusters}
"#
	)
}
