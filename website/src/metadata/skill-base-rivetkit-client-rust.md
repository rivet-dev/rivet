# RivetKit Rust Client

Use this skill when building Rust clients that connect to Rivet Actors with `rivetkit::client`.

## Version

RivetKit version: {{RIVETKIT_VERSION}}

## First Steps

1. Add the dependency
   ```sh
   cargo add rivetkit anyhow async-trait
   cargo add serde --features derive
   cargo add tokio --features full
   ```
2. Create a client with `Client::new(ClientConfig::new(endpoint))` and call typed actions with `get_or_create_typed_default::<A>(...)`.

## Error Handling Policy

- Prefer fail-fast behavior by default. Propagate `anyhow::Result` with `?`.
- Avoid swallowing errors with broad `match` arms unless absolutely needed.
- If an error is handled inline, handle it explicitly, at minimum by logging it.

<!-- CONTENT -->

## Need More Than the Client?

If you need more about Rivet Actors, registries, or server-side RivetKit, add the main skill:

```bash
npx skills add rivet-dev/skills
```

Then use the `rivetkit` skill for backend guidance.
