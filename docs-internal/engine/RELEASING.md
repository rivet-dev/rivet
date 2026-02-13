# Releasing

## Frontend

Frontend services (including `dashboard.rivet.dev` and `inspect.rivet.dev`, but excluding the website) are deployed from the `prod` branch.

To promote to production, run:

```sh
./scripts/git/promote-prod.sh
```

This will validate that you're on the `main` branch and up to date with the remote before pushing.

To skip validation and push the current ref directly:

```sh
./scripts/git/promote-prod.sh --force
```

<details>
<summary>Why a branch instead of tags?</summary>

Railway does not support deploying services based on tags. We use the `prod` branch like a tag by force pushing the entire history to it.

</details>

## Website

The website deploys from `main`, not `prod`.

## Engine

To release a new version, run:

```sh
just release --patch   # Bump patch version (e.g., 1.0.0 -> 1.0.1)
just release --minor   # Bump minor version (e.g., 1.0.0 -> 1.1.0)
just release --major   # Bump major version (e.g., 1.0.0 -> 2.0.0)
```

To release a specific version:

```sh
just release --version 1.2.3
```

To release a new version reusing artifacts from a previous release (skips rebuilding Docker images and binaries):

```sh
just release --patch --reuse-engine-version 1.0.0
```

Run `just release --help` for all available options.

