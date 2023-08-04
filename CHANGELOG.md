# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-   **Bolt** Support for connecting to Redis databases with `bolt redis sh`
-   **Infra** Support configuring multiple S3 providers
-   **Infra** Replace Promtail-based log shipping with native Loki Docker driver
-   **Infra** Add local Traefik Cloudflare proxy daemon for connecting to Cloudflare Access services

### Changed

-   **Infra** Update Nomad to 1.6.1

### Security

-   Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)

### Fixed

-   **Infra** Resolve [RUSTSEC-2023-0044](https://rustsec.org/advisories/RUSTSEC-2023-0044)
-   Skip captcha if no Turnstile key provided
