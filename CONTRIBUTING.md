# Contributing

## For Open Source Contributors

1. **Fork and Clone**: Fork the repository and clone it locally.
2. **Branch**: Create a feature or bugfix branch.
3. **Pull Request**:
   - Provide a clear title and description. The title must follow
     [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). These are used for generating our
     changelogs with [Release Please](https://github.com/googleapis/release-please).
   - Link the related GitHub issue (if applicable).
   - Validate that required checks pass. We ensure that Rivet's required checks run within < 5 minutes.

## For Rivet Employees

1. **Branching and Commits**:
   - Use [Graphite](https://graphite.dev/) for creating and managing branches.
   - Follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification when
     creating branches.
2. **Pull Request**:
   - Push your branch to the repo and request a review through Graphite.
   - Link related Linear issues in your PR body using the "magic words" `Fixes XXX-123`.
     [More information.](https://linear.app/docs/github#link-using-pull-requests)
     - Manually mark your issue as _Ready to Merge_ when ready.
   - Validate that required checks pass. We ensure that required checks run within reasonable time.
3. **Merging**:
   - Once approved, it's up to you to merge your commit. If deploying the frontend, make sure to monitor the
     changes from Sentry before going offline.
   - Manually mark your issue as _Complete_ once finished.
