# Changelog

## Unreleased

- Restored `Registry.handler(request)` and `Registry.serve()` for the native serverless runner endpoint described in `.agent/specs/serverless-restoration.md`. The route surface is `/api/rivet`, `/api/rivet/health`, `/api/rivet/metadata`, and `/api/rivet/start`; user traffic still goes through the Rivet Engine gateway.
- `Registry.start()` now starts the native envoy path only. Built-in `staticDir` serving is not wired through the native engine subprocess yet and remains a follow-up.
