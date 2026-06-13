/// Path of the GitHub Actions workflow installed by `rivet setup-ci`.
pub const RIVET_DEPLOY_WORKFLOW_PATH: &str = ".github/workflows/rivet-deploy.yml";

/// GitHub Actions workflow that deploys to Rivet Cloud on push and pull
/// request. Kept in sync with the dashboard cloud onboarding flow
/// (`frontend/src/app/getting-started.tsx`).
pub fn rivet_deploy_workflow() -> &'static str {
	r#"name: Rivet Deploy

on:
  pull_request:
    types: [opened, synchronize, reopened, closed]
  push:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: rivet-deploy-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

jobs:
  rivet-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
      - uses: rivet-dev/deploy-action@v1.1.2
        with:
          rivet-token: ${{ secrets.RIVET_CLOUD_TOKEN }}
"#
}
