# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-   **Infra** Support configuring multiple S3 providers
-   **Infra** Support multipart uploads
-   **Infra** Replace Promtail-based log shipping with native Loki Docker driver
-   **Infra** Local Traefik Cloudflare proxy daemon for connecting to Cloudflare Access services
-   **Infra** Upload service builds to default S3 provider instead of hardcoded bucket
-   **Bolt** Support for connecting to Redis databases with `bolt redis sh`
-   **Bolt** Add confirmation before running any command in the production namespace

### Changed

-   **Infra** Update CNI plugins to 1.3.0
-   **Infra** Update ClickHouse to 23.7.2.25
-   **Infra** Update Cockroach to 23.1.7
-   **Infra** Update Consul Exporter to 1.9.0
-   **Infra** Update Consul to 1.16.0
-   **Infra** Update Imagor to 1.4.7
-   **Infra** Update NATS server to 2.9.20
-   **Infra** Update Node Exporter server to 1.6.1
-   **Infra** Update Nomad to 1.6.1
-   **Infra** Update Prometheus server to 2.46.0
-   **Infra** Update Redis Exporter to 1.52.0
-   **Infra** Update Redis to 7.0.12
-   **Infra** Update Treafik to 2.10.4
-   **Bolt** PostHog events are now captured in a background task

### Security

-   Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)

### Fixed

-   **Portal** Skip captcha if no Turnstile key provided
-   **Infra** Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)
-   **Bolt** Correctly resolve project root when building services natively
