# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-  **Bolt** Timeout for tests
-  **Bolt** New summary view of test progress
-  **Bolt** `config show` command
-  **Dev** Bolt automatically builds with Nix shell

### Changed

-  **Infra** Moved default API routes from `{service}.api.rivet.gg/v1` to `api.rivet.gg/{service}`
-  **Infra** Removed version flat from API request paths
-  **Bolt** Tests are built in batch and binaries are ran in parallel in order to speed up test times
-  **Bolt** Run tests inside of Kubernetes pod inside cluster, removing the need for port forwarding for tests
-  **Bolt** Remove `disable_cargo_workspace` flag since it is seldom used
-  **Bolt** Remove  `skip_dependencies`, `force_build`, and `skip_generate` on `bolt up` and `bolt test` commands that are no longer relevant
-  **api-route** Split up routes in to `/traefik/config/core` and `/traefik/config/game-guard`
-  **Imagor** CORS now mirror the default CORS configured for S3
-  **Dev** `git lfs install` automatically runs in `shellHook`
-  **Dev** Removed `setup.sh` in lieu of `shellHook`
-  Replaced `cdn.rivet.gg` domains with presigned requests directly to the S3 provider
-  **api-matchmaker** Gracefully disable automatic region selection when coords not obtainable
-  **Infra** Disabling DNS uses `X-Forwarded-For` header for the client IP
-  **Infra** Pool connections are now created in parallel for faster tests & service start times

### Fixed

-  `LC_ALL: cannot change locale` error from glibc
-  **Dev** Bolt uses `write_if_different` for auto-generated files to prevent cache purging

## [23.1.0-rc4] - 2023-09-02

### Changed

-   Revert Fern TypeScript generator to 0.5.6 to fix bundled export

## [23.1.0-rc3] - 2023-09-02

### Changed

-   Don't publish internal Fern package on tag to prevent duplicate pushes

## [23.1.0-rc2] - 2023-09-02

### Changed

-   Update to Fern 0.15.0-rc7
-   Update Fern TypeScript, Java, and Go generators

## [23.1.0-rc1] - 2023-09-02

### Added

-   **Matchmaker** Support custom lobbies
-   **Matchmaker** Support lobby state
-   **Matchmaker** Support external verification
-   **Library** Support Java library
-   **Library** Support Go library
-   **Cloud** Support multipart uploads for builds
-   **Infra** Support configuring multiple S3 providers
-   **Infra** Support multipart uploads
-   **Infra** Replace Promtail-based log shipping with native Loki Docker driver
-   **Infra** Local Traefik Cloudflare proxy daemon for connecting to Cloudflare Access services
-   **Infra** Upload service builds to default S3 provider instead of hardcoded bucket
-   **Infra** Enable Apache Traffic Server pull-through cache for Docker builds
-   **Bolt** Support for connecting to Redis databases with `bolt redis sh`
-   **Bolt** Confirmation before running any command in the production namespace
-   **Bolt** `--start-at` flag for all infra commands
-   **Bolt** Explicit database dependencies in services to reduce excess database pools

### Changed

-   **Infra** Update CNI plugins to 1.3.0
-   **Infra** Update ClickHouse to 23.7.2.25
-   **Infra** Update Cockroach to 23.1.7
-   **Infra** Update Consul Exporter to 1.9.0
-   **Infra** Update Consul to 1.16.0
-   **Infra** Update Imagor to 1.4.7
-   **Infra** Update NATS server to 2.9.20
-   **Infra** Update Node Exporter server to 1.6.0
-   **Infra** Update Nomad to 1.6.0
-   **Infra** Update Prometheus server to 2.46.0
-   **Infra** Update Redis Exporter to 1.52.0
-   **Infra** Update Redis to 7.0.12
-   **Infra** Update Treafik to 2.10.4
-   **Bolt** PostHog events are now captured in a background task
-   **Bolt** Auto-install rsync on Salt Master
-   **Bolt** Recursively add dependencies from overridden services when using additional roots
-   **KV** Significantly rate limit of all endpoints

### Security

-   Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)
-   Resolve [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093)
-   Resolve [RUSTSEC-2023-0053](https://rustsec.org/advisories/RUSTSEC-2023-0053)

### Fixed

-   **Portal** Skip captcha if no Turnstile key provided
-   **Infra** Missing dpenedency on mounting volumn before setting permissions of /var/\* for Cockroach, ClickHouse, Prometheus, and Traffic Server
-   **Chrip** Empty message parameters now have placeholder so NATS doesn't throw an error
-   **Chrip** Messages with no parameters no longer have a trailing dot
-   **Bolt** Correctly resolve project root when building services natively
-   **Bolt** Correctly determine executable path for `ExecServiceDriver::UploadedBinaryArtifact` with different Cargo names
